use anyhow::{anyhow, Result};
use nix::errno::errno;

pub const SWITCH2USER: &str = "nobody"; // "mail_kraken";
const ROOT_UID: uid_t = 0;
const ROOT_GID: gid_t = 0;

use crate::{strerror, UserInfo};
use log::debug;
use nix::{
    libc::{gid_t, setresgid, setresuid, uid_t},
    unistd::{getgid, getuid},
};

pub fn switch_to_user(root: bool) -> Result<()> {
    let (username, dest_uid, dest_gid) = if root {
        ("root", ROOT_UID, ROOT_GID)
    } else {
        let user_info = UserInfo::from_name(SWITCH2USER)?;
        (SWITCH2USER, user_info.get_uid(), user_info.get_gid())
    };

    // TODO: this check does not appear to be working - fix
    /*    if getuid().as_raw() == dest_uid && getgid().as_raw() == dest_uid {
            debug!("switch_to_user: already {}", username);
            return Ok(());
        }
    */
    match unsafe { setresgid(0xFFFFFFFF, dest_gid, ROOT_GID) } {
        0 => debug!("setresgid success"),
        _ => {
            return Err(anyhow!(
                "failed to setegid to {}: {:?}",
                dest_gid,
                strerror(errno()).unwrap_or_else(|| "unknown".to_owned())
            ))
        }
    }

    match unsafe { setresuid(0xFFFFFFFF, dest_uid, ROOT_UID) } {
        0 => debug!("setresuid success"),
        _ => {
            return Err(anyhow!(
                "failed to seteuid to {} {}: {:?}",
                username,
                dest_uid,
                strerror(errno()).unwrap_or_else(|| "unknown".to_owned())
            ))
        }
    }

    Ok(())
}
