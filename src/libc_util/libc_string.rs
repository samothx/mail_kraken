use anyhow::Context;
use nix::libc::c_char;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub struct LibCString(CString);

impl TryFrom<&Path> for LibCString {
    type Error = anyhow::Error;
    fn try_from(value: &Path) -> std::result::Result<Self, Self::Error> {
        Ok(LibCString(
            CString::new(value.as_os_str().as_bytes())
                .with_context(|| format!("failed to create CString from path {:?}", value))?,
        ))
    }
}

impl TryFrom<&str> for LibCString {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Ok(LibCString(CString::new(value.as_bytes()).with_context(
            || format!("failed to create CString from &str {:?}", value),
        )?))
    }
}

impl TryFrom<String> for LibCString {
    type Error = anyhow::Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Ok(LibCString(CString::new(value.as_bytes()).with_context(
            || format!("failed to create CString from String {:?}", value),
        )?))
    }
}

impl LibCString {
    #[allow(dead_code)]
    pub fn to_ptr(&self) -> *const c_char {
        self.0.as_ptr()
    }
}
