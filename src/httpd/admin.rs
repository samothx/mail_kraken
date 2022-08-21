use crate::httpd::error::ErrorTemplate;
use crate::httpd::{StateData, ADMIN_NAME};
use actix_identity::Identity;
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
) -> HttpResponse {
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
                                return ErrorTemplate::to_response(
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    e.to_string(),
                                );
                            }
                        }

                        state.db_conn = Some(pool);
                        // TODO: update & save config
                        HttpResponse::Ok().body(())
                    }
                    Err(e) => {
                        ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                    }
                }

                /*                HttpResponse::SeeOther()
                                   .insert_header(("Location", "/admin_dash"))
                                   .body(())

                */
            }
            Err(e) => ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    } else {
        HttpResponse::Unauthorized().body(())
    }
}

#[post("/api/v1/admin/passwd")]
pub async fn admin_passwd() -> HttpResponse {
    todo!()
}

#[get("/admin_dash")]
pub async fn admin_dash(state: web::Data<StateData>, id: Identity) -> HttpResponse {
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
    } else {
        HttpResponse::Unauthorized().body(())
    }
}
