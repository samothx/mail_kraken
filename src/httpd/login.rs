use crate::httpd::error::ErrorTemplate;
use crate::httpd::state_data::StateData;
use crate::httpd::ADMIN_NAME;
use actix_identity::Identity;
use actix_web::{get, http::StatusCode, post, web, HttpRequest, HttpResponse};
use askama::Template;
use log::{debug, error, warn};
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
pub async fn admin_login_form() -> HttpResponse {
    debug!("admin_login_form: ");
    let template = LoginTemplate {
        name_type: "text",
        default_name: "admin",
    };

    match template.render() {
        Ok(res) => HttpResponse::Ok()
            .content_type("text/html; charset=UTF-8")
            .body(res),
        Err(e) => ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[get("/login")]
pub async fn login_form(state: web::Data<StateData>) -> HttpResponse {
    debug!("login_form: called");
    match state.get_state() {
        Ok(state) => {
            debug!("login_form: for admin: {}", state.db_conn.is_none());
            let template = if state.db_conn.is_some() {
                let tmpl = LoginTemplate {
                    name_type: "email",
                    default_name: "",
                };
                tmpl.render()
            } else {
                let tmpl = LoginTemplate {
                    name_type: "text",
                    default_name: "admin",
                };
                tmpl.render()
            };

            match template {
                Ok(res) => HttpResponse::Ok()
                    .content_type("text/html; charset=UTF-8")
                    .body(res),
                Err(e) => {
                    ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            }
        }
        Err(e) => ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }

    // debug_cookies("login_form:", &req);
}

#[derive(Deserialize, Debug)]
pub struct Payload {
    login: String,
    passwd: String,
}

#[post("/api/v1/login")]
pub async fn login_handler(
    req: HttpRequest,
    state: web::Data<StateData>,
    payload: web::Json<Payload>,
    id: Identity,
) -> HttpResponse {
    debug!("login_handler: query: {:?}", req);
    // debug!("login_handler: payload: {:?}", payload);
    // debug_cookies("login_handler:", &req);
    debug!(
        "login_handler: called with id: {:?}, login: {}",
        id.identity(),
        payload.login
    );

    if payload.login.eq("admin") {
        match state.get_state() {
            Ok(state) => {
                debug!("got state");
                match state.config.is_admin_passwd(payload.passwd.as_str()) {
                    Ok(is_passwd) => {
                        if is_passwd {
                            id.remember(ADMIN_NAME.to_owned());
                            debug!(
                                "login_handler: login successful, id: {}",
                                id.identity().unwrap_or_else(|| "unknown".to_owned())
                            );
                            HttpResponse::Ok().body("empty")
                        } else {
                            id.forget();
                            warn!("login failure:");
                            HttpResponse::Unauthorized().body(())
                        }
                    }
                    Err(e) => {
                        error!("failed to check admin password: {:?}", e);
                        HttpResponse::InternalServerError().body(())
                    }
                }
            }
            Err(e) => HttpResponse::InternalServerError().body(()),
        }
    } else {
        id.forget();
        HttpResponse::InternalServerError().body(())
    }
}
