use crate::httpd::error::{ApiError, ApiResult, SiteError, SiteResult};
use crate::httpd::{StateData, ADMIN_NAME};
use crate::BCRYPT_COST;
use actix_identity::Identity;
use actix_web::web::Json;
use actix_web::{get, http::StatusCode, post, web, HttpResponse};
use anyhow::Context;
use askama::Template;
use log::debug;
use mysql_async::Pool;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "admin_dash.html")]
struct AdminDashboard {
    db_url: String,
}

#[derive(Debug, Deserialize)]
pub struct PayloadDbUrl {
    db_url: String,
}

#[post("/api/v1/admin/db_url")]
pub async fn admin_db_url(
    id: Identity,
    state: web::Data<StateData>,
    payload: web::Form<PayloadDbUrl>,
) -> ApiResult {
    debug!("admin_db_url: called with id: {:?}", id.identity());
    if id
        .identity()
        .unwrap_or_else(|| "noone".to_owned())
        .eq(ADMIN_NAME)
    {
        debug!("admin_db_url: payload: {:?}", payload);

        match state.get_mut_state() {
            Ok(mut state) => {
                match Pool::from_url(payload.db_url.as_str()) {
                    Ok(pool) => {
                        state.config.set_db_url(payload.db_url.as_str());
                        if let Some(db_conn) = state.db_conn.take() {
                            let _ = db_conn.disconnect();
                        }

                        match state.config.save().with_context(|| "failed to save config") {
                            Ok(_) => (),
                            Err(e) => {
                                return Err(ApiError::Internal(Some(e.to_string())));
                            }
                        }

                        state.db_conn = Some(pool);
                        // TODO: update & save config
                        Ok(HttpResponse::Ok().body(()))
                    }
                    Err(e) => Err(ApiError::Internal(Some(e.to_string()))),
                }
            }
            Err(e) => Err(ApiError::Internal(Some(e.to_string()))),
        }
    } else {
        Err(ApiError::Auth())
    }
}

#[derive(Deserialize, Debug)]
pub struct PasswdPayload {
    passwd: String,
    passwd_new: String,
}

enum PasswdRes {
    Ok(String),
    ErrUnauthorized(),
    ErrBadPasswd(),
    ErrInternal(),
}

#[post("/api/v1/passwd")]
pub async fn admin_passwd(
    state: web::Data<StateData>,
    id: Identity,
    payload: Json<PasswdPayload>,
) -> ApiResult {
    debug!("admin_dash: called with id: {:?}", id.identity());
    if id
        .identity()
        .unwrap_or_else(|| "noone".to_owned())
        .eq(ADMIN_NAME)
    {
        let pw_hash = {
            let state = state
                .get_state()
                .map_err(|e| ApiError::Internal(Some(e.to_string())))?;
            state.config.get_pw_hash()
        };

        let res = tokio::task::spawn_blocking(move || {
            match bcrypt::verify(payload.passwd.as_str(), pw_hash.as_str()) {
                Ok(is_admin) => {
                    if is_admin {
                        let mut pw_chars = payload.passwd_new.chars();

                        if (payload.passwd_new.len() >= 8)
                            && pw_chars.clone().any(|ch| ch.is_uppercase())
                            && pw_chars.clone().any(|ch| ch.is_lowercase())
                            && pw_chars.clone().any(|ch| ch.is_digit(10))
                            && pw_chars.any(|ch| !ch.is_alphanumeric())
                        {
                            match bcrypt::hash(payload.passwd_new.clone(), BCRYPT_COST) {
                                Ok(hash) => PasswdRes::Ok(hash),
                                Err(_) => PasswdRes::ErrInternal(),
                            }
                        } else {
                            PasswdRes::ErrBadPasswd()
                        }
                    } else {
                        PasswdRes::ErrUnauthorized()
                    }
                }
                Err(_) => PasswdRes::ErrInternal(),
            }
        })
        .await
        .map_err(|e| ApiError::Internal(Some(e.to_string())))?;
        match res {
            PasswdRes::Ok(pw_hash_new) => {
                let mut state = state
                    .get_mut_state()
                    .map_err(|e| ApiError::Internal(Some(e.to_string())))?;
                state.config.set_pw_hash(pw_hash_new);
                state
                    .config
                    .save()
                    .map_err(|e| ApiError::Internal(Some(e.to_string())))?;
                Ok(HttpResponse::Ok().body(()))
            }
            PasswdRes::ErrBadPasswd() => {
                Err(ApiError::Passwd("invalid old admin password".to_owned()))
            }
            PasswdRes::ErrInternal() => Err(ApiError::Internal(None)),
            PasswdRes::ErrUnauthorized() => Err(ApiError::Auth()),
        }
    } else {
        Err(ApiError::Auth())
    }
}

#[get("/admin_dash")]
pub async fn admin_dash(state: web::Data<StateData>, id: Identity) -> SiteResult {
    debug!("admin_dash: called with id: {:?}", id.identity());
    // debug_cookies("admin_dash:", &req);
    if id
        .identity()
        .unwrap_or_else(|| "noone".to_owned())
        .eq(ADMIN_NAME)
    {
        match state.get_state() {
            Ok(state) => {
                let default = String::from("user:passwd@host:port/database");
                let template = AdminDashboard {
                    db_url: state.config.get_db_url().unwrap_or(&default).to_owned(),
                };
                match template.render() {
                    Ok(res) => Ok(HttpResponse::Ok()
                        .content_type("text/html; charset=UTF-8")
                        .body(res)),
                    Err(e) => Err(SiteError::Internal(Some(e.to_string()))),
                }
            }
            Err(e) => Err(SiteError::Internal(Some(e.to_string()))),
        }
    } else {
        // TODO: reroute to admin login instead
        Err(SiteError::Auth(
            "only the admin user can access the admin dash".to_owned(),
        ))
    }
}
