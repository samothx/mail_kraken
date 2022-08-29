use crate::doveadm::{Fetch, FetchFieldRes, FetchParams, FetchRecord, ImapField, SearchParam};
use anyhow::{anyhow, Context, Result};
use log::{debug, error, info};
use mysql_async::prelude::{BatchQuery, Query, Queryable, WithParams};
use mysql_async::{params, Conn, Pool};
use regex::Regex;
use tokio::task::JoinHandle;

const DB_VERSION: u32 = 1;

pub async fn init_db(pool: Pool) -> Result<()> {
    debug!("init: entered");
    let mut db_conn = pool
        .get_conn()
        .await
        .with_context(|| "failed to get a connection from database pool".to_owned())?;

    let tables: Vec<String> = "show tables".fetch(&mut db_conn).await?;
    if tables.is_empty() {
        // database is uninitialized
        info!("initializing database");
        let db_init_script = String::from_utf8_lossy(include_bytes!("../sql/init_db_v1.sql"));
        match db_init_script.ignore(&mut db_conn).await {
            Ok(_) => (),
            Err(e) => {
                let err_str = e.to_string();
                error!("failed to initialize database: {:?}", err_str.as_str());
                return Err(e).with_context(|| {
                    format!("failed to initialize database: {:?}", err_str.as_str())
                });
            }
        }

        r"insert into db_ver (version) values(:version)"
            .with(params! { "version" => DB_VERSION })
            .ignore(&mut db_conn)
            .await?;
        info!("database was initialized successfully");

        Ok(())
    } else if !tables.contains(&"db_ver".to_owned()) {
        Err(anyhow!(
            "database is not empty but does not contain the db_ver table"
        ))
    } else {
        let db_ver: Option<u32> = db_conn
            .query_first("select max(version) from db_ver")
            .await?;
        if let Some(version) = db_ver {
            if version == DB_VERSION {
                Ok(())
            } else {
                Err(anyhow!(
                    "invalid database version: expected {}, got {}",
                    DB_VERSION,
                    version
                ))
            }
        } else {
            Err(anyhow!(
                "invalid database version: expected {}, got none",
                DB_VERSION,
            ))
        }
    }
}

pub async fn init_user(pool: Pool, user: &str) -> Result<Option<JoinHandle<Result<()>>>> {
    debug!("init_user: called for {}", user);
    let mut db_conn = pool.get_conn().await?;
    let user_id: Option<u64> = r"select id from user where user=:user"
        .with(params! {"user"=>user})
        .first(&mut db_conn)
        .await?;

    let (user_id, count) = if let Some(user_id) = user_id {
        (
            user_id,
            r"select count(*) from record where user_id=:user_id"
                .with(params! {"user_id"=>user_id})
                .first(&mut db_conn)
                .await?
                .unwrap_or(0u64),
        )
    } else {
        r"insert into user (user) values(:user)"
            .with(params! {"user"=>user})
            .ignore(&mut db_conn)
            .await?;
        (
            db_conn
                .last_insert_id()
                .ok_or_else(|| anyhow!("failed to retrieve last user_id from db"))?,
            0,
        )
    };

    if count == 0 {
        // start a background job fill database for user
        let user = user.to_owned();
        Ok(Some(tokio::spawn(async move {
            scan(db_conn, user, user_id).await
        })))
    } else {
        Ok(None)
    }
}

const RECV_UID: u32 = 0x1;
const RECV_GUID: u32 = 0x2;
const RECV_DATE_RECV: u32 = 0x4;
const RECV_DATE_SAVD: u32 = 0x8;
const RECV_DATE_SENT: u32 = 0x10;
const RECV_SIZE: u32 = 0x20;
const RECV_MAILBOX: u32 = 0x40;
const RECV_FLAGS: u32 = 0x80;
const RECV_HDRS: u32 = 0x100;
const RECV_ALL: u32 = RECV_UID
    | RECV_GUID
    | RECV_DATE_SAVD
    | RECV_DATE_RECV
    | RECV_DATE_SENT
    | RECV_SIZE
    | RECV_MAILBOX
    | RECV_FLAGS
    | RECV_HDRS;

pub async fn scan(db_conn: Conn, user: String, user_id: u64) -> Result<()> {
    debug!("init_user: fetching,  id: {} ", user_id);

    let date_time_tz_regex = Regex::new(r"^(\d{4}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2})\s\(([^)]+)\)$")?;
    let fetch_params = FetchParams::new(user)
        .add_field(ImapField::Flags)
        .add_field(ImapField::Guid)
        .add_field(ImapField::Uid)
        .add_field(ImapField::Mailbox)
        .add_field(ImapField::Hdr)
        .add_field(ImapField::SizePhysical)
        .add_field(ImapField::DateSent)
        .add_field(ImapField::DateSaved)
        .add_field(ImapField::DateReceived)
        .add_search_param(SearchParam::All);
    let mut fetch_cmd = Fetch::new(fetch_params)?;

    let mut wa = Workaround { db_conn };

    let mut read_buf = ReadBuf::new();
    while let Some(record) = fetch_cmd.parse_record().await? {
        // debug!("got record: {:?}", record);
        match process_record(&mut wa, user_id, record, &mut read_buf, &date_time_tz_regex).await {
            Ok(_) => (),
            Err(e) => {
                error!("scan: process_record failed: {:?}", e);
                continue;
            }
        };
    }
    debug!("scan: done parsing records");
    Ok(())
}

async fn process_record(
    wa: &mut Workaround,
    user_id: u64,
    record: FetchRecord,
    read_buf: &mut ReadBuf,
    date_time_tz_regex: &Regex,
) -> Result<()> {
    let mut received = 0;
    for item in record.into_iter() {
        debug!("process_record: got {:?}", item);
        match item {
            FetchFieldRes::Uid(value) => {
                read_buf.uid = value;
                received |= RECV_UID;
            }
            FetchFieldRes::Guid(value) => {
                read_buf.guid = value;
                received |= RECV_GUID;
            }
            FetchFieldRes::Mailbox(value) => {
                read_buf.mailbox = value;
                received |= RECV_MAILBOX;
            }
            FetchFieldRes::Flags(value) => {
                read_buf.flags = value;
                received |= RECV_FLAGS;
            }
            FetchFieldRes::DateReceived(value) => {
                read_buf.date_received = value;
                received |= RECV_DATE_RECV
            }
            FetchFieldRes::DateSaved(value) => {
                read_buf.date_saved = value;
                received |= RECV_DATE_SAVD;
            }
            FetchFieldRes::DateSent(value) => {
                read_buf.date_sent = value;
                received |= RECV_DATE_SENT;
            }
            FetchFieldRes::SizePhysical(value) => {
                read_buf.size_physical = value;
                received |= RECV_SIZE
            }
            FetchFieldRes::Hdr(val) => {
                read_buf.hdr = val;
                received |= RECV_HDRS
            }
            FetchFieldRes::Generic((_imap_field, _value)) => {
                todo!()
            }
        }
    }

    if received == RECV_ALL {
        let (date_time_sent, offset) =
            if let Some(captures) = date_time_tz_regex.captures(read_buf.date_sent.as_str()) {
                (
                    captures
                        .get(1)
                        .ok_or_else(|| {
                            anyhow!("process_record: failed to get date_time capture from regex")
                        })?
                        .as_str()
                        .to_owned(),
                    captures
                        .get(2)
                        .ok_or_else(|| {
                            anyhow!("process_record: failed to offset_time capture from regex")
                        })?
                        .as_str()
                        .to_owned(),
                )
            } else {
                return Err(anyhow!(
                    "process_record: failed to parse date_sent from: {}",
                    read_buf.date_sent.as_str()
                ));
            };

        debug!(
            "process_record: date time sent: [{}],[{}]",
            date_time_sent, offset
        );

        r"insert into record (user_id,uid,guid,mailbox,) values(:user_id,:uid,:guid,:mailbox)"
            .with(params! {
            "user_id"=>user_id,
            "uid"=>read_buf.uid.as_str(),
            "guid"=>read_buf.guid.as_str(),
            "mailbox"=>read_buf.mailbox.as_str(),
            "dt_sent"=>date_time_sent.as_str().parse::<u32>()?,
            "tz_sent"=>offset.as_str(),
            "dt_recv"=>read_buf.date_received.as_str(),
            "dt_saved"=>read_buf.date_saved.as_str(),
            "size"=>read_buf.size_physical})
            .ignore(&mut wa.db_conn)
            .await
            .with_context(|| "failed to insert record".to_owned())?;
        if let Some(record_id) = wa.db_conn.last_insert_id() {
            if !read_buf.flags.is_empty() {
                r"insert into imap_flag (record_id, name) values(:record_id,:name)"
                    .with(
                        read_buf
                            .flags
                            .iter()
                            .map(|flag| params! {"record_id" => record_id, "name" => flag}),
                    )
                    .batch(&mut wa.db_conn)
                    .await
                    .with_context(|| "failed to insert imap_flags".to_owned())?;
            }
            if !read_buf.hdr.is_empty() {
                r"insert into header (record_id, seq, name, value) values(:record_id,:seq,:name,:value)"
                    .with(read_buf.hdr.iter().enumerate().map(|(idx, hdr)| {
                        params! {
                                        "record_id"=> record_id,
                                        "seq"=> idx,
                                        "name" => hdr.0.to_owned(),
                                        "value" => hdr.1.to_owned()}
                    })).batch(&mut wa.db_conn).await.with_context(|| "failed to insert headers".to_owned())?;
            }
            Ok(())
        } else {
            Err(anyhow!("process_record: failed to insert record"))
        }
    } else {
        Err(anyhow!(
            "process_record: missing FetchFieldRes: received: {:x} ",
            received
        ))
    }
}

struct Workaround {
    db_conn: Conn,
}

struct ReadBuf {
    guid: String,
    uid: String,
    mailbox: String,
    flags: Vec<String>,
    hdr: Vec<(String, String)>,
    date_received: String,
    date_saved: String,
    date_sent: String,
    size_physical: usize,
}

impl ReadBuf {
    pub fn new() -> ReadBuf {
        ReadBuf {
            guid: String::new(),
            uid: String::new(),
            mailbox: String::new(),
            flags: Vec::new(),
            hdr: Vec::new(),
            date_received: String::new(),
            date_saved: String::new(),
            date_sent: String::new(),
            size_physical: 0,
        }
    }
}
