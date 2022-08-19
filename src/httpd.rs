use crate::{Config, ServeCmd};
use anyhow::{Context, Result};
use bcrypt::{hash_with_salt, Version, DEFAULT_COST};
use rand::{thread_rng, Rng};
use std::sync::Arc;

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

use crate::httpd::admin::admin_dash;
use crate::httpd::login::{admin_login_form, login_form, login_handler};

#[derive(Debug)]
pub struct SharedData {
    config: Config,
    db_conn: Option<Pool>,
}

pub async fn serve(args: ServeCmd, config: Option<Config>) -> Result<()> {
    debug!("serve: entered");
    let config = if let Some(config) = config {
        config
    } else {
        let mut admin_pw_salt = vec![0u8; 16];
        thread_rng().fill(&mut admin_pw_salt[..]);

        Config {
            db_url: None,
            admin_pw_hash: hash_passwd(args.init_passwd.as_str(), &admin_pw_salt)
                .with_context(|| "failed to hash default password")?,
            admin_pw_salt,
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
            .service(ActixFiles::new("/assets", ".").show_files_listing())
            .service(admin_login_form)
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
fn hash_passwd(passwd: &str, salt: &[u8]) -> Result<String> {
    assert!(salt.len() >= 16);
    let mut salt_cp = [0u8; 16];
    salt_cp.iter_mut().zip(salt).for_each(|(dest, src)| {
        *dest = *src;
    });
    Ok(hash_with_salt(passwd, DEFAULT_COST, salt_cp)?.format_for_version(Version::TwoA))
}
