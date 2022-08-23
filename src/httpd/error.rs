use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use anyhow::Error;
use std::fmt::{Debug, Display, Formatter};

pub type SiteResult = std::result::Result<HttpResponse, SiteError>;

pub enum SiteError {
    Auth(String),
    Internal(Option<String>),
    Redirect(String),
}

impl Debug for SiteError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SiteError::Auth(msg) => write!(f, "authentification error: {}", msg),
            SiteError::Internal(msg) => write!(
                f,
                "internal error: {}",
                if let Some(msg) = msg {
                    msg.as_str()
                } else {
                    ""
                }
            ),
            SiteError::Redirect(dest) => write!(f, "redirection: {}", dest),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: Error) -> Self {
        ApiError::Internal(Some(err.to_string()))
    }
}

impl Display for SiteError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SiteError::Auth(msg) => write!(f, "authentification error: {}", msg),
            SiteError::Internal(msg) => write!(
                f,
                "internal error: {}",
                if let Some(msg) = msg {
                    msg.as_str()
                } else {
                    ""
                }
            ),
            SiteError::Redirect(dest) => write!(f, "redirection: {}", dest),
        }
    }
}

impl actix_web::ResponseError for SiteError {
    fn status_code(&self) -> StatusCode {
        match self {
            SiteError::Auth(_) => StatusCode::UNAUTHORIZED,
            SiteError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SiteError::Redirect(_) => StatusCode::SEE_OTHER,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            SiteError::Auth(msg) => HttpResponse::Unauthorized().body(msg.to_owned()),
            SiteError::Internal(msg) => {
                if let Some(msg) = msg {
                    HttpResponse::InternalServerError().body(msg.to_owned())
                } else {
                    HttpResponse::InternalServerError().body(())
                }
            }
            SiteError::Redirect(location) => HttpResponse::SeeOther()
                .insert_header(("Location", location.to_owned()))
                .body(()),
        }
    }
}

pub type ApiResult = std::result::Result<HttpResponse, ApiError>;

pub enum ApiError {
    Passwd(String),
    Auth(),
    Internal(Option<String>),
    Redirect(String),
    NotImpl(),
    BadRequest(Option<String>),
}

impl Debug for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::BadRequest(msg) => write!(
                f,
                "error: bad request, {}",
                msg.as_ref().unwrap_or(&"".to_owned())
            ),
            ApiError::NotImpl() => write!(f, "error: not implemented"),
            ApiError::Passwd(msg) => write!(f, "error: password authentification, {}", msg),
            ApiError::Auth() => write!(f, "error: unauthorized"),
            ApiError::Internal(msg) => write!(
                f,
                "error: internal, {}",
                msg.as_ref().unwrap_or(&"".to_owned())
            ),

            ApiError::Redirect(dest) => write!(f, "redirection to {}", dest),
        }
    }
}

impl From<anyhow::Error> for SiteError {
    fn from(err: Error) -> Self {
        SiteError::Internal(Some(err.to_string()))
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::BadRequest(msg) => write!(
                f,
                "error: bad request, {}",
                msg.as_ref().unwrap_or(&"".to_owned())
            ),
            ApiError::NotImpl() => write!(f, "error: not implemented"),
            ApiError::Passwd(msg) => write!(f, "error: password verificaion,  {}", msg),
            ApiError::Auth() => write!(f, "error: unauthorized",),
            ApiError::Internal(msg) => {
                write!(
                    f,
                    "error: internal,  {}",
                    msg.as_ref().unwrap_or(&"".to_owned())
                )
            }
            ApiError::Redirect(dest) => write!(f, "error: redirection to {}", dest),
        }
    }
}

impl actix_web::ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotImpl() => StatusCode::NOT_IMPLEMENTED,
            ApiError::Passwd(_) => StatusCode::UNAUTHORIZED,
            ApiError::Auth() => StatusCode::UNAUTHORIZED,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Redirect(_) => StatusCode::SEE_OTHER,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            ApiError::BadRequest(msg) => {
                if let Some(msg) = msg {
                    HttpResponse::BadRequest().body(msg.to_owned())
                } else {
                    HttpResponse::BadRequest().body(())
                }
            }
            ApiError::NotImpl() => HttpResponse::NotImplemented().body(()),
            ApiError::Passwd(msg) => HttpResponse::Unauthorized().body(msg.to_owned()),
            ApiError::Auth() => HttpResponse::Unauthorized().body(()),
            ApiError::Internal(msg) => {
                if let Some(msg) = msg {
                    HttpResponse::InternalServerError().body(msg.to_owned())
                } else {
                    HttpResponse::InternalServerError().body(())
                }
            }
            ApiError::Redirect(location) => HttpResponse::SeeOther()
                .insert_header(("Location", location.to_owned()))
                .body(()),
        }
    }
}
