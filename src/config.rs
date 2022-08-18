use anyhow::Result;
use nix::dir::Type::File;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub db_url: Option<String>,
    pub admin_passwd: String,
    pub bind_to: String,
}

impl Config {
    pub fn from_file(file_name: &str) -> Result<Config> {
        let cfg_str = fs::read_to_string(file_name)?;
        Ok(toml::from_str(cfg_str.as_str())?)
    }
}
