use crate::doveadm::{Fetch, FetchFieldRes, FetchParams, ImapField, SearchParam};
use anyhow::{anyhow, Context, Result};
use log::{debug, error, info};
use mysql_async::prelude::{BatchQuery, Query, Queryable, WithParams};
use mysql_async::{params, Conn, Pool};
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
        db_init_script
            .ignore(&mut db_conn)
            .await
            .with_context(|| "failed to initialize database")?;
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

pub async fn scan(mut db_conn: Conn, user: String, user_id: u64) -> Result<()> {
    debug!("init_user: fetching,  id: {} ", user_id);
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

    let mut guid = String::new();
    let mut uid = String::new();
    let mut mailbox = String::new();
    let mut flags = Vec::new();
    let mut hdr = Vec::new();
    let mut date_received = String::new();
    let mut date_saved = String::new();
    let mut date_sent = String::new();
    let mut size_physical = 0;

    while let Some(record) = fetch_cmd.parse_record().await? {
        // debug!("got record: {:?}", record);
        let mut received = 0;
        for item in record.into_iter() {
            debug!("init_user: got {:?}", item);
            match item {
                FetchFieldRes::Uid(value) => {
                    uid = value;
                    received |= RECV_UID;
                }
                FetchFieldRes::Guid(value) => {
                    guid = value;
                    received |= RECV_GUID;
                }
                FetchFieldRes::Mailbox(value) => {
                    mailbox = value;
                    received |= RECV_MAILBOX;
                }
                FetchFieldRes::Flags(value) => {
                    flags = value;
                    received |= RECV_FLAGS;
                }
                FetchFieldRes::DateReceived(value) => {
                    date_received = value;
                    received |= RECV_DATE_RECV
                }
                FetchFieldRes::DateSaved(value) => {
                    date_saved = value;
                    received |= RECV_DATE_SAVD;
                }
                FetchFieldRes::DateSent(value) => {
                    date_sent = value;
                    received |= RECV_DATE_SENT;
                }
                FetchFieldRes::SizePhysical(value) => {
                    size_physical = value;
                    received |= RECV_SIZE
                }
                FetchFieldRes::Hdr(val) => {
                    hdr = val;
                    received |= RECV_HDRS
                }
                FetchFieldRes::Generic((_imap_field, _value)) => {
                    todo!()
                }
            }
        }

        if received == RECV_ALL {
            r"insert into record (user_id,uid,guid,mailbox) values(:user_id,:uid,:guid,:mailbox)"
                .with(params! {"user_id"=>user_id,"uid"=>uid.as_str(),"guid"=>guid.as_str(),"mailbox"=>mailbox.as_str()})
                .ignore(&mut db_conn)
                .await?;
            if let Some(record_id) = db_conn.last_insert_id() {
                if !flags.is_empty() {
                    r"insert into imap_flag (record_id, name) values(:record_id,:name)"
                        .with(
                            flags
                                .iter()
                                .map(|flag| params! {"record_id" => record_id, "name" => flag}),
                        )
                        .batch(&mut db_conn)
                        .await?;
                }
                if !hdr.is_empty() {
                    match r"insert into header (record_id, seq, name, value) values(:record_id,:seq,:name,:value)"
                        .with(hdr.iter().enumerate().map(|(idx, hdr)| {
                            params! {
                                        "record_id"=> record_id, 
                                        "seq"=> idx, 
                                        "name" => hdr.0.to_owned(), 
                                        "value" => hdr.1.to_owned()}
                        })).batch(&mut db_conn).await {
                        Ok(_) => { debug!("added headers"); }
                        Err(e) => {
                            error!("failed to add headers: {:?}", e);
                            continue;
                        }
                    }
                }
            } else {
                error!("init_user: failed to insert record");
                continue;
            }
        } else {
            error!("missing FetchFieldRes: received: {:x} ", received)
        }
    }
    debug!("done parsing records");
    Ok(())
}
