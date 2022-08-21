use crate::{Config, ServeCmd};
use anyhow::{Context, Result};
use rand::Rng;

use mysql_async::Pool;

use actix_files::Files as ActixFiles;
use actix_identity::{CookieIdentityPolicy, IdentityService};

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};

use log::{debug, error};
use std::net::SocketAddr;

const ADMIN_NAME: &str = "admin";

mod admin;
mod error;
mod login;
mod state_data;

use crate::httpd::admin::{admin_dash, admin_db_url, admin_passwd};
use crate::httpd::login::{admin_login_form, login_form, login_handler};
use state_data::{SharedData, StateData};

pub async fn serve(args: ServeCmd, config: Option<Config>) -> Result<()> {
    debug!("serve: entered");
    let config = if let Some(config) = config {
        config
    } else {
        Config::new(None, args.init_passwd, args.bind_to)?
    };

    let pool = if let Some(db_url) = config.get_db_url() {
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
        .get_bind_to()
        .parse::<SocketAddr>()
        .with_context(|| format!("unable to parse IP address {}", config.get_bind_to()))?;

    let shared_data = StateData::new(SharedData {
        db_conn: pool,
        config,
    });

    let private_key = rand::thread_rng().gen::<[u8; 32]>();

    HttpServer::new(move || {
        let data = shared_data.clone();
        App::new()
            .app_data(web::Data::new(data))
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&private_key)
                    .name("mail-kraken")
                    .secure(false),
            ))
            .route("/", web::get().to(HttpResponse::Ok))
            .service(ActixFiles::new("/assets", "."))
            .service(admin_login_form)
            .service(admin_passwd)
            .service(admin_db_url)
            .service(login_form)
            .service(login_handler)
            .service(admin_dash)
    })
    .bind(ip_addr)
    .with_context(|| "failed to bind to ip address".to_owned())?
    .run()
    .await
    .with_context(|| "failed to serve http content")
}

#[allow(dead_code)]
fn debug_cookies(hdr: &str, req: &HttpRequest) {
    debug!("{hdr}: cookies:");
    req.cookies().iter().enumerate().for_each(|(idx, cookie)| {
        debug!(" - {:02}: {:?}", idx, cookie);
    });
}
