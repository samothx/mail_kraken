use crate::httpd::error::{ApiError, SiteError, SiteResult};
use crate::httpd::state_data::StateData;
use crate::httpd::util::{format_data_size, format_percent};
use actix_identity::Identity;
use actix_web::{get, web, HttpResponse};
use anyhow::{anyhow, Result};
use askama::Template;
use log::debug;
use mysql_async::prelude::{Query, WithParams};
use mysql_async::{from_row, params, prelude::Queryable, Conn};

#[derive(Template)]
#[template(path = "dash.html")]
struct UserDashboard {
    msg_count: u64,
    msg_data: String,
    fract_seen: String,
    email_count: u64,
    // email_sndr: String,
    email_recvr: String,
}

const ST_SEL_COUNT_RCRD: &str = r#"select count(*),sum(size) from record where user_id=:user_id"#;
const ST_SEL_COUNT_SEEN: &str = r#"select count(*) from record
left join imap_flag on record.id = imap_flag.record_id
where imap_flag.name = '\\Seen' and record.user_id=:user_id"#;

const ST_SEL_COUNT_EMAIL: &str =
    r#"select count(*) from mail_stats where user_id=:user_id and inbound>0"#;
const ST_SEL_COUNT_EMAIL_RECV: &str =
    r#"select count(*) from mail_stats where user_id=:user_id and receiver>0 and inbound>0"#;

impl UserDashboard {
    pub async fn new(db_conn: &mut Wa, user_id: u64) -> Result<Self> {
        let (msg_count, msg_size) = if let Some(res) = ST_SEL_COUNT_RCRD
            .with(params! {"user_id"=>user_id})
            .first(&mut db_conn.db_conn)
            .await?
        {
            from_row::<(u64, u64)>(res)
        } else {
            return Err(anyhow!("no result from query: {}", ST_SEL_COUNT_RCRD));
        };

        let fract_seen = if let Some(res) = ST_SEL_COUNT_SEEN
            .with(params! {"user_id"=>user_id})
            .first(&mut db_conn.db_conn)
            .await?
        {
            from_row::<(u64)>(res)
        } else {
            return Err(anyhow!("no result from query: {}", ST_SEL_COUNT_SEEN));
        };

        let email_count = if let Some(res) = ST_SEL_COUNT_EMAIL
            .with(params! {"user_id"=>user_id})
            .first(&mut db_conn.db_conn)
            .await?
        {
            from_row::<(u64)>(res)
        } else {
            return Err(anyhow!("no result from query: {}", ST_SEL_COUNT_EMAIL));
        };

        let email_recvr = if let Some(res) = ST_SEL_COUNT_EMAIL_RECV
            .with(params! {"user_id"=>user_id})
            .first(&mut db_conn.db_conn)
            .await?
        {
            from_row::<(u64)>(res)
        } else {
            return Err(anyhow!("no result from query: {}", ST_SEL_COUNT_EMAIL_RECV));
        };

        Ok(UserDashboard {
            msg_count,
            msg_data: format_data_size(msg_size),
            fract_seen: format_percent(fract_seen, msg_count),
            email_count,
            email_recvr: format_percent(email_recvr, email_count),
        })
    }
}

#[get("/dash")]
pub async fn user_dash(id: Identity, state: web::Data<StateData>) -> SiteResult {
    debug!("admin_dash: called with id: {:?}", id.identity());
    if let Some(_login) = id.identity() {
        let state = state
            .get_state()
            .map_err(|e| SiteError::Internal(Some(e.to_string())))?;

        if let Some(pool) = state.db_conn.as_ref() {
            let mut db_conn = pool
                .get_conn()
                .await
                .map_err(|e| SiteError::Internal(Some(e.to_string())))?;
            let template = UserDashboard::new(
                &mut Wa { db_conn },
                state.user_id.ok_or_else(|| {
                    SiteError::Internal(Some("missing userid in state".to_owned()))
                })?,
            )
            .await?;

            Ok(HttpResponse::Ok().body(
                template
                    .render()
                    .map_err(|e| SiteError::Internal(Some(e.to_string())))?,
            ))
        } else {
            return Err(SiteError::Internal(Some(
                "No database connection".to_owned(),
            )));
        }
    } else {
        // TODO: reroute to admin login instead
        Err(SiteError::Auth(
            "you need to be loged in to access the dashboard".to_owned(),
        ))
    }
}

struct Wa {
    db_conn: Conn,
}
