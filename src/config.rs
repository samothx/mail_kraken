use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub db_url: Option<String>,
    pub admin_pw_hash: String,
    pub admin_pw_salt: Vec<u8>,
    pub bind_to: String,
}

impl Config {
    pub fn from_file(file_name: &str) -> Result<Config> {
        let cfg_str = fs::read_to_string(file_name)?;
        Ok(toml::from_str(cfg_str.as_str())?)
    }
}
