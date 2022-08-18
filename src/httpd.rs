use crate::{Config, ServeCmd};
use anyhow::{anyhow, Context, Result};
use askama::Template;
use bcrypt::{hash, DEFAULT_COST};
use rand::Rng;
use serde::Deserialize;
use std::sync::Arc;

use mysql_async::{prelude::*, Pool};

use actix_files;
// use actix_http::http::header::ContentType;
use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_web::{
    body::BoxBody, cookie::Key, get, http::StatusCode, post, web, App, HttpResponse, HttpServer,
};

use log::{debug, error};
use nix::libc::passwd;
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
            Ok(res) => HttpResponse::Ok()
                .content_type("text/html; charset=UTF-8")
                .body(res), // (StatusCode::INTERNAL_SERVER_ERROR, BoxBody::from(res))
            Err(e) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

#[derive(Template)]
#[template(path = "admin_login.html")]
struct AdminLoginTemplate {}

#[get("/admin_login")]
async fn admin_login_form() -> HttpResponse {
    debug!("admin_login_form: ");
    let template = LoginTemplate {};

    match template.render() {
        Ok(res) => HttpResponse::Ok()
            .content_type("text/html; charset=UTF-8")
            .body(res),
        Err(e) => ErrorTemplate::to_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[get("/login")]
async fn login_form(state: web::Data<Arc<SharedData>>) -> HttpResponse {
    debug!("login_form: admin_login: {}", state.db_conn.is_none());
    let template = if state.db_conn.is_some() {
        let tmpl = LoginTemplate {};
        tmpl.render()
    } else {
        let tmpl = AdminLoginTemplate {};
        tmpl.render()
    };

    match template {
        Ok(res) => HttpResponse::Ok()
            .content_type("text/html; charset=UTF-8")
            .body(res),
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
#[post("/api/v1/login")]
async fn login_handler(
    state: web::Data<Arc<SharedData>>,
    payload: web::Form<Payload>,
    session: Session,
) -> HttpResponse {
    let _ = session.remove("user");
    debug!("login_handler: payload: {:?}", payload);
    if payload.name.eq("admin") {
        let pw_hash = match hash_passwd(payload.passwd.as_str()) {
            Ok(pw_hash) => pw_hash,
            Err(e) => {
                error!("failed to hash admin password: {:?}", e);
                return ErrorTemplate::to_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to create hash admin password".to_owned(),
                );
            }
        };
        if pw_hash.eq(state.config.admin_passwd.as_str()) {
            session
                .insert("user", "admin")
                .expect("failed to insert user into session");
            let template = AdminDashboard {};
            match template.render() {
                Ok(res) => HttpResponse::Ok()
                    .content_type("text/html; charset=UTF-8")
                    .body(res),
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
            admin_passwd: hash_passwd(args.init_passwd.as_str())
                .with_context(|| "failed to hash default password")?,
            bind_to: args.bind_to,
        }
    };

    let pool = if let Some(db_url) = config.db_url.as_ref() {
        match Pool::from_url(db_url.as_str()) {
            Ok(pool) => Some(pool),
            Err(e) => {
                error!("failed to log in to database: {:?}", e);
                None
            }
        }
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
            .app_data(web::Data::new(data))
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .route("/", web::get().to(HttpResponse::Ok))
            .service(actix_files::Files::new("/assets", ".").show_files_listing())
            .service(admin_login_form)
            .service(login_form)
            .service(login_handler)
    })
    .bind(ip_addr)
    .with_context(|| "failed to bind to ip address".to_owned())?
    .run()
    .await
    .with_context(|| "failed to serve http content")
}

fn hash_passwd(passwd: &str) -> Result<String> {
    Ok(hash(passwd, DEFAULT_COST)?)
}
