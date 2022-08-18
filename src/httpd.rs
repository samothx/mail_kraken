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

use log::error;
use nix::errno::errno;
use std::collections::btree_map::Values;

type WebResult<T> = Result<T, axum::http::StatusCode>;

struct SharedData {
    config: Option<Config>,
    db_conn: Option<Pool>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

async fn login_form() -> WebResult<String> {
    Ok(LoginTemplate {}
        .render()
        .map_err(|e| StatusCode::INTERNAL_SERVER_ERROR)?)
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "login-name")]
    name: String,
    passwd: String,
}

async fn login_handler(
    Form(payload): Form<Payload>,
    mut session: WritableSession,
) -> WebResult<String> {
    if let Some(user) = session.get::<String>("user") {
        session.destroy();
    }

    if payload.name.eq("admin") {
        todo!()
    }
    todo!()
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

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/login", get(login_form))
        .route("/api/v1/login", post(login_handler))
        .layer(Extension(Arc::new(shared_data)));

    Ok(axum::Server::bind(&args.bind_to.parse()?)
        .serve(app.into_make_service())
        .await?)
}
