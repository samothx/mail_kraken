use anyhow::{Context, Result};
use log::{debug, info};
use mysql_async::{prelude::Query, Pool};

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

pub struct DbConCntr {
    db_conn: Conn,
}
