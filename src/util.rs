use anyhow::{anyhow, Context, Result};
use base64::{decode, encode};
use nix::errno::errno;

const SALT_LEN: usize = 16;
pub const SWITCH2USER: &str = "nobody"; // "mail_kraken";
const ROOT_UID: uid_t = 0;
const ROOT_GID: gid_t = 0;

use crate::{strerror, UserInfo};
use bcrypt::{hash_with_salt, Version, DEFAULT_COST};
use log::debug;
use nix::{
    libc::{gid_t, setresgid, setresuid, uid_t},
    unistd::{getgid, getuid},
};
use rand::{thread_rng, Rng};

pub fn switch_to_user(root: bool) -> Result<()> {
    let (username, dest_uid, dest_gid) = if root {
        ("root", ROOT_UID, ROOT_GID)
    } else {
        let user_info = UserInfo::from_name(SWITCH2USER)?;
        (SWITCH2USER, user_info.get_uid(), user_info.get_gid())
    };

    if getuid().as_raw() == dest_uid && getgid().as_raw() == dest_uid {
        return Ok(());
    }

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

pub fn make_salt() -> String {
    let mut admin_pw_salt = [0u8; SALT_LEN];
    thread_rng().fill(&mut admin_pw_salt[..]);
    encode(admin_pw_salt)
}

pub fn hash_passwd(passwd: &str, salt: &str) -> Result<String> {
    let salt_arr =
        decode(salt).with_context(|| format!("failed to decode base64 salt string {}", salt))?;
    assert_eq!(salt_arr.len(), SALT_LEN);
    let mut salt_cp = [0; SALT_LEN];
    salt_cp.iter_mut().zip(salt_arr).for_each(|(dest, src)| {
        *dest = src;
    });
    Ok(hash_with_salt(passwd, DEFAULT_COST, salt_cp)?.format_for_version(Version::TwoA))
}
