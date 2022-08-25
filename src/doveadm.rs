const MB_SIZE: usize = 1024 * 1024;
const DOVEADM_CMD: &str = "doveadm";

// pub use cmd_args::{CmdArgs, ServeCmd, Command};

pub use fetch::params::{FetchParams, ImapField, SearchParam};

mod auth;
mod fetch;

pub use auth::authenticate;
