use actix_web::{http::StatusCode, HttpResponse};
use askama::Template;

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    status: String,
    message: String,
}

impl ErrorTemplate {
    pub fn to_response(status: StatusCode, message: String) -> HttpResponse {
        let template = ErrorTemplate {
            status: status.as_str().to_owned(),
            message,
        };

        match template.render() {
            Ok(res) => HttpResponse::Ok()
                .content_type("text/html; charset=UTF-8")
                .body(res), // (StatusCode::INTERNAL_SERVER_ERROR, BoxBody::from(res))
            Err(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
