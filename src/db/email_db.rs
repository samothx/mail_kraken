use crate::db::DbConCntr;
use anyhow::{anyhow, Context, Result};
use log::warn;
use mysql_async::prelude::{BatchQuery, Query, WithParams};
use mysql_async::{params, ServerError};
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub struct EmailDb {
    email: BTreeMap<String, EmailInfo>,
    name: BTreeSet<(String, u64)>,
}

impl EmailDb {
    pub fn new() -> Self {
        Self {
            email: BTreeMap::new(),
            name: BTreeSet::new(),
        }
    }

    pub async fn add_email(
        &mut self,
        db_conn: &mut DbConCntr,
        email: &str,
        name: Option<&str>,
        email_type: EmailType,
    ) -> Result<u64> {
        let email_id = if let Some(info) = self.email.get_mut(email) {
            // work with cached value
            email_type.process(info);
            // update to db ?
            if let Some(name) = name {
                let _ = info.names.insert(name.to_owned());
            }
            info.id
        } else {
            // try to insert
            let email_id = if let Err(e) = r#"insert into email (email) values(:email)"#
                .with(params! {
                    "email" => email
                })
                .ignore(&mut db_conn.db_conn)
                .await
            {
                if is_db_dup_key(&e) {
                    if let Some(res) = r#"select id from email where email=:email"#
                        .with(params! {"email" => email})
                        .first(&mut db_conn.db_conn)
                        .await?
                    {
                        res
                    } else {
                        return Err(anyhow!(
                        "add_email: failed to select id for email: {}",
                        email
                    ));
                    }
                } else {
                    return Err(e).with_context(|| format!("failed to insert email: {}", email));
                }
            } else {
                db_conn.db_conn.last_insert_id().ok_or_else(|| {
                    anyhow!("add_email: failed to retrieve last insert id for email")
                })?
            };
            let mut email_info = EmailInfo {
                id: email_id,
                referenced: 0,
                inbound: 0,
                outbound: 0,
                seen: 0,
                spam: 0,
                names: HashSet::new(),
            };

            email_type.process(&mut email_info);

            if let Some(name) = name {
                let _ = email_info.names.insert(name.to_owned());
            }
            if let Some(_) = self.email.insert(email.to_owned(), email_info) {
                panic!("add_email: unexpected existing value in table");
            }
            email_id
        };

        Ok(email_id)
    }

    pub async fn flush_to_db(&self, db_conn: &mut DbConCntr, user_id: u64) -> Result<()> {
        r#"insert into mail_stats (email_id,user_id,inbound,outbound,seen,spam) values(:email_id,:user_id,:inbound,:outbound,:seen,:spam)"#
            .with(self.email.iter().map(|(_, info)|
            params! {
                "email_id"=>info.id,
                "user_id"=>user_id,
                "inbound"=>info.inbound,
                "outbound"=>info.outbound,
                "seen"=>info.seen,
                "spam"=>info.spam
            })).batch(&mut db_conn.db_conn).await?;

        r#"insert into mail_name (email_id,name) values(:email_id,:name)"#
            .with(
                self.email
                    .iter()
                    .flat_map(|(_, info)| info.names.iter().map(|name| (info.id, name)))
                    .map(|(email_id, name)| {
                        params! {
                            "email_id"=>email_id
                            ,"name"=>name.as_str()
                        }
                    }),
            )
            .batch(&mut db_conn.db_conn)
            .await?;
        Ok(())
    }
}

pub enum EmailType {
    InboundFrom((bool, bool)), // the sender of an inbound mail
    OutboundFrom,              // the sender of an outbound mail
    Other,                     // to, cc, bcc of in/outbound mail
}

impl EmailType {
    pub fn process(&self, info: &mut EmailInfo) {
        info.referenced += 1;
        match self {
            EmailType::InboundFrom((seen, spam)) => {
                info.inbound += 1;
                if *seen {
                    info.seen += 1
                }
                if *spam {
                    info.spam += 1
                }
            }
            EmailType::OutboundFrom => {
                info.outbound += 1;
            }
            _ => (),
        }
    }
}

pub struct EmailInfo {
    id: u64,
    referenced: u32,
    inbound: u32,
    outbound: u32,
    seen: u32,
    spam: u32,
    names: HashSet<String>,
}

fn is_db_dup_key(err: &mysql_async::Error) -> bool {
    if let mysql_async::Error::Server(err) = err {
        return err.code == 1064;
    }
    false
}
