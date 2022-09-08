use super::doveadm::{Fetch, FetchParams, ImapField, SearchParam};
use super::{Config, ImportArgs};
use anyhow::{anyhow, Context, Result};
use log::{error, info};
use mod_logger::Logger;
use mysql::{prelude::Queryable, Pool};

use nix::unistd::getuid;
use regex::Regex;

mod db;
use db::init_user;
mod email_db;

mod email_parser;
use crate::import::db::{process_record, Buffers};
use crate::import::email_db::EmailDb;
use crate::DB_VERSION;
use email_parser::EmailParser;

// sync import used by sync process
pub fn import(args: ImportArgs) -> Result<()> {
    Logger::set_default_level(args.log_level);
    Logger::set_color(true);
    Logger::set_brief_info(true);

    info!("started for user : '{}'", args.user);

    if !getuid().is_root() {
        return Err(anyhow!("please run this command as root"));
    }

    let config = Config::from_file().with_context(|| "failed to read config file".to_owned())?;

    let pool = Pool::new(
        config
            .get_db_url()
            .ok_or_else(|| anyhow!("db_url is not set in config"))?
            .as_str(),
    )
    .with_context(|| "failed to connect to db".to_owned())?;

    let mut pooled_db_conn = pool.get_conn()?;
    let db_conn = pooled_db_conn.as_mut();
    if let Some(version) = db_conn.query_first::<u32, &str>(r#"select max(version) from db_ver"#)? {
        if version != DB_VERSION {
            return Err(anyhow!(format!(
                "invalid database version found: got {}, required {}",
                version, DB_VERSION
            )));
        }
    } else {
        return Err(anyhow!("unable to retrieve version from database"));
    }

    let (user_id, empty) = init_user(db_conn, args.user.as_str())?;

    let fetch_params = FetchParams::new(args.user.to_owned())
        .add_field(ImapField::Flags)
        .add_field(ImapField::Guid)
        .add_field(ImapField::Uid)
        .add_field(ImapField::Mailbox)
        .add_field(ImapField::Hdr)
        .add_field(ImapField::SizePhysical)
        .add_field(ImapField::DateSent)
        .add_field(ImapField::DateSaved)
        .add_field(ImapField::DateReceived)
        .add_search_param(SearchParam::All);
    let mut fetch_cmd = Fetch::new(fetch_params)?;

    // Yes, score=10.7 required=7.0 tests=BAYES_50,DIET_1,HTML_MESSAGE,
    // 	HTML_OFF_PAGE,RCVD_IN_SBL_CSS,RDNS_NONE,T_REMOTE_IMAGE,URIBL_ABUSE_SURBL,
    // 	URIBL_BLOCKED,URIBL_DBL_SPAM autolearn=no autolearn_force=no version=3.4.0
    let spam_score_regex = Regex::new(r#"^(Yes|No), score=(\d+\.\d+) required=(\d+\.\d+) "#)?;
    let date_time_tz_regex = Regex::new(r"^(\d{4}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2})\s\(([^)]+)\)$")?;
    let mut email_db = EmailDb::new();
    let mut email_parser = EmailParser::new();
    let mut buffers = Buffers::new();

    let mut msg_count = 0usize;
    let mut insert_count = 0usize;
    let ts_start = chrono::Local::now();

    if empty {
        let mut last_flush = ts_start;
        while let Some(record) = match fetch_cmd.parse_record() {
            Ok(res) => res,
            Err(e) => {
                error!("scan: parser failed with {:?}", e);
                return Err(e);
            }
        } {
            msg_count += 1;
            match process_record(
                db_conn,
                user_id,
                record,
                &mut buffers,
                &spam_score_regex,
                &date_time_tz_regex,
                &mut email_parser,
                &mut email_db,
            ) {
                Ok(_) => {
                    insert_count += 1;
                }
                Err(e) => {
                    error!("import: process_record failed: {:?}", e);
                    continue;
                }
            };
            if msg_count % 20 == 0 {
                let now = chrono::Local::now();
                let duration = now - ts_start;
                info!(
                    "import: processed {} messages, inserted {} in {} seconds, {} inserted/second",
                    msg_count,
                    insert_count,
                    duration.num_seconds(),
                    insert_count * 1000 / duration.num_milliseconds() as usize
                );

                if (now - last_flush).num_seconds() > 20 {
                    last_flush = now;
                    email_db.flush_to_db(db_conn, user_id)?;
                }
            }
        }

        email_db.flush_to_db(db_conn, user_id)?;
    } else {
        return Err(anyhow!(
            "user db contains values & update is not implemented"
        ));
    }

    Ok(())
}
