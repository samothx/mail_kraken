use crate::libc_util::{chmod, chown};
use crate::util::SWITCH2USER;
use crate::{switch_to_user, UserInfo, BCRYPT_COST};
use anyhow::{Context, Result};
use bcrypt::hash;
use log::debug;
use nix::unistd::getuid;
use serde::{Deserialize, Serialize};
use std::fs;

pub const CONFIG_FILE: &str = "/etc/mail_kraken.cfg";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    db_url: Option<String>,
    admin_pw_hash: String,
    bind_to: String,
}

impl Config {
    pub fn new(db_url: Option<String>, admin_pw: String, bind_to: String) -> Result<Config> {
        Ok(Config {
            db_url,
            admin_pw_hash: hash(admin_pw.as_str(), BCRYPT_COST)?,
            bind_to,
        })
    }

    pub fn from_file() -> Result<Config> {
        debug!("attempting to read config from {}", CONFIG_FILE);
        let cfg_str = fs::read_to_string(CONFIG_FILE)?;
        Ok(toml::from_str(cfg_str.as_str())?)
    }

    pub fn set_pw_hash(&mut self, pw_hash: String) {
        self.admin_pw_hash = pw_hash;
    }

    pub fn get_pw_hash(&self) -> String {
        self.admin_pw_hash.clone()
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
