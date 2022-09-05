use anyhow::{anyhow, Context, Result};
use log::{error, info};
use mod_logger::Logger;
use nix::unistd::getuid;

mod cmd_args;
mod doveadm;
mod httpd;
pub use cmd_args::{CmdArgs, ImportArgs};
mod config;
mod db;
mod libc_util;
mod util;

use crate::cmd_args::{Command, ServeCmd};
use crate::config::Config;
use crate::httpd::serve;
use crate::libc_util::{strerror, UserInfo};
use crate::util::switch_to_user;

const BCRYPT_COST: u32 = 8;

pub fn import(args: ImportArgs) -> Result<()> {
    todo!()
}

pub async fn run(cmd_args: CmdArgs) -> Result<()> {
    // TODO: probably should not do this as su
    Logger::set_default_level(cmd_args.log_level);
    Logger::set_color(true);
    Logger::set_brief_info(true);

    info!("initializing - cmd: {:?}", cmd_args.cmd);

    let config = match Config::from_file().await {
        Ok(config) => Some(config),
        Err(e) => {
            error!("failed to read config file: {}", e);
            None
        }
    };

    if !getuid().is_root() {
        return Err(anyhow!("please run this command as root"));
    }

    switch_to_user(false).with_context(|| "failed to switch user".to_owned())?;

    match cmd_args.cmd {
        Command::Serve(args) => serve(args, config).await,
    }
}

/*
fn fetch() -> Result<()> {
    Logger::set_default_level(cmd_args.log_level);
    Logger::set_color(true);
    Logger::set_brief_info(true);


    // TODO: set userid to nobody
    //

    let mut fetch_params = FetchParams::new(cmd_args.user);

    fetch_params
        .add_search_param(SearchParam::Mailbox("INBOX".to_owned()))
        .add_search_param(SearchParam::Seen);

    cmd_args.fields.iter().for_each(|field| {
        let _ = fetch_params.add_field(field.clone());
    });

    info!("fetch: calling doveadm with parameters {:?}", fetch_params);
    let mut doveadm = DoveadmFetch::new(fetch_params)?;
    while let Some(record) = doveadm.parse_record()? {
        info!("fetch: Got: \n {:?}", record);
    }
    todo!()
}
*/

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
