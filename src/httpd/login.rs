use crate::httpd::error::ErrorTemplate;
use crate::httpd::{SharedData, ADMIN_NAME};
use actix_identity::Identity;
use actix_web::{get, http::StatusCode, post, web, HttpRequest, HttpResponse};
use askama::Template;
use log::{debug, error, warn};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

#[derive(Template)]
#[template(path = "admin_login.html")]
struct AdminLoginTemplate {}

#[get("/admin_login")]
pub async fn admin_login_form() -> HttpResponse {
    debug!("admin_login_form: ");
    let template = LoginTemplate {};

    match template.render() {
        Ok(res) => HttpResponse::Ok()
            .content_type("text/html; charset=UTF-8")
            .body(res),
        Err(e) => ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[get("/login")]
pub async fn login_form(state: web::Data<Arc<SharedData>>) -> HttpResponse {
    debug!("login_form: admin_login: {}", state.db_conn.is_none());
    // debug_cookies("login_form:", &req);
    let template = if state.db_conn.is_some() {
        let tmpl = LoginTemplate {};
        tmpl.render()
    } else {
        let tmpl = AdminLoginTemplate {};
        tmpl.render()
    };

    match template {
        Ok(res) => HttpResponse::Ok()
            .content_type("text/html; charset=UTF-8")
            .body(res),
        Err(e) => ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(Deserialize, Debug)]
pub struct Payload {
    #[serde(rename = "login-name")]
    name: String,
    passwd: String,
}

#[post("/api/v1/login")]
pub async fn login_handler(
    req: HttpRequest,
    state: web::Data<Arc<SharedData>>,
    payload: web::Form<Payload>,
    id: Identity,
) -> HttpResponse {
    debug!("login_handler: query: {:?}", req.query_string(),);
    debug!("login_handler: payload: {:?}", payload);
    // debug_cookies("login_handler:", &req);
    debug!("login_handler: called with id: {:?}", id.identity());
    if payload.name.eq("admin") {
        match state.config.is_admin_passwd(payload.passwd.as_str()) {
            Ok(is_passwd) => {
                if is_passwd {
                    id.remember(ADMIN_NAME.to_owned());
                    debug!(
                        "login_handler: login successful, id: {}",
                        id.identity().unwrap_or_else(|| "unknown".to_owned())
                    );
                    HttpResponse::SeeOther()
                        .insert_header(("Location", "/admin_dash"))
                        // .cookie(session.)
                        .body(())
                } else {
                    id.forget();

                    warn!("login failure:");
                    ErrorTemplate::to_response(
                        StatusCode::UNAUTHORIZED,
                        "please supply a valid password for admin".to_owned(),
                    )
                }
            }
            Err(e) => {
                error!("failed to check admin password: {:?}", e);
                ErrorTemplate::to_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to check admin password".to_owned(),
                )
            }
        }
    } else {
        id.forget();
        ErrorTemplate::to_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "not implemented: please login as admin with password".to_owned(),
        )
    }
}
