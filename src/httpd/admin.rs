use crate::httpd::error::ErrorTemplate;
use crate::httpd::{debug_cookies, SharedData, ADMIN_NAME};
use actix_identity::Identity;
use actix_web::{get, http::StatusCode, web, HttpRequest, HttpResponse};
use askama::Template;
use log::debug;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "admin_dashboard.html")]
struct AdminDashboard {
    db_url: String,
}

#[get("/admin_dash")]
pub async fn admin_dash(
    req: HttpRequest,
    state: web::Data<Arc<SharedData>>,
    id: Identity,
) -> HttpResponse {
    debug!("admin_dash: called with id: {:?}", id.identity());
    // debug_cookies("admin_dash:", &req);
    if id
        .identity()
        .unwrap_or_else(|| "noone".to_owned())
        .eq(ADMIN_NAME)
    {
        let template = AdminDashboard {
            db_url: state
                .config
                .db_url
                .clone()
                .unwrap_or_else(|| "user:passwd@host:port/database".to_owned()),
        };
        match template.render() {
            Ok(res) => HttpResponse::Ok()
                .content_type("text/html; charset=UTF-8")
                .body(res),
            Err(e) => ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    } else {
        HttpResponse::Unauthorized().body(())
    }
}
