use anyhow::{Context, Result};
use mysql::{params, prelude::Queryable, Conn};
use std::collections::{BTreeMap, HashSet};

const ST_MS_INSERT: &str = r#"insert into mail_stats (email_id,user_id,referenced,inbound,outbound,receiver, aux_receiver,seen,spam)
    values(:email_id,:user_id,:referenced,:inbound,:outbound,:receiver,:aux_receiver,:seen,:spam)"#;
const ST_MS_UPDATE: &str = r#"update mail_stats set referenced=:referenced,inbound=:inbound,outbound=:outbound,receiver=:receiver,
    aux_receiver=:aux_receiver,seen=:seen,spam=:spam where email_id=:email_id and user_id=:user_id"#;
const ST_MN_INSERT: &str = r#""#;

const ST_M_INSERT: &str = r#"insert into email (email) values(:email)"#;
const ST_M_SELECT: &str = r#"select id from email where email=:email"#;

pub struct EmailDb {
    email: BTreeMap<String, EmailInfo>,
}

impl EmailDb {
    pub fn new() -> Self {
        Self {
            email: BTreeMap::new(),
        }
    }

    pub fn add_email(
        &mut self,
        db_conn: &mut Conn,
        user_id: u64,
        email: &str,
        name: Option<&str>,
        email_type: EmailType,
    ) -> Result<u64> {
        let email_id = if let Some(info) = self.email.get_mut(email) {
            // work with cached value
            info.process(&email_type);

            // update to db ?
            if let Some(name) = name {
                if info.names.insert(name.to_owned()) {
                    db_conn.exec_drop(
                        ST_MN_INSERT,
                        params! {
                            "email_id"=>info.id,"name"=>name
                        },
                    )?;
                }
            }
            info.id
        } else {
            // try to insert
            let email_id = if let Err(e) =
                db_conn.exec_drop(ST_M_INSERT, params! {"email" => email})
            {
                if is_db_dup_key(&e) {
                    if let Some(email_id) =
                        db_conn.exec_first(ST_M_SELECT, params! {"email" => email})?
                    {
                        email_id
                    } else {
                        return Err(e).with_context(|| {
                            format!("failed to retrieve id for email: {}", email)
                        });
                    }
                } else {
                    return Err(e).with_context(|| format!("failed to insert email: {}", email));
                }
            } else {
                db_conn.last_insert_id()
            };

            let mut info = EmailInfo::new(email_id, &email_type);
            db_conn.exec_drop(
                ST_MS_INSERT,
                params! {
                    "email_id"=>info.id,
                    "user_id"=>user_id,
                    "referenced"=>info.referenced,
                    "inbound"=>info.inbound,
                    "outbound"=>info.outbound,
                     "receiver"=>info.receiver,
                     "aux_receiver"=>info.aux_receiver,
                    "seen"=>info.seen,
                    "spam"=>info.spam
                },
            )?;

            if let Some(name) = name {
                if info.names.insert(name.to_owned()) {
                    db_conn.exec_drop(
                        ST_MN_INSERT,
                        params! {
                            "email_id"=>email_id,"name"=>name
                        },
                    )?;
                }
            }
            if let Some(_) = self.email.insert(email.to_owned(), info) {
                panic!("add_email: unexpected existing value in table");
            }
            email_id
        };

        Ok(email_id)
    }

    pub fn flush_to_db(mut self, db_conn: &mut Conn, user_id: u64) -> Result<()> {
        db_conn.exec_batch(
            ST_MS_UPDATE,
            self.email
                .iter_mut()
                .filter(|(_, info)| info.updated)
                .map(|(_, info)| {
                    info.updated = false;
                    params! {
                            "email_id"=>info.id,
                           "user_id"=>user_id,
                           "inbound"=>info.inbound,
                           "outbound"=>info.outbound,
                            "receiver"=> info.receiver,
                           "aux_receiver"=> info.aux_receiver,
                           "seen"=>info.seen,
                           "spam"=>info.spam
                    }
                }),
        )?;
        Ok(())
    }
}

pub struct EmailInfo {
    id: u64,
    referenced: u32,
    inbound: u32,
    outbound: bool,
    receiver: u32,
    aux_receiver: u32,
    seen: u32,
    spam: u32,
    names: HashSet<String>,
    updated: bool,
}

impl EmailInfo {
    pub fn new(id: u64, etype: &EmailType) -> Self {
        let mut res = Self {
            id,
            referenced: 0,
            inbound: 0,
            outbound: false,
            receiver: 0,
            aux_receiver: 0,
            seen: 0,
            spam: 0,
            names: HashSet::new(),
            updated: false,
        };
        res.process(etype);
        res
    }

    pub fn process(&mut self, etype: &EmailType) {
        match etype {
            EmailType::InboundFrom((seen, spam)) => {
                self.inbound += 1;
                if *seen {
                    self.seen += 1
                }
                if *spam {
                    self.spam += 1
                }
            }
            EmailType::OutboundFrom => {
                self.outbound = true;
            }
            EmailType::OutboundReceipient => {
                self.receiver += 1;
            }
            EmailType::OutboundAuxReceipient => {
                self.aux_receiver += 1;
            }
            EmailType::Other => (),
        }
        self.referenced += 1;
        self.updated = true;
    }
}

pub enum EmailType {
    InboundFrom((bool, bool)), // the sender of an inbound mail // todo answered
    OutboundFrom,              // the sender of an outbound mail
    OutboundReceipient,        // the receipient of an outbound mail fom me
    OutboundAuxReceipient,     // the receipient of an outbound mail fom me via cc/bcc
    Other,                     // to, cc, bcc of in/outbound mail
}

fn is_db_dup_key(err: &mysql::Error) -> bool {
    if let mysql::Error::MySqlError(err) = err {
        return err.code == 1062;
    }
    false
}
