use crate::libc_util::{chmod, chown};
use crate::util::{hash_passwd, make_salt, SWITCH2USER};
use crate::{switch_to_user, UserInfo};
use anyhow::{Context, Result};
use log::debug;
use nix::unistd::getuid;
use serde::{Deserialize, Serialize};
use std::fs;

pub const CONFIG_FILE: &str = "/etc/mail_kraken.cfg";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    db_url: Option<String>,
    admin_pw_hash: String,
    admin_pw_salt: String,
    bind_to: String,
}

impl Config {
    pub fn new(db_url: Option<String>, admin_pw: String, bind_to: String) -> Result<Config> {
        let admin_pw_salt = make_salt();
        Ok(Config {
            db_url,
            admin_pw_hash: hash_passwd(admin_pw.as_str(), admin_pw_salt.as_str())?,
            admin_pw_salt,
            bind_to,
        })
    }

    pub fn from_file() -> Result<Config> {
        debug!("attempting to read config from {}", CONFIG_FILE);
        let cfg_str = fs::read_to_string(CONFIG_FILE)?;
        Ok(toml::from_str(cfg_str.as_str())?)
    }

    pub fn is_admin_passwd(&self, passwd: &str) -> Result<bool> {
        debug!("is_admin_passwd:");
        debug!(
            "is_admin_passwd: comparing hashes: \n{}\n{}",
            self.admin_pw_hash,
            hash_passwd(passwd, &self.admin_pw_salt).expect("failed to hash password")
        );
        Ok(hash_passwd(passwd, &self.admin_pw_salt)?.eq(&self.admin_pw_hash))
    }

    pub fn get_db_url(&self) -> Option<&String> {
        self.db_url.as_ref()
    }

    pub fn set_db_url(&mut self, db_url: &str) {
        self.db_url = Some(db_url.to_owned())
    }

    pub fn get_bind_to(&self) -> &str {
        self.bind_to.as_str()
    }

    pub fn save(&self) -> Result<()> {
        let toml_str =
            toml::to_string(self).with_context(|| "failed to serialize config".to_owned())?;

        let switchchback = if getuid().is_root() {
            false
        } else {
            switch_to_user(true).with_context(|| "failed to switch to user root".to_owned())?;
            true
        };

        let res = self.save_int(toml_str.as_str());

        if switchchback {
            switch_to_user(false)
                .with_context(|| "failed to switch to mail_kraken user".to_owned())?;
        }
        res
    }

    fn save_int(&self, toml: &str) -> Result<()> {
        fs::write(CONFIG_FILE, toml)
            .with_context(|| format!("failed to write config to {}", CONFIG_FILE))?;

        let user_info = UserInfo::from_name(SWITCH2USER)?;
        chown(CONFIG_FILE, user_info.get_uid(), user_info.get_gid())?;
        chmod(CONFIG_FILE, 0x660)?;
        Ok(())
    }
}
