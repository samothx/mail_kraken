use crate::db::DbConCntr;
use anyhow::{anyhow, Context, Result};
use mysql_async::params;
use mysql_async::prelude::{Query, WithParams};
use std::collections::{BTreeMap, HashSet};

pub struct EmailDb {
    email: BTreeMap<String, EmailInfo>,
}

impl EmailDb {
    pub fn new() -> Self {
        Self {
            email: BTreeMap::new(),
        }
    }

    pub async fn add_email(
        &mut self,
        db_conn: &mut DbConCntr,
        user_id: u64,
        email: &str,
        name: Option<&str>,
        email_type: EmailType,
    ) -> Result<u64> {
        let email_id = if let Some(info) = self.email.get_mut(email) {
            // work with cached value
            email_type.process(info);

            r#"update mail_stats set referenced=:referenced,inbound=:inbound,outbound=:outbound,seen=:seen,spam=:spam) where email_id=:email_id and user_id=:user_id"#
                .with(params! {
                    "email_id"=>info.id,
                    "user_id"=>user_id,
                    "referenced"=> info.referenced,
                    "inbound"=>info.inbound,
                    "outbound"=>info.outbound,
                    "seen"=> info.seen,
                    "spam"=>info.spam }).ignore(&mut db_conn.db_conn).await?;

            // update to db ?
            if let Some(name) = name {
                if info.names.insert(name.to_owned()) {
                    r#"insert into mail_name (email_id,name) values(:email_id,:name)"#
                        .with(params! {
                            "email_id"=>info.id,"name"=>name
                        })
                        .ignore(&mut db_conn.db_conn)
                        .await?;
                }
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
            let mut info = EmailInfo {
                id: email_id,
                referenced: 0,
                inbound: 0,
                outbound: 0,
                seen: 0,
                spam: 0,
                names: HashSet::new(),
            };

            email_type.process(&mut info);

            r#"insert into mail_stats (email_id,user_id,referenced,inbound,outbound,seen,spam) values(:email_id,:user_id,:referenced,:inbound,:outbound,:seen,:spam)"#
                .with(params! {
                    "email_id"=>email_id,
                    "user_id"=>user_id,
                    "referenced"=> info.referenced,
                    "inbound"=>info.inbound,
                    "outbound"=>info.outbound,
                    "seen"=> info.seen,
                    "spam"=>info.spam }).ignore(&mut db_conn.db_conn).await?;

            if let Some(name) = name {
                if info.names.insert(name.to_owned()) {
                    r#"insert into mail_name (email_id,name) values(:email_id,:name)"#
                        .with(params! {
                            "email_id"=>email_id,"name"=>name
                        })
                        .ignore(&mut db_conn.db_conn)
                        .await?;
                }
            }
            if let Some(_) = self.email.insert(email.to_owned(), info) {
                panic!("add_email: unexpected existing value in table");
            }
            email_id
        };

        Ok(email_id)
    }

    /*
       pub async fn flush_to_db(mut self, db_conn: &mut DbConCntr, user_id: u64) -> Result<()> {
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
           self.email.clear();

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
           self.name.clear();
           Ok(())
       }
    */
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
        return err.code == 1062;
    }
    false
}
