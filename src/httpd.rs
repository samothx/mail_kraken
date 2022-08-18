use crate::{Config, ServeCmd};
use anyhow::{anyhow, Context, Result};
use askama::Template;
use rand::Rng;
use std::sync::Arc;

use serde::Deserialize;

use mysql_async::{prelude::*, Pool};

use actix_files;
use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_web::{
    body::BoxBody, cookie::Key, get, http::StatusCode, web, App, HttpResponse, HttpServer,
};

use log::{debug, error};
use nix::errno::errno;
use std::collections::btree_map::Values;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
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
            Ok(res) => HttpResponse::Ok().body(res), // (StatusCode::INTERNAL_SERVER_ERROR, BoxBody::from(res))
            Err(e) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

#[get("/login")]
async fn login_form() -> HttpResponse {
    let template = LoginTemplate {};
    match template.render() {
        Ok(res) => HttpResponse::Ok().body(res),
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
#[get("/api/v1/login")]
async fn login_handler(
    state: web::Data<Arc<SharedData>>,
    payload: web::Form<Payload>,
    session: Session,
) -> HttpResponse {
    let _ = session.remove("user");
    debug!("payload: {:?}", payload);
    if payload.name.eq("admin") {
        if payload.passwd.eq(state.config.admin_passwd.as_str()) {
            session
                .insert("user", "admin")
                .expect("failed to insert user into session");
            let template = AdminDashboard {};
            match template.render() {
                Ok(res) => HttpResponse::Ok().body(res),
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
        ErrorTemplate::to_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "not implemented: please login as admin with password".to_owned(),
        )
    }
}

#[derive(Debug)]
struct SharedData {
    config: Config,
    db_conn: Option<Pool>,
}

pub async fn serve(args: ServeCmd, config: Option<Config>) -> Result<()> {
    let config = if let Some(config) = config {
        config
    } else {
        Config {
            db_url: None,
            admin_passwd: args.init_passwd,
            bind_to: args.bind_to,
        }
    };

    let pool = if let Some(db_url) = config.db_url.as_ref() {
        Some(Pool::from_url(db_url.as_str())?)
    } else {
        None
    };

    let ip_addr = config
        .bind_to
        .parse::<SocketAddr>()
        .with_context(|| format!("unable to parse IP address {}", config.bind_to.as_str()))?;

    let shared_data = Arc::new(SharedData {
        db_conn: pool,
        config,
    });

    let secret_key = Key::generate();

    HttpServer::new(move || {
        let data = shared_data.clone();
        App::new()
            .app_data(data)
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .route("/", web::get().to(HttpResponse::Ok))
            .service(actix_files::Files::new("/assets", ".").show_files_listing())
            .service(login_form)
            .service(login_handler)
    })
    .bind(ip_addr)
    .with_context(|| "failed to bind to ip address".to_owned())?
    .run()
    .await
    .with_context(|| "failed to serve http content")
}
