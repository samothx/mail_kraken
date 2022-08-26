use crate::doveadm::{Fetch, FetchParams, ImapField, SearchParam};
use anyhow::{anyhow, Context, Result};
use log::{debug, info};
use mysql_async::prelude::{Query, Queryable, WithParams};
use mysql_async::{params, Pool};

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

struct db_user {
    id: i64,
    user: String,
}

pub async fn init_user(pool: Pool, user: &str) -> Result<()> {
    debug!("init_user: called for {}", user);
    let mut db_conn = pool.get_conn().await?;

    let mut user_id: Option<u64> = r"select id from user where user=:user"
        .with(params! {"user"=>user})
        .first(&mut db_conn)
        .await?;
    if user_id.is_none() {
        r"insert into user (user) values(:user)"
            .with(params! {"user"=>user})
            .ignore(&mut db_conn)
            .await?;
        user_id = db_conn.last_insert_id()
    }

    if let Some(user_id) = user_id {
        debug!("init_user: fetching,  id: {} ", user_id);
        let fetch_params = FetchParams::new(user.to_owned())
            .add_field(ImapField::Flags)
            .add_field(ImapField::Guid)
            .add_field(ImapField::Mailbox)
            .add_field(ImapField::Hdr)
            .add_search_param(SearchParam::All);
        let mut fetch_cmd = Fetch::new(fetch_params)?;
        while let Some(record) = fetch_cmd.parse_record().await? {
            debug!("got record: {:?}", record);
        }
        debug!("done parsing records");
        Ok(())
    } else {
        Err(anyhow!(
            "failed to get user id for {}, trying select & insert",
            user
        ))
    }
}
