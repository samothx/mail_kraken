use crate::{UserId, DB_VERSION};
use anyhow::{anyhow, Context, Result};
use log::{debug, error, info};
use mysql_async::{
    params,
    prelude::{Query, Queryable, WithParams},
    Pool,
};

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
        let db_init_script = String::from_utf8_lossy(include_bytes!("../../sql/init_db_v1.sql"));
        match db_init_script.ignore(&mut db_conn).await {
            Ok(_) => (),
            Err(e) => {
                let err_str = e.to_string();
                error!("failed to initialize database: {:?}", err_str.as_str());
                return Err(e).with_context(|| {
                    format!("failed to initialize database: {:?}", err_str.as_str())
                });
            }
        }

        Ok(())
    } else if !tables.contains(&"db_ver".to_owned()) {
        Err(anyhow!(
            "database is not empty but does not contain the db_ver table"
        ))
    } else {
        // let db_ver: Option<u32> = ;
        if let Some(version) = db_conn
            .query_first::<u32, &str>(r#"select max(version) from db_ver"#)
            .await?
        {
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

pub async fn init_user(pool: Pool, user: &str) -> Result<UserId> {
    debug!("init_user: called for {}", user);

    let mut db_conn = pool.get_conn().await?;

    let user_id = r#"select id from user where user=:user"#
            .with(params! {"user"=>user})
            .first(&mut db_conn)
            .await?;

    Ok(if let Some(user_id) = user_id {
        user_id
    } else {
        r#"insert into user (user) values(:user)"#
            .with(params! {"user"=>user})
            .ignore(&mut db_conn)
            .await?;
        db_conn
            .last_insert_id()
            .ok_or_else(|| anyhow!("init_user: failed to retrieve last insert id"))?
    })
}

/*pub struct DbConCntr {
    db_conn: Conn,
}*/
