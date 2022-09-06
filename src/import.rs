use super::doveadm::{Fetch, FetchParams, ImapField, SearchParam};
use super::{Config, ImportArgs};
use anyhow::{anyhow, Context, Result};
use log::{debug, error, info};
use mod_logger::Logger;
use mysql::{prelude::Queryable, Pool};

use nix::unistd::getuid;
use regex::Regex;

mod db;
use db::init_user;
mod email_db;

mod email_parser;
use crate::import::db::{process_record, Buffers};
use crate::import::email_db::EmailDb;
use email_parser::EmailParser;

const DB_VERSION: u32 = 1;

// sync import used by sync process
pub fn import(args: ImportArgs) -> Result<()> {
    Logger::set_default_level(args.log_level);
    Logger::set_color(true);
    Logger::set_brief_info(true);

    info!("started for user : '{}'", args.user);

    if !getuid().is_root() {
        return Err(anyhow!("please run this command as root"));
    }

    let config = Config::from_file().with_context(|| "failed to read config file".to_owned())?;

    let pool = Pool::new(
        config
            .get_db_url()
            .ok_or_else(|| anyhow!("db_url is not set in config"))?
            .as_str(),
    )
    .with_context(|| "failed to connect to db".to_owned())?;

    let mut pooled_db_conn = pool.get_conn()?;
    let db_conn = pooled_db_conn.as_mut();
    if let Some(version) = db_conn.query_first::<u32, &str>(r#"select max(version) from db_ver"#)? {
        if version != DB_VERSION {
            return Err(anyhow!(format!(
                "invalid database version found: got {}, required {}",
                version, DB_VERSION
            )));
        }
    } else {
        return Err(anyhow!("unable to retrieve version from database"));
    }

    let (user_id, empty) = init_user(db_conn, args.user.as_str())?;

    let fetch_params = FetchParams::new(args.user.to_owned())
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

    let date_time_tz_regex = Regex::new(r"^(\d{4}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2})\s\(([^)]+)\)$")?;
    let mut email_db = EmailDb::new();
    let mut email_parser = EmailParser::new();
    let mut buffers = Buffers::new();

    let mut msg_count = 0usize;
    let mut insert_count = 0usize;
    let ts_start = chrono::Local::now();

    if empty {
        let mut last_flush = ts_start;
        while let Some(record) = match fetch_cmd.parse_record() {
            Ok(res) => res,
            Err(e) => {
                error!("scan: parser failed with {:?}", e);
                return Err(e);
            }
        } {
            msg_count += 1;
            match process_record(
                db_conn,
                user_id,
                record,
                &mut buffers,
                &date_time_tz_regex,
                &mut email_parser,
                &mut email_db,
            ) {
                Ok(_) => {
                    insert_count += 1;
                }
                Err(e) => {
                    error!("import: process_record failed: {:?}", e);
                    continue;
                }
            };
            if msg_count % 20 == 0 {
                let now = chrono::Local::now();
                let duration = now - ts_start;
                info!(
                    "import: processed {} messages, inserted {} in {} seconds, {} inserted/second",
                    msg_count,
                    insert_count,
                    duration.num_seconds(),
                    insert_count * 1000 / duration.num_milliseconds() as usize
                );

                if (now - last_flush).num_seconds() > 20 {
                    last_flush = now;
                    email_db.flush_to_db(db_conn, user_id)?;
                }
            }
        }

        email_db.flush_to_db(db_conn, user_id)?;
    } else {
        return Err(anyhow!(
            "user db contains values & update is not implemented"
        ));
    }

    Ok(())
}

/*

pub fn scan(db_conn: Conn, user: String, user_id: u64) -> Result<()> {
    debug!("scan: fetching,  id: {} ", user_id);

    let date_time_tz_regex = Regex::new(r"^(\d{4}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2})\s\(([^)]+)\)$")?;
    let mut email_parser = EmailParser::new();
    let mut email_db = EmailDb::new();

    let fetch_params = FetchParams::new(user.to_owned())
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

    let mut db = DbConCntr { db_conn };

    let mut read_buf = ReadBuf::new();
    let mut msg_count = 0usize;
    let mut insert_count = 0usize;
    let ts_start = chrono::Local::now();
    while let Some(record) = match fetch_cmd.parse_record().await {
        Ok(res) => res,
        Err(e) => {
            error!("scan: parser failed with {:?}", e);
            return Err(e);
        }
    } {
        trace!("got record: {:?}", record);
        msg_count += 1;
        match process_record(
            &mut db,
            user_id,
            record,
            &mut read_buf,
            &date_time_tz_regex,
            &mut email_parser,
            &mut email_db,
        )
        .await
        {
            Ok(_) => {
                insert_count += 1;
            }
            Err(e) => {
                error!("scan: process_record failed: {:?}", e);
                continue;
            }
        };
        if msg_count % 20 == 0 {
            let duration = chrono::Local::now() - ts_start;
            debug!(
                "scan: processed {} messages, inserted {} in {} seconds, {} inserted/second",
                msg_count,
                insert_count,
                duration.num_seconds(),
                insert_count * 1000 / duration.num_milliseconds() as usize
            );
        }
    }

    // email_db.flush_to_db(&mut db, user_id).await?;

    let duration = chrono::Local::now() - ts_start;
    let status = fetch_cmd
        .get_exit_status()
        .await
        .with_context(|| "failed to retrieve exit status from fetch process".to_owned())?;

    info!(
        "scan: processed {} messages, inserted {} in {} seconds, {} inserted/second",
        msg_count,
        insert_count,
        duration.num_seconds(),
        insert_count * 1000 / duration.num_milliseconds() as usize
    );
    info!(
        "scan: done parsing records: fetch exit status: {}",
        status.code().unwrap_or(-1)
    );

    Ok(())
}

async fn process_record(
    db_conn: &mut DbConCntr,
    user_id: u64,
    record: FetchRecord,
    read_buf: &mut ReadBuf,
    date_time_tz_regex: &Regex,
    email_parser: &mut EmailParser,
    email_db: &mut EmailDb,
) -> Result<()> {
    let mut received = 0;
    for item in record.into_iter() {
        // debug!("process_record: got {:?}", item);
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
        }
    }

    if (received & RECV_HDRS) == RECV_HDRS {
        for (name, value) in read_buf.hdr.iter() {
            match name.as_str() {
                HDR_NAME_RECV => {
                    received |= RECV_RECEIVED;
                }
                HDR_NAME_TO => {
                    read_buf.to = email_parser.parse(value.as_str());
                    received |= RECV_TO;
                }
                HDR_NAME_FROM => {
                    read_buf.from = email_parser.parse(value.as_str());
                    received |= RECV_FROM;
                }
                HDR_NAME_CC => {
                    read_buf.cc = email_parser.parse(value.as_str());
                    received |= RECV_CC;
                }
                HDR_NAME_BCC => {
                    read_buf.bcc = email_parser.parse(value.as_str());
                    received |= RECV_BCC;
                }
                HDR_NAME_SUBJ => {
                    read_buf.subj = value.to_owned();
                    received |= RECV_SUBJ;
                }
                _ => (),
            };
            if (received & RECV_HEADERS_ALL) == RECV_HEADERS_ALL {
                break;
            }
        }
        if (received & RECV_FROM) != RECV_FROM {
            let mut header_names = String::new();
            read_buf.hdr.iter().for_each(|(name, _)| {
                header_names.push_str(format!("\"{}\" ", name).as_str());
            });
            warn!(
                "process_record: required header From, missing from: {}",
                header_names
            );
        }
    }

    if (received & RECV_REQUIRED) == RECV_REQUIRED {
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

        trace!(
            "process_record: date time sent: [{}],[{}]",
            date_time_sent,
            offset
        );

        let outbound = (received & RECV_RECEIVED) != RECV_RECEIVED;
        let email_from = if let Some((email, name, valid)) = read_buf.from.get(0) {
            if *valid {
                Some(
                    email_db
                        .add_email(
                            db_conn,
                            user_id,
                            email.as_str(),
                            name.as_ref().map(|val| val.as_str()),
                            if outbound {
                                EmailType::OutboundFrom
                            } else {
                                EmailType::InboundFrom((
                                    read_buf
                                        .flags
                                        .iter()
                                        .any(|name| name.as_str() == r#"\Seen"#),
                                    false,
                                ))
                            },
                        )
                        .await?,
                )
            } else {
                None
            }
        } else {
            None
        };

        r#"insert into record (user_id,uid,guid,mailbox,dt_sent,tz_sent,dt_recv,dt_saved,size,mail_subj,outbound, mail_from)
 values(:user_id,:uid,:guid,:mailbox,:dt_sent,:tz_sent,:dt_recv,:dt_saved,:size,:subj,:outbound, :mail_from)"#
            .with(params! {
            "user_id"=>user_id,
            "uid"=>read_buf.uid.as_str(),
            "guid"=>read_buf.guid.as_str(),
            "mailbox"=>read_buf.mailbox.as_str(),
            "dt_sent"=>date_time_sent,
            "tz_sent"=>offset.as_str().parse::<f32>().with_context(|| format!("failed to parse [{}] to f32", offset.as_str()))?,
            "dt_recv"=>read_buf.date_received.as_str(),
            "dt_saved"=>read_buf.date_saved.as_str(),
            "size"=>read_buf.size_physical,
            "subj"=>read_buf.subj.as_str(),
            "outbound"=>(received & RECV_RECEIVED) != RECV_RECEIVED,
                "mail_from" => email_from})
            .ignore(&mut db_conn.db_conn)
            .await
            .with_context(|| "failed to insert record".to_owned())?;
        if let Some(record_id) = db_conn.db_conn.last_insert_id() {
            if !read_buf.flags.is_empty() {
                r"insert into imap_flag (record_id, name) values(:record_id,:name)"
                    .with(
                        read_buf
                            .flags
                            .iter()
                            .map(|flag| params! {"record_id" => record_id, "name" => flag}),
                    )
                    .batch(&mut db_conn.db_conn)
                    .await
                    .with_context(|| "failed to insert imap_flags".to_owned())?;
            }
            if !read_buf.hdr.is_empty() {
                r"insert into header (record_id, seq, name, value) values(:record_id,:seq,:name,:value)"
                    .with(read_buf.hdr.iter()
                        .filter(|(name,_value)|
                            match name.as_str() {
                                HDR_NAME_FROM | HDR_NAME_TO | HDR_NAME_SUBJ | HDR_NAME_CC | HDR_NAME_BCC => false,
                                _ => true
                            }
                        )
                        .enumerate()
                        .map(|(idx, hdr)| {
                            params! {   "record_id"=> record_id,
                                    "seq"=> idx,
                                    "name" => hdr.0.to_owned(),
                                    "value" => hdr.1.to_owned()}
                        })).batch(&mut db_conn.db_conn).await.with_context(|| "process_record: failed to insert headers".to_owned())?;
            }
            if !read_buf.to.is_empty() {
                for (email, name, valid) in read_buf.to.iter() {
                    if *valid {
                        let email_id = email_db
                            .add_email(
                                db_conn,
                                user_id,
                                email.as_str(),
                                name.as_ref().map(|val| val.as_str()),
                                if outbound {
                                    EmailType::OutboundReceipient
                                } else {
                                    EmailType::Other
                                },
                            )
                            .await?;

                        r"insert into mail_to (record_id, email_id) values(:record_id,:email_id)"
                            .with(params! {   "record_id"=> record_id, "email_id"=> email_id })
                            .ignore(&mut db_conn.db_conn)
                            .await
                            .with_context(|| {
                                "process_record: failed to insert mail_to".to_owned()
                            })?;
                    }
                }
            }
            if !read_buf.cc.is_empty() {
                for (email, name, valid) in read_buf.cc.iter() {
                    if *valid {
                        let email_id = email_db
                            .add_email(
                                db_conn,
                                user_id,
                                email.as_str(),
                                name.as_ref().map(|val| val.as_str()),
                                if outbound {
                                    EmailType::OutboundAuxReceipient
                                } else {
                                    EmailType::Other
                                },
                            )
                            .await?;

                        r"insert into mail_cc (record_id, email_id) values(:record_id,:email_id)"
                            .with(params! {   "record_id"=> record_id, "email_id"=> email_id })
                            .ignore(&mut db_conn.db_conn)
                            .await
                            .with_context(|| {
                                "process_record: failed to insert mail_cc".to_owned()
                            })?;
                    }
                }
            }
            if !read_buf.bcc.is_empty() {
                for (email, name, valid) in read_buf.bcc.iter() {
                    if *valid {
                        let email_id = email_db
                            .add_email(
                                db_conn,
                                user_id,
                                email.as_str(),
                                name.as_ref().map(|val| val.as_str()),
                                if outbound {
                                    EmailType::OutboundAuxReceipient
                                } else {
                                    EmailType::Other
                                },
                            )
                            .await?;

                        r"insert into mail_bcc (record_id, email_id) values(:record_id,:email_id)"
                            .with(params! {   "record_id"=> record_id, "email_id"=> email_id })
                            .ignore(&mut db_conn.db_conn)
                            .await
                            .with_context(|| {
                                "process_record: failed to insert mail_bcc".to_owned()
                            })?;
                    }
                }
            }

            Ok(())
        } else {
            Err(anyhow!("process_record: failed to insert record"))
        }
    } else {
        Err(anyhow!(
            "process_record: missing FetchFieldRes: received: {:x}, expected: {:x}",
            received,
            RECV_REQUIRED
        ))
    }
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
    to: Vec<(String, Option<String>, bool)>,
    from: Vec<(String, Option<String>, bool)>,
    cc: Vec<(String, Option<String>, bool)>,
    bcc: Vec<(String, Option<String>, bool)>,
    subj: String,
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
            to: Vec::new(),
            from: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subj: String::new(),
        }
    }
}
*/
