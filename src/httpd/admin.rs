use crate::httpd::error::ErrorTemplate;
use crate::httpd::{debug_cookies, SharedData, ADMIN_NAME};
use actix_identity::Identity;
use actix_web::{get, http::StatusCode, web, HttpRequest, HttpResponse};
use askama::Template;
use log::debug;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "admin_dashboard.html")]
struct AdminDashboard {}

#[get("/admin_dash")]
pub async fn admin_dash(
    req: HttpRequest,
    _state: web::Data<Arc<SharedData>>,
    id: Identity,
) -> HttpResponse {
    debug!("admin_dash: called with id: {:?}", id.identity());
    // debug_cookies("admin_dash:", &req);
    let is_admin = id
        .identity()
        .unwrap_or_else(|| "noone".to_owned())
        .eq(ADMIN_NAME);
    /*match session.get::<u32>(SESS_ADMIN) {
        Ok(admin) => admin.is_some(),
        Err(e) => {
            error!("failed to extract {} from session: {:?}", SESS_ADMIN, e);
            return ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    };
     */
    debug!("admin_dash: admin_login: {}", is_admin);
    let template = AdminDashboard {};
    match template.render() {
        Ok(res) => HttpResponse::Ok()
            .content_type("text/html; charset=UTF-8")
            .body(res),
        Err(e) => ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}
