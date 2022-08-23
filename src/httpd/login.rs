use crate::httpd::error::{ApiError, ApiResult, SiteError, SiteResult};
use crate::httpd::state_data::StateData;
use crate::httpd::ADMIN_NAME;
use actix_identity::Identity;
use actix_web::{get, post, web, HttpResponse};
use askama::Template;
use log::{debug, warn};
use serde::Deserialize;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    name_type: &'a str,
    default_name: &'a str,
}

/*#[derive(Template)]
#[template(path = "admin_login.html")]
struct AdminLoginTemplate {}
*/

#[get("/admin_login")]
pub async fn admin_login_form() -> SiteResult {
    debug!("admin_login_form: ");
    let template = LoginTemplate {
        name_type: "text",
        default_name: "admin",
    };

    match template.render() {
        Ok(res) => Ok(HttpResponse::Ok()
            .content_type("text/html; charset=UTF-8")
            .body(res)),
        Err(e) => Err(SiteError::Internal(Some(e.to_string()))),
    }
}

#[get("/login")]
pub async fn login_form(state: web::Data<StateData>) -> SiteResult {
    debug!("login_form: called");
    let db_initialized = {
        let state = state
            .get_state()
            .map_err(|e| SiteError::Internal(Some(e.to_string())))?;
        state.db_conn.is_some()
    };

    debug!("login_form: for admin: {}", !db_initialized);
    let template = if db_initialized {
        LoginTemplate {
            name_type: "email",
            default_name: "",
        }
    } else {
        LoginTemplate {
            name_type: "text",
            default_name: "admin",
        }
    };

    match template.render() {
        Ok(res) => Ok(HttpResponse::Ok()
            .content_type("text/html; charset=UTF-8")
            .body(res)),
        Err(e) => Err(SiteError::Internal(Some(e.to_string()))),
    }
}

#[derive(Deserialize, Debug)]
pub struct Payload {
    login: String,
    passwd: String,
}

#[post("/api/v1/login")]
pub async fn login_handler(
    // req: HttpRequest,
    state: web::Data<StateData>,
    payload: web::Json<Payload>,
    id: Identity,
) -> ApiResult {
    debug!(
        "login_handler: called with id: {:?}, login: {}",
        id.identity(),
        payload.login
    );

    let pw_hash = {
        let state = state
            .get_state()
            .map_err(|e| ApiError::Internal(Some(e.to_string())))?;
        state.config.get_pw_hash()
    };

    if payload.login.eq("admin") {
        let passwd_valid = tokio::task::spawn_blocking(move || {
            bcrypt::verify(payload.passwd.as_str(), pw_hash.as_str())
        })
        .await
        .map_err(|e| ApiError::Internal(Some(e.to_string())))?
        .map_err(|e| ApiError::Internal(Some(e.to_string())))?;

        if passwd_valid {
            id.remember(ADMIN_NAME.to_owned());
            debug!(
                "login_handler: login successful, id: {}",
                id.identity().unwrap_or_else(|| "unknown".to_owned())
            );
            Ok(HttpResponse::Ok().body(()))
        } else {
            id.forget();
            warn!("login failure:");
            Err(ApiError::Passwd("invalid password".to_string()))
        }
    } else {
        Err(ApiError::NotImpl())
    }
}
