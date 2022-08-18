use crate::{Config, ServeCmd};
use anyhow::{anyhow, Result};
use askama::Template;
use rand::Rng;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::{
    extract::{Extension, Query},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::{
    cookie::{Cookie, Key, PrivateCookieJar},
    Form,
};

use axum_sessions::{
    async_session::MemoryStore,
    extractors::{ReadableSession, WritableSession},
    SessionLayer,
};

use serde::Deserialize;

use mysql_async::{prelude::*, Pool};

// use axum_extra::extract::cookie::PrivateCookieJar;

use axum::response::{Html, IntoResponse, Response};
use log::{debug, error};
use nix::errno::errno;
use std::collections::btree_map::Values;

type WebResult<T> = Result<T, axum::http::StatusCode>;

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    status: &'static str,
    message: String,
}

impl ErrorTemplate {
    pub fn to_response(
        status: StatusCode,
        message: String,
    ) -> axum::http::Result<Response<Html<String>>> {
        let template = ErrorTemplate {
            status: status.as_str(),
            message,
        };

        match template.render() {
            Ok(res) => Response::builder().status(status).body(Html::from(res)),
            Err(e) => Err(anyhow!("failed to render template")),
        }
    }
}

struct SharedData {
    config: Option<Config>,
    db_conn: Option<Pool>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

async fn login_form() -> axum::http::Result<Response<Html<String>>> {
    let template = LoginTemplate {};
    match template.render() {
        Ok(res) => Response::builder()
            .status(StatusCode::OK)
            .body(Html::from(res)),
        Err(e) => ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(Deserialize, Debug)]
struct Payload {
    #[serde(rename = "login-name")]
    name: String,
    passwd: String,
}

#[derive(Template)]
#[template(path = "admin_dashboard.html")]
struct AdminDashboard {}

async fn login_handler(
    Form(payload): Form<Payload>,
    mut session: WritableSession,
    Extension(state): Extension<Arc<SharedData>>,
) -> axum::http::Result<Response> {
    if let Some(user) = session.get::<String>("user") {
        session.destroy();
    }

    debug!("payload: {:?}", payload);
    if payload.name.eq("admin") {
        if let Some(config) = state.config.as_ref() {
            if payload.passwd.eq(config.admin_passwd.as_str()) {
                session
                    .insert("user", "admin")
                    .expect("failed to insert user into session");
                let template = AdminDashboard {};
                match template.render() {
                    Ok(res) => Response::builder()
                        .status(StatusCode::OK)
                        .body(Html::from(res)),

                    Err(e) => ErrorTemplate::to_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to create template for AdminDashboard".to_owned(),
                    ),
                }
            } else {
                ErrorTemplate::to_response(
                    StatusCode::UNAUTHORIZED,
                    "please supply a valid password for admin".to_owned(),
                )
            }
        } else {
            ErrorTemplate::t_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "no config received - please supply a password as parameter".to_owned(),
            )
        }
    } else {
        ErrorTemplate::to_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "not implemented: please login as admin with password".to_owned(),
        )
    }
}

pub async fn serve(args: ServeCmd, mut config: Option<Config>) -> Result<()> {
    let pool = if let Some(config) = config.as_mut() {
        if config.admin_passwd.is_empty() {
            if !args.init_passwd.is_empty() {
                config.admin_passwd = args.init_passwd;
            } else {
                error!("no admin password given");
                return Err(anyhow!(
                    "no admin password given, please specify init password on the command line"
                ));
            }
        }
        Some(Pool::from_url(config.dd_url.as_str())?)
    } else {
        None
    };

    let shared_data = Arc::new(SharedData {
        db_conn: pool,
        config,
    });

    let store = MemoryStore::new();
    let mut secret = [0u8; 128];
    rand::thread_rng().fill(&mut secret);
    debug!("secret: {:?}", secret);
    let session_layer = SessionLayer::new(store, &secret);

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/login", get(login_form))
        .route("/api/v1/login", post(login_handler))
        .layer(Extension(Arc::new(shared_data)))
        .layer(session_layer);

    Ok(axum::Server::bind(&args.bind_to.parse()?)
        .serve(app.into_make_service())
        .await?)
}
