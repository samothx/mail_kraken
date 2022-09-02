use anyhow::{anyhow, Context, Result};
use log::debug;

use nix::{
    errno::{errno, Errno::ERANGE},
    libc::{
        c_int, getpwnam_r, getpwuid_r, gid_t, passwd, stat as libc_stat, sysconf, uid_t,
        _SC_GETPW_R_SIZE_MAX,
    },
    unistd::{self},
};

use nix::libc::mode_t;
use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::path::Path;

mod libc_string;
use libc_string::LibCString;

pub struct UserInfo {
    info: passwd,
    #[allow(dead_code)]
    buffer: Vec<i8>,
}

impl UserInfo {
    #[allow(dead_code)]
    pub fn new() -> Result<UserInfo> {
        Self::from_id(unistd::getuid().as_raw())
    }

    pub fn from_name(name: &str) -> Result<UserInfo> {
        let c_name = CString::new(name)?;
        let mut pw_data: passwd = unsafe { MaybeUninit::zeroed().assume_init() };
        let mut bufsize = unsafe { sysconf(_SC_GETPW_R_SIZE_MAX) } as usize;
        let mut buf: Vec<i8> = vec![0; bufsize];
        let mut res: *mut passwd = std::ptr::null_mut();
        let mut rc;
        let mut repeat = false;

        loop {
            rc = unsafe {
                getpwnam_r(
                    c_name.as_ptr(),
                    &mut pw_data,
                    buf.as_mut_ptr(),
                    bufsize,
                    &mut res,
                )
            };

            if (rc == ERANGE as c_int) && !repeat {
                debug!(
                    "UserInfo::from_name({}): get getpwnam_r with bufsize {} returned {}",
                    name, bufsize, rc
                );
                repeat = true;
                bufsize *= 2;
                buf = vec![0; bufsize];
                debug!(
                    "UserInfo::from_name({}): retrying with bufsize: {}",
                    name, bufsize
                );

                continue;
            }
            break;
        }

        if res.is_null() {
            if rc == 0 {
                Err(anyhow!(format!(
                    "UserInfo::from_name({}): username not found",
                    name
                )))
            } else {
                Err(anyhow!(format!(
                    "UserInfo::from_name({}): getpwnam_r returned rc: {} : {}",
                    name,
                    rc,
                    strerror(rc).unwrap_or_else(|| "unknown".to_owned())
                )))
            }
        } else {
            assert_eq!((&mut pw_data) as *mut passwd, res);
            Ok(UserInfo {
                info: pw_data,
                buffer: buf,
            })
        }
    }

    pub fn from_id(id: uid_t) -> Result<UserInfo> {
        let mut pw_data: passwd = unsafe { MaybeUninit::zeroed().assume_init() };
        let mut bufsize = unsafe { sysconf(_SC_GETPW_R_SIZE_MAX) } as usize;
        let mut buf: Vec<i8> = vec![0; bufsize];
        let mut res: *mut passwd = std::ptr::null_mut();
        let mut rc;
        let mut repeat = false;

        loop {
            rc = unsafe { getpwuid_r(id, &mut pw_data, buf.as_mut_ptr(), bufsize, &mut res) };

            if (rc == ERANGE as c_int) && !repeat {
                debug!(
                    "UserInfo::from_id({}): get getpwnam_r with bufsize {} returned {}",
                    id, bufsize, rc
                );
                repeat = true;
                bufsize *= 2;
                buf = vec![0; bufsize];
                debug!(
                    "UserInfo::from_id({}): retrying with bufsize: {}",
                    id, bufsize
                );

                continue;
            }
            break;
        }

        if res.is_null() {
            if rc == 0 {
                Err(anyhow!(format!(
                    "UserInfo::from_id({}): user id not found",
                    id
                )))
            } else {
                Err(anyhow!(format!(
                    "UserInfo::from_id({}): getpwnuid_r returned rc: {} : {}",
                    id,
                    rc,
                    strerror(rc).unwrap_or_else(|| "unknown".to_owned())
                )))
            }
        } else {
            assert_eq!((&mut pw_data) as *mut passwd, res);
            Ok(UserInfo {
                info: pw_data,
                buffer: buf,
            })
        }
    }

    #[allow(dead_code)]
    pub fn get_name(&self) -> Result<String> {
        let c_name = unsafe { CStr::from_ptr(self.info.pw_name) };
        Ok(c_name
            .to_str()
            .context("UserInfo::get_name(): unable to read name")?
            .to_owned())
    }

    pub fn get_uid(&self) -> uid_t {
        self.info.pw_uid
    }

    pub fn get_gid(&self) -> gid_t {
        self.info.pw_gid
    }

    #[allow(dead_code)]
    pub fn is_root(&self) -> bool {
        self.info.pw_uid == 0
    }
}

pub fn strerror(rc: c_int) -> Option<String> {
    let err_ptr = unsafe { nix::libc::strerror(rc) };
    if err_ptr.is_null() {
        None
    } else {
        let err_str = unsafe { CStr::from_ptr(err_ptr) };
        match err_str.to_str() {
            Ok(msg) => Some(msg.to_owned()),
            Err(_) => None,
        }
    }
}

pub fn chown<P: AsRef<Path>>(path: P, uid: uid_t, gid: gid_t) -> Result<()> {
    let c_path = LibCString::try_from(path.as_ref())?;
    let rc = unsafe { nix::libc::chown(c_path.to_ptr(), uid, gid) };
    if rc == 0 {
        Ok(())
    } else {
        Err(anyhow!(
            strerror(errno().to_owned()).unwrap_or_else(|| "strerror failed".to_owned())
        ))
    }
}

pub fn chmod<P: AsRef<Path>>(path: P, mode: mode_t) -> Result<()> {
    let c_path = LibCString::try_from(path.as_ref())?;
    let rc = unsafe { nix::libc::chmod(c_path.to_ptr(), mode) };
    if rc == 0 {
        Ok(())
    } else {
        Err(anyhow!(
            strerror(errno().to_owned()).unwrap_or_else(|| "strerror failed".to_owned())
        ))
    }
}

#[allow(dead_code)]
pub fn stat<P: AsRef<Path>>(path: P) -> Result<libc_stat> {
    let c_path = LibCString::try_from(path.as_ref())?;
    let mut stat_info: libc_stat = unsafe { MaybeUninit::zeroed().assume_init() };
    let rc = unsafe { nix::libc::stat(c_path.to_ptr(), &mut stat_info) };
    if rc == 0 {
        Ok(stat_info)
    } else {
        Err(anyhow!(
            strerror(errno().to_owned()).unwrap_or_else(|| "strerror failed".to_owned())
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::unistd::getuid;

    #[test]
    fn test_user_info() {
        let test_file = "./config/test.yaml";
        let stat_info = stat(test_file).expect(format!("stat failed for {}", test_file).as_ref());
        match UserInfo::new() {
            Ok(user_info) => {
                // assert_eq!(user_info.get_name().unwrap(), "thomas".to_owned());
                assert_eq!(user_info.get_gid(), stat_info.st_gid);
                assert_eq!(user_info.get_uid(), stat_info.st_uid);
                assert!(!user_info.is_root());
            }
            Err(e) => {
                panic!("UserInfo::new() failed: {}", e);
            }
        };

        let user_name = "root";
        match UserInfo::from_name(user_name) {
            Ok(user_info) => {
                assert_eq!(user_info.get_name().unwrap(), "root".to_owned());
                assert_eq!(user_info.get_uid(), 0);
                assert!(user_info.is_root());
            }
            Err(e) => {
                panic!("UserInfo::from_name({}) failed: {}", user_name, e);
            }
        };

        let user_name = "_xyz";
        match UserInfo::from_name(user_name) {
            Ok(_) => {
                panic!("unexpected: UserInfo created for user {}", user_name);
            }
            Err(e) => {
                assert!(e.to_string().starts_with(
                    format!("UserInfo::from_name({}): username not found", user_name).as_str()
                ))
            }
        };
    }

    #[test]
    fn test_stat() {
        let path = Path::new("./config/test.yaml");
        match stat(path) {
            Ok(stat_info) => {
                assert_eq!(stat_info.st_uid, getuid().as_raw())
            }
            Err(e) => {
                panic!("stat({:?}) failed with error {}", path, e)
            }
        }
    }
}
