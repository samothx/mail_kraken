use crate::httpd::error::{SiteError, SiteResult};
use actix_identity::Identity;
use actix_web::{get, HttpResponse};
use askama::Template;
use log::debug;

#[derive(Template)]
#[template(path = "dash.html")]
struct UserDashboard {}

#[get("/dash")]
pub async fn user_dash(id: Identity) -> SiteResult {
    debug!("admin_dash: called with id: {:?}", id.identity());
    // debug_cookies("admin_dash:", &req);
    if let Some(_login) = id.identity() {
        let template = UserDashboard {};
        Ok(HttpResponse::Ok().body(
            template
                .render()
                .map_err(|e| SiteError::Internal(Some(e.to_string())))?,
        ))
    } else {
        // TODO: reroute to admin login instead
        Err(SiteError::Auth(
            "you need to be loged in to access the dashboard".to_owned(),
        ))
    }
}
