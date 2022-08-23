use crate::httpd::admin::PasswdRes::ErrBadPasswd;
use crate::httpd::error::{ApiError, ApiResult, SiteError, SiteResult};
use crate::httpd::{StateData, ADMIN_NAME};
use crate::BCRYPT_COST;
use actix_identity::Identity;
use actix_web::web::{Bytes, Json};
use actix_web::{get, http::StatusCode, post, web, HttpMessage, HttpRequest, HttpResponse};
use anyhow::Context;
use askama::Template;
use log::{debug, info};
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
    ErrBadPasswd(String),
    ErrInternal(),
}

#[post("/api/v1/passwd")]
pub async fn admin_passwd(
    // bytes: Bytes,
    state: web::Data<StateData>,
    id: Identity,
    payload: Json<PasswdPayload>,
) -> ApiResult {
    debug!("admin_passwd: called with id: {:?}", id.identity());

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

                        if (payload.passwd_new.len() < 8) {
                            ErrBadPasswd("new password is too short".to_owned())
                        } else if !pw_chars.clone().any(|ch| ch.is_uppercase()) {
                            ErrBadPasswd(
                                "new password contains no upper case characters ".to_owned(),
                            )
                        } else if !pw_chars.clone().any(|ch| ch.is_lowercase()) {
                            ErrBadPasswd(
                                "new password contains no lower case characters ".to_owned(),
                            )
                        } else if !pw_chars.clone().any(|ch| ch.is_digit(10)) {
                            ErrBadPasswd("new password contains no digit characters ".to_owned())
                        } else if !pw_chars.any(|ch| !ch.is_alphanumeric()) {
                            ErrBadPasswd("new password contains no special characters ".to_owned())
                        } else {
                            match bcrypt::hash(payload.passwd_new.clone(), BCRYPT_COST) {
                                Ok(hash) => PasswdRes::Ok(hash),
                                Err(_) => PasswdRes::ErrInternal(),
                            }
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
            PasswdRes::ErrBadPasswd(msg) => Err(ApiError::BadRequest(Some(format!(
                "new password is not complex enough: {}",
                msg
            )))),
            PasswdRes::ErrInternal() => Err(ApiError::Internal(None)),
            PasswdRes::ErrUnauthorized() => {
                Err(ApiError::Passwd("invalid old admin password".to_owned()))
            }
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
