use anyhow::{anyhow, Result};
use log::{debug, info};
use mod_logger::Logger;
use nix::errno::errno;
use nix::libc::{setegid, seteuid};
use nix::unistd::getuid;

mod cmd_args;
mod libc_util;
use crate::cmd_args::{Command, ServeCmd};
pub use cmd_args::CmdArgs;

mod doveadm;

use crate::libc_util::{strerror, UserInfo};

const SWITCH2USER: &str = "nobody"; // "mail_kraken";

pub async fn run(cmd_args: CmdArgs) -> Result<()> {
    // TODO: probably should not do this as su
    Logger::set_default_level(cmd_args.log_level);
    Logger::set_color(true);
    Logger::set_brief_info(true);

    info!("initializing");
    if !getuid().is_root() {
        return Err(anyhow!("please run this command as root"));
    }

    {
        debug!("switching uid/gid to {}", SWITCH2USER);

        let user_info = UserInfo::from_name(SWITCH2USER)?;
        
        match unsafe { setegid(user_info.get_gid()) } {
            0 => debug!("setegid OK"),
            _ => {
                return Err(anyhow!(
                    "failed to setegid to {}: {:?}",
                    user_info.get_gid(),
                    strerror(errno()).unwrap_or("unknown".to_owned())
                ))
            }
        }
        
        match unsafe { seteuid(user_info.get_uid()) } {
            0 => debug!("seteuid OK"),
            _ => {
                return Err(anyhow!(
                    "failed to seteuid to {} {}: {:?}",
                    SWITCH2USER,
                    user_info.get_uid(),
                    strerror(errno()).unwrap_or("unknown".to_owned())
                ))
            }
        }


    };

    match cmd_args.cmd {
        Command::Serve(args) => serve(args).await,
    }
}

async fn serve(_args: ServeCmd) -> Result<()> {
    todo!()
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
