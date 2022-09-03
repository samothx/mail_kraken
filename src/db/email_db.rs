use crate::db::DbConCntr;
use anyhow::{anyhow, Result};
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
        is_outbound: bool,
        is_seen: bool,
        is_spam: bool,
    ) -> Result<u64> {
        let email_id = if let Some(info) = self.email.get_mut(email) {
            // work with cached value
            if is_outbound {
                info.outbound += 1
            } else {
                info.inbound += 1
            }
            if is_seen {
                info.seen += 1;
            }
            if is_spam {
                info.spam += 1;
            }
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
                warn!("add_email: failed to insert {:?}", e);
                /* if let mysql_async::Error::ServerError(e) = e {
                    if e.code ==
                }*/
                // else select from existing
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
                db_conn.db_conn.last_insert_id().ok_or_else(|| {
                    anyhow!("add_email: failed to retrieve last insert id for email")
                })?
            };
            let mut email_info = EmailInfo {
                id: email_id,
                inbound: if is_outbound { 0 } else { 1 },
                outbound: if is_outbound { 1 } else { 0 },
                seen: if is_seen { 1 } else { 0 },
                spam: if is_spam { 1 } else { 0 },
                names: HashSet::new(),
            };
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

struct EmailInfo {
    id: u64,
    inbound: u32,
    outbound: u32,
    seen: u32,
    spam: u32,
    names: HashSet<String>,
}
