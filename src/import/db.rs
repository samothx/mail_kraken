use super::email_db::{EmailDb, EmailType};
use crate::doveadm::{FetchFieldRes, FetchRecord};
use crate::import::email_parser::EmailParser;
use anyhow::{anyhow, Context, Result};
use log::{debug, trace, warn};
use mysql::{params, prelude::Queryable, Conn};
use regex::Regex;
use std::process::id;

const PROFILE: bool = true;

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
    spam_score_regex: &Regex,
    date_time_tz_regex: &Regex,
    email_parser: &mut EmailParser,
    email_db: &mut EmailDb,
) -> Result<()> {
    buffers.clear();

    let mut received = 0;

    // collect fetch fields, ensure all needed fields are there & store in Buffers
    {
        let ts_start = if PROFILE {
            Some(chrono::Local::now())
        } else {
            None
        };

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
        if PROFILE {
            debug!(
                "process_record: evaluating fetch field results took {} ms",
                (chrono::Local::now() - ts_start.unwrap()).num_milliseconds()
            );
        }
    }

    if (received & RECV_HDRS) == RECV_HDRS {
        // extract fields fom headers
        {
            let ts_start = if PROFILE {
                Some(chrono::Local::now())
            } else {
                None
            };

            for (name, value) in buffers.hdr.iter() {
                match name.to_lowercase().as_str() {
                    HDR_NAME_RECV => {
                        received |= RECV_RECEIVED;
                    }
                    HDR_NAME_MSG_ID => {
                        let msg_id = value.trim().trim_start_matches('<').trim_end_matches('>');

                        buffers.msg_id = if msg_id.is_empty() {
                            None
                        } else {
                            Some(msg_id.to_owned())
                        };
                    }
                    HDR_NAME_REFERENCED => {
                        if !value.is_empty() {
                            value.split_whitespace().for_each(|part| {
                                buffers.references.push(
                                    part.trim_start_matches('<')
                                        .trim_end_matches('>')
                                        .to_owned(),
                                )
                            });
                        }
                    }
                    HDR_NAME_RECV_SPF => {
                        buffers.spf = if value.to_lowercase().starts_with("pass") {
                            Some(true)
                        } else if value.to_lowercase().starts_with("none") {
                            Some(false)
                        } else {
                            None
                        }
                    }
                    HDR_NAME_X_SPAM_STATUS => {
                        buffers.spam =
                            if let Some(captures) = spam_score_regex.captures(value.as_str()) {
                                if let Ok(score) = captures[2].parse() {
                                    if let Ok(required) = captures[3].parse() {
                                        (
                                            Some(captures[1].eq_ignore_ascii_case("yes")),
                                            Some(score),
                                            Some(required),
                                        )
                                    } else {
                                        warn!(
                                    "process_record: failed to parse required capture [{}] to f32",
                                    &captures[3]
                                );
                                        (
                                            Some(captures[1].eq_ignore_ascii_case("yes")),
                                            Some(score),
                                            None,
                                        )
                                    }
                                } else {
                                    warn!(
                                        "process_record: failed to parse score capture [{}] to f32",
                                        &captures[2]
                                    );
                                    (Some(captures[1].eq_ignore_ascii_case("yes")), None, None)
                                }
                            } else {
                                (None, None, None)
                            }
                    }
                    HDR_NAME_TO => {
                        email_parser.parse(value.as_str(), &mut buffers.to);
                        received |= RECV_TO;
                    }
                    HDR_NAME_FROM => {
                        email_parser.parse(value.as_str(), &mut buffers.from);
                        received |= RECV_FROM;
                    }
                    HDR_NAME_CC => {
                        email_parser.parse(value.as_str(), &mut buffers.cc);
                        received |= RECV_CC;
                    }
                    HDR_NAME_BCC => {
                        email_parser.parse(value.as_str(), &mut buffers.bcc);
                        received |= RECV_BCC;
                    }
                    HDR_NAME_SUBJ => {
                        buffers.subj = value.to_owned();
                        received |= RECV_SUBJ;
                    }

                    _ => (),
                };
                /* if (received & RECV_HEADERS_ALL) == RECV_HEADERS_ALL {
                    break;
                }*/
            }

            if PROFILE {
                debug!(
                    "process_record: parsing headers took {} ms",
                    (chrono::Local::now() - ts_start.unwrap()).num_milliseconds()
                );
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
        // extract date/time & timezone from date sent
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

        // its an outbound message if there are no received headers
        // might be an unsent message too in drafts or trash
        let outbound = (received & RECV_RECEIVED) != RECV_RECEIVED;

        // extract and save first email-from if valid
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
                            buffers.spam.0.unwrap_or(false),
                        ))
                    },
                )?)
            } else {
                None
            }
        } else {
            None
        };

        {
            let ts_start = if PROFILE {
                Some(chrono::Local::now())
            } else {
                None
            };

            // insert record
            db_conn.exec_drop(ST_REC_INSERT, params! {
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
            "mail_from" => email_from,
            "spf"=>buffers.spf,
            "is_spam"=>buffers.spam.0,
            "spam_score"=>buffers.spam.1,
            "spam_req"=>buffers.spam.2,
            "msg_id"=>buffers.msg_id.as_ref(),
        }).with_context(|| "failed to insert record".to_owned())
                .with_context(|| "failed to insert record".to_owned())?;

            if PROFILE {
                debug!(
                    "process_record: insert record took {} ms",
                    (chrono::Local::now() - ts_start.unwrap()).num_milliseconds()
                );
            }
        }

        let record_id = db_conn.last_insert_id();

        if !buffers.flags.is_empty() {
            let ts_start = if PROFILE {
                Some(chrono::Local::now())
            } else {
                None
            };

            db_conn
                .exec_batch(
                    ST_IF_INSERT,
                    buffers
                        .flags
                        .iter()
                        .map(|flag| params! {"record_id" => record_id, "name" => flag.as_str()}),
                )
                .with_context(|| "failed to insert imap_flags".to_owned())?;

            if PROFILE {
                debug!(
                    "process_record: inserting flags took {} ms",
                    (chrono::Local::now() - ts_start.unwrap()).num_milliseconds()
                );
            }
        }

        if !buffers.hdr.is_empty() {
            // insert headers
            let ts_start = if PROFILE {
                Some(chrono::Local::now())
            } else {
                None
            };

            db_conn
                .exec_batch(
                    ST_HDR_INSERT,
                    buffers
                        .hdr
                        .iter()
                        .filter(|(name, _value)| {
                            !matches!(
                                name.to_lowercase().as_str(),
                                HDR_NAME_FROM
                                    | HDR_NAME_TO
                                    | HDR_NAME_SUBJ
                                    | HDR_NAME_CC
                                    | HDR_NAME_BCC
                                    | HDR_NAME_REFERENCED
                                    | HDR_NAME_RECV_SPF
                                    | HDR_NAME_MSG_ID
                                    | HDR_NAME_FROM
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

            if PROFILE {
                debug!(
                    "process_record: insert headers took {} ms",
                    (chrono::Local::now() - ts_start.unwrap()).num_milliseconds()
                );
            }
        }

        if !buffers.to.is_empty() {
            // insert To email addresses & names if valid

            let ts_start = if PROFILE {
                Some(chrono::Local::now())
            } else {
                None
            };

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
            if PROFILE {
                debug!(
                    "process_record: insert mail_to took {} ms",
                    (chrono::Local::now() - ts_start.unwrap()).num_milliseconds()
                );
            }
        }
        if !buffers.cc.is_empty() {
            // insert Cc email addresses & names if valid
            let ts_start = if PROFILE {
                Some(chrono::Local::now())
            } else {
                None
            };

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

                if PROFILE {
                    debug!(
                        "process_record: insert mail_cc took {} ms",
                        (chrono::Local::now() - ts_start.unwrap()).num_milliseconds()
                    );
                }
            }
        }
        if !buffers.bcc.is_empty() {
            // insert Bcc email addresses & names if valid

            let ts_start = if PROFILE {
                Some(chrono::Local::now())
            } else {
                None
            };

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
            if PROFILE {
                debug!(
                    "process_record: insert mail_bcc took {} ms",
                    (chrono::Local::now() - ts_start.unwrap()).num_milliseconds()
                );
            }
        }

        if !buffers.references.is_empty() {
            // insert Referenced message ids

            let ts_start = if PROFILE {
                Some(chrono::Local::now())
            } else {
                None
            };

            for (seq, msg_id) in buffers.references.iter().enumerate() {
                db_conn
                    .exec_drop(
                        ST_REF_INSERT,
                        params! { "record_id"=> record_id, "seq"=>seq,  "msg_id"=> msg_id },
                    )
                    .with_context(|| "process_record: failed to insert referenced".to_owned())?;
            }
            if PROFILE {
                debug!(
                    "process_record: insert references took {} ms",
                    (chrono::Local::now() - ts_start.unwrap()).num_milliseconds()
                );
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
    spf: Option<bool>,
    spam: (Option<bool>, Option<f32>, Option<f32>),
    msg_id: Option<String>,
    references: Vec<String>,
}

impl Buffers {
    pub fn new() -> Buffers {
        Buffers {
            guid: String::with_capacity(256),
            uid: String::with_capacity(256),
            mailbox: String::with_capacity(256),
            flags: Vec::with_capacity(32),
            hdr: Vec::with_capacity(1024),
            date_received: String::with_capacity(100),
            date_saved: String::with_capacity(100),
            date_sent: String::with_capacity(100),
            size_physical: 0,
            to: Vec::with_capacity(32),
            from: Vec::new(),
            cc: Vec::with_capacity(100),
            bcc: Vec::with_capacity(100),
            subj: String::with_capacity(1024),
            spf: None,
            spam: (None, None, None),
            msg_id: None,
            references: Vec::with_capacity(32),
        }
    }

    pub fn clear(&mut self) {
        self.guid.clear();
        self.uid.clear();
        self.mailbox.clear();
        self.flags.clear();
        self.hdr.clear();
        self.date_received.clear();
        self.date_saved.clear();
        self.date_sent.clear();
        self.size_physical = 0;
        self.to.clear();
        self.from.clear();
        self.cc.clear();
        self.bcc.clear();
        self.subj.clear();
        self.spf = None;
        self.spam = (None, None, None);
        self.msg_id = None;
        self.references.clear();
    }
}

const HDR_NAME_FROM: &str = "from";
const HDR_NAME_TO: &str = "to";
const HDR_NAME_CC: &str = "cc";
const HDR_NAME_BCC: &str = "bcc";
const HDR_NAME_SUBJ: &str = "subject";
const HDR_NAME_RECV: &str = "received";
const HDR_NAME_RECV_SPF: &str = "received-spf";
const HDR_NAME_X_SPAM_STATUS: &str = "x-spam-status";
const HDR_NAME_MSG_ID: &str = "message-id";
const HDR_NAME_REFERENCED: &str = "references";

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

const ST_REC_INSERT: &str = r#"insert into record (user_id,uid,guid,mailbox,dt_sent,tz_sent,dt_recv,
    dt_saved,size,mail_subj,outbound, mail_from, spf, is_spam, spam_score, spam_req, msg_id)
    values(:user_id,:uid,:guid,:mailbox,:dt_sent,:tz_sent,:dt_recv,:dt_saved,:size,:subj,:outbound, 
    :mail_from, :spf, :is_spam, :spam_score, :spam_req, :msg_id)"#;

const ST_IF_INSERT: &str = r#"insert into imap_flag (record_id, name) values(:record_id,:name)"#;

const ST_HDR_INSERT: &str =
    r#"insert into header (record_id, seq, name, value) values(:record_id,:seq,:name,:value)"#;

const ST_MTO_INSERT: &str =
    r#"insert into mail_to (record_id, email_id, seq) values(:record_id,:email_id,:seq)"#;

const ST_MCC_INSERT: &str =
    r#"insert into mail_cc (record_id, email_id, seq) values(:record_id,:email_id,:seq)"#;

const ST_MBCC_INSERT: &str =
    r#"insert into mail_bcc (record_id, email_id, seq) values(:record_id,:email_id,:seq)"#;

const ST_REF_INSERT: &str =
    r#"insert into referenced (record_id, seq, msg_id) values(:record_id,:seq,:msg_id)"#;
