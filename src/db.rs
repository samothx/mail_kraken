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

pub async fn scan(mut db_conn: Conn, user: String, user_id: u64) -> Result<()> {
    debug!("init_user: fetching,  id: {} ", user_id);
    let fetch_params = FetchParams::new(user)
        .add_field(ImapField::Flags)
        .add_field(ImapField::Guid)
        .add_field(ImapField::Uid)
        .add_field(ImapField::Mailbox)
        .add_field(ImapField::Hdr)
        .add_search_param(SearchParam::All);
    let mut fetch_cmd = Fetch::new(fetch_params)?;
    while let Some(record) = fetch_cmd.parse_record().await? {
        // debug!("got record: {:?}", record);
        let mut guid: Option<String> = None;
        let mut uid: Option<String> = None;
        let mut mailbox: Option<String> = None;
        let mut flags = Vec::new();
        let mut hdr = Vec::new();

        for item in record.into_iter() {
            debug!("init_user: got {:?}", item);
            match item {
                FetchFieldRes::Uid(value) => {
                    uid = Some(value);
                }
                FetchFieldRes::Guid(value) => {
                    guid = Some(value);
                }
                FetchFieldRes::Mailbox(value) => {
                    mailbox = Some(value);
                }
                FetchFieldRes::Flags(value) => {
                    flags = value;
                }
                FetchFieldRes::Hdr(val) => {
                    hdr = val;
                }
                FetchFieldRes::Generic((_imap_field, _value)) => {
                    todo!()
                }
            }
        }

        if let Some(uid) = uid {
            if let Some(guid) = guid {
                if let Some(mailbox) = mailbox {
                    r"insert into record (user_id,uid,guid,mailbox) values(:user_id,:uid,:guid,:mailbox)"
                            .with(params! {"user_id"=>user_id,"uid"=>uid,"guid"=>guid,"mailbox"=>mailbox})
                            .ignore(&mut db_conn).await?;
                    if let Some(record_id) = db_conn.last_insert_id() {
                        if !flags.is_empty() {
                            r"insert into imap_flag (record_id, name) values(:record_id,:name)"
                                .with(
                                    flags.iter().map(
                                        |flag| params! {"record_id" => record_id, "name" => flag},
                                    ),
                                )
                                .batch(&mut db_conn)
                                .await?;
                        }
                        if !hdr.is_empty() {
                            r"insert into header (record_id, seq, name, value) values(:record_id,:seq,:name:value)"
                                .with(hdr.iter().enumerate().map(|(idx,hdr)| {
                                    params! {
                                        "record_id"=> record_id, 
                                        "seq"=> idx, 
                                        "name" => hdr.0.to_owned(), 
                                        "value" => hdr.1.to_owned()}
                                })).batch(&mut db_conn).await?
                        }
                    } else {
                        error!("init_user: failed to insert record");
                        continue;
                    }
                } else {
                    error!("init_user: mailbox not found in record, skipping record");
                    continue;
                }
            } else {
                error!("init_user: guid not found in record, skipping record");
                continue;
            }
        } else {
            error!("init_user: uid not found in record, skipping record");
            continue;
        }
    }
    debug!("done parsing records");
    Ok(())
}
