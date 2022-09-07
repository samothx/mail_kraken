use super::email_db::{EmailDb, EmailType};
use crate::doveadm::{FetchFieldRes, FetchRecord};
use crate::import::email_parser::EmailParser;
use anyhow::{anyhow, Context, Result};
use log::{debug, trace, warn};
use mysql::{params, prelude::Queryable, Conn};
use regex::Regex;

type UserId = u64;

pub fn init_user(db_conn: &mut Conn, user: &str) -> Result<(UserId, bool)> {
    debug!("init_user: called for {}", user);

    let user_id = db_conn.exec_first(
        r#"select id from user where user=:user"#,
        params! {"user"=>user},
    )?;

    Ok(if let Some(user_id) = user_id {
        let count: u64 = db_conn
            .exec_first(
                r#"select count(*) from record where user_id=:user_id"#,
                params! {"user_id"=>user_id},
            )?
            .expect("unexpected missing result from query");

        (user_id, count == 0)
    } else {
        db_conn.exec_drop(
            r#"insert into user (user) values(:user)"#,
            params! {"user"=>user},
        )?;
        let user_id = db_conn.last_insert_id();
        (user_id, true)
    })
}

pub fn process_record(
    db_conn: &mut Conn,
    user_id: u64,
    record: FetchRecord,
    buffers: &mut Buffers,
    date_time_tz_regex: &Regex,
    email_parser: &mut EmailParser,
    email_db: &mut EmailDb,
) -> Result<()> {
    let mut received = 0;

    // collect fetch fields, ensure all needed fields are there & store in Buffers
    for item in record.into_iter() {
        // debug!("process_record: got {:?}", item);
        match item {
            FetchFieldRes::Uid(value) => {
                buffers.uid = value;
                received |= RECV_UID;
            }
            FetchFieldRes::Guid(value) => {
                buffers.guid = value;
                received |= RECV_GUID;
            }
            FetchFieldRes::Mailbox(value) => {
                buffers.mailbox = value;
                received |= RECV_MAILBOX;
            }
            FetchFieldRes::Flags(value) => {
                buffers.flags = value;
                received |= RECV_FLAGS;
            }
            FetchFieldRes::DateReceived(value) => {
                buffers.date_received = value;
                received |= RECV_DATE_RECV
            }
            FetchFieldRes::DateSaved(value) => {
                buffers.date_saved = value;
                received |= RECV_DATE_SAVD;
            }
            FetchFieldRes::DateSent(value) => {
                buffers.date_sent = value;
                received |= RECV_DATE_SENT;
            }
            FetchFieldRes::SizePhysical(value) => {
                buffers.size_physical = value;
                received |= RECV_SIZE
            }
            FetchFieldRes::Hdr(val) => {
                buffers.hdr = val;
                received |= RECV_HDRS
            }
        }
    }

    if (received & RECV_HDRS) == RECV_HDRS {
        // extract fields fom headers
        for (name, value) in buffers.hdr.iter() {
            match name.to_lowercase().as_str() {
                HDR_NAME_RECV => {
                    received |= RECV_RECEIVED;
                }
                HDR_NAME_TO => {
                    buffers.to = email_parser.parse(value.as_str());
                    received |= RECV_TO;
                }
                HDR_NAME_FROM => {
                    buffers.from = email_parser.parse(value.as_str());
                    received |= RECV_FROM;
                }
                HDR_NAME_CC => {
                    buffers.cc = email_parser.parse(value.as_str());
                    received |= RECV_CC;
                }
                HDR_NAME_BCC => {
                    buffers.bcc = email_parser.parse(value.as_str());
                    received |= RECV_BCC;
                }
                HDR_NAME_SUBJ => {
                    buffers.subj = value.to_owned();
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
            buffers.hdr.iter().for_each(|(name, _)| {
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
            if let Some(captures) = date_time_tz_regex.captures(buffers.date_sent.as_str()) {
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
                    buffers.date_sent.as_str()
                ));
            };

        trace!(
            "process_record: date time sent: [{}],[{}]",
            date_time_sent,
            offset
        );

        let outbound = (received & RECV_RECEIVED) != RECV_RECEIVED;
        let email_from = if let Some((email, name, valid)) = buffers.from.get(0) {
            if *valid {
                Some(email_db.add_email(
                    db_conn,
                    user_id,
                    email.as_str(),
                    name.as_ref().map(|val| val.as_str()),
                    if outbound {
                        EmailType::OutboundFrom
                    } else {
                        EmailType::InboundFrom((
                            buffers.flags.iter().any(|name| name.as_str() == r#"\Seen"#),
                            false,
                        ))
                    },
                )?)
            } else {
                None
            }
        } else {
            None
        };

        db_conn.exec_drop(ST_REC_INSERT,params! {
            "user_id"=>user_id,
            "uid"=>buffers.uid.as_str(),
            "guid"=>buffers.guid.as_str(),
            "mailbox"=>buffers.mailbox.as_str(),
            "dt_sent"=>date_time_sent,
            "tz_sent"=>offset.as_str().parse::<f32>().with_context(|| format!("failed to parse [{}] to f32", offset.as_str()))?,
            "dt_recv"=>buffers.date_received.as_str(),
            "dt_saved"=>buffers.date_saved.as_str(),
            "size"=>buffers.size_physical,
            "subj"=>buffers.subj.as_str(),
            "outbound"=>(received & RECV_RECEIVED) != RECV_RECEIVED,
                "mail_from" => email_from}).with_context(|| "failed to insert record".to_owned())
            .with_context(|| "failed to insert record".to_owned())?;

        let record_id = db_conn.last_insert_id();

        if !buffers.flags.is_empty() {
            debug!("process_record: inserting flags: {:?}", buffers.flags);
            db_conn
                .exec_batch(
                    ST_IF_INSERT,
                    buffers
                        .flags
                        .iter()
                        .map(|flag| params! {"record_id" => record_id, "name" => flag.as_str()}),
                )
                .with_context(|| "failed to insert imap_flags".to_owned())?;
            debug!("process_record: flags inserted");
        }
        if !buffers.hdr.is_empty() {
            db_conn
                .exec_batch(
                    ST_HDR_INSERT,
                    buffers
                        .hdr
                        .iter()
                        .filter(|(name, _value)| {
                            !matches!(
                                name.as_str(),
                                HDR_NAME_FROM
                                    | HDR_NAME_TO
                                    | HDR_NAME_SUBJ
                                    | HDR_NAME_CC
                                    | HDR_NAME_BCC
                            )
                        })
                        .enumerate()
                        .map(|(idx, hdr)| {
                            params! {   "record_id"=> record_id,
                            "seq"=> idx,
                            "name" => hdr.0.to_owned(),
                            "value" => hdr.1.to_owned()}
                        }),
                )
                .with_context(|| "process_record: failed to insert headers".to_owned())?;
        }
        if !buffers.to.is_empty() {
            for (seq, (email, name, valid)) in buffers.to.iter().enumerate() {
                if *valid {
                    let email_id = email_db.add_email(
                        db_conn,
                        user_id,
                        email.as_str(),
                        name.as_ref().map(|val| val.as_str()),
                        if outbound {
                            EmailType::OutboundReceipient
                        } else {
                            EmailType::Other
                        },
                    )?;

                    db_conn
                        .exec_drop(
                            ST_MTO_INSERT,
                            params! {   "seq"=>seq, "record_id"=> record_id, "email_id"=> email_id },
                        )
                        .with_context(|| "process_record: failed to insert mail_to".to_owned())?;
                }
            }
        }
        if !buffers.cc.is_empty() {
            for (seq, (email, name, valid)) in buffers.cc.iter().enumerate() {
                if *valid {
                    let email_id = email_db.add_email(
                        db_conn,
                        user_id,
                        email.as_str(),
                        name.as_ref().map(|val| val.as_str()),
                        if outbound {
                            EmailType::OutboundAuxReceipient
                        } else {
                            EmailType::Other
                        },
                    )?;

                    db_conn
                        .exec_drop(
                            ST_MCC_INSERT,
                            params! {  "seq"=>seq,  "record_id"=> record_id, "email_id"=> email_id },
                        )
                        .with_context(|| "process_record: failed to insert mail_cc".to_owned())?;
                }
            }
        }
        if !buffers.bcc.is_empty() {
            for (seq, (email, name, valid)) in buffers.bcc.iter().enumerate() {
                if *valid {
                    let email_id = email_db.add_email(
                        db_conn,
                        user_id,
                        email.as_str(),
                        name.as_ref().map(|val| val.as_str()),
                        if outbound {
                            EmailType::OutboundAuxReceipient
                        } else {
                            EmailType::Other
                        },
                    )?;

                    db_conn
                        .exec_drop(
                            ST_MBCC_INSERT,
                            params! { "seq"=>seq, "record_id"=> record_id, "email_id"=> email_id },
                        )
                        .with_context(|| "process_record: failed to insert mail_bcc".to_owned())?;
                }
            }
        }

        Ok(())
    } else {
        Err(anyhow!(
            "process_record: missing FetchFieldRes: received: {:x}, expected: {:x}",
            received,
            RECV_REQUIRED
        ))
    }
}

pub struct Buffers {
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

impl Buffers {
    pub fn new() -> Buffers {
        Buffers {
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

const HDR_NAME_FROM: &str = "from";
const HDR_NAME_TO: &str = "to";
const HDR_NAME_CC: &str = "cc";
const HDR_NAME_BCC: &str = "bcc";
const HDR_NAME_SUBJ: &str = "subject";
const HDR_NAME_RECV: &str = "received";

const RECV_UID: u32 = 0x1;
const RECV_GUID: u32 = 0x2;
const RECV_DATE_RECV: u32 = 0x4;
const RECV_DATE_SAVD: u32 = 0x8;
const RECV_DATE_SENT: u32 = 0x10;
const RECV_SIZE: u32 = 0x20;
const RECV_MAILBOX: u32 = 0x40;
const RECV_FLAGS: u32 = 0x80;
const RECV_HDRS: u32 = 0x100;
const RECV_FROM: u32 = 0x200;
const RECV_TO: u32 = 0x400;
const RECV_SUBJ: u32 = 0x800;
const RECV_CC: u32 = 0x1000;
const RECV_BCC: u32 = 0x2000;
const RECV_RECEIVED: u32 = 0x4000;

const RECV_HEADERS: u32 = RECV_TO | RECV_FROM | RECV_SUBJ;
const RECV_HEADERS_ALL: u32 = RECV_HEADERS | RECV_CC | RECV_BCC | RECV_RECEIVED;

const RECV_REQUIRED: u32 = RECV_UID
    | RECV_GUID
    | RECV_DATE_SAVD
    | RECV_DATE_RECV
    | RECV_DATE_SENT
    | RECV_SIZE
    | RECV_MAILBOX
    | RECV_FLAGS
    | RECV_HDRS;

const ST_REC_INSERT: &str = r#"insert into record (user_id,uid,guid,mailbox,dt_sent,tz_sent,dt_recv,dt_saved,size,mail_subj,outbound, mail_from)
    values(:user_id,:uid,:guid,:mailbox,:dt_sent,:tz_sent,:dt_recv,:dt_saved,:size,:subj,:outbound, :mail_from)"#;

const ST_IF_INSERT: &str = r#"insert into imap_flag (record_id, name) values(:record_id,:name)"#;

const ST_HDR_INSERT: &str =
    r#"insert into header (record_id, seq, name, value) values(:record_id,:seq,:name,:value)"#;

const ST_MTO_INSERT: &str =
    r#"insert into mail_to (record_id, email_id, seq) values(:record_id,:email_id,:seq)"#;

const ST_MCC_INSERT: &str =
    r#"insert into mail_cc (record_id, email_id, seq) values(:record_id,:email_id,:seq)"#;

const ST_MBCC_INSERT: &str =
    r#"insert into mail_bcc (record_id, email_id, seq) values(:record_id,:email_id,:seq)"#;
