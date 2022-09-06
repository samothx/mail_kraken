mod config;
mod doveadm;
mod libc_util;
mod util;

mod cmd_args;
pub use cmd_args::{ImportArgs, ServeArgs};
mod import;
pub use import::import;
// mod httpd;
// pub use httpd::serve;

use crate::config::Config;
use crate::libc_util::{strerror, UserInfo};
use crate::util::switch_to_user;

const BCRYPT_COST: u32 = 8;
