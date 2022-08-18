use anyhow::Result;
use nix::dir::Type::File;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub dd_url: String,
    pub admin_passwd: String,
}

impl Config {
    pub fn from_file(file_name: &str) -> Result<Config> {
        let cfg_str = fs::read_to_string(file_name)?;
        Ok(toml::from_str(cfg_str.as_str())?)
    }
}
