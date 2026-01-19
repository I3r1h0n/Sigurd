use core::fmt;
use std::{
    ptr, 
    ffi::OsString,
    os::windows::ffi::OsStringExt
};
use winapi::{
    shared::minwindef::DWORD,
    um::{
        errhandlingapi::GetLastError,
        winbase::{
            FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS,
            FormatMessageW, LocalFree
        }
    }
};

#[derive(Clone, Debug)]
pub struct SigurdError {
    pub code: u32,
    pub msg: String,
    pub last_error: Option<String>
}

impl SigurdError {
    pub fn default(msg: &str) -> Self {
        return SigurdError { code: 1, msg: msg.to_string(), last_error: None };
    }

    pub fn last(msg: &str) -> Self {
        return SigurdError { 
            code: unsafe { GetLastError() }, 
            msg: msg.to_string(),
            last_error: Some(GetLastErrorString())
        }
    }
}

impl fmt::Display for SigurdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref last) = self.last_error {
            write!(f, "Error code {}: {} (last_error: {})", self.code, self.msg, last)
        } else {
            write!(f, "{}", self.msg)
        }
    }
}

impl From<superfetch::error::SpfError> for SigurdError {
    fn from(e: superfetch::error::SpfError) -> Self {
        return Self::default(&format!("Superfetch error: {}", e));
    }
}

impl From<std::io::Error> for SigurdError {
    fn from(e: std::io::Error) -> Self {
        return Self::default(&format!("IO Error: {}", e));
    }
}

impl From<toml::ser::Error> for SigurdError {
    fn from(e: toml::ser::Error) -> Self {
        return Self::default(&format!("Can't serialize to string: {}", e));
    }
}

impl From<serde_json::Error> for SigurdError {
    fn from(e: serde_json::Error) -> Self {
        return Self::default(&format!("Can't serialize to string: {}", e));
    }
}

pub fn GetLastErrorString() -> String {
    unsafe {
        let error_code: DWORD = GetLastError();
        
        let mut buffer: *mut u16 = ptr::null_mut();
        
        let chars = FormatMessageW(
            FORMAT_MESSAGE_ALLOCATE_BUFFER | 
            FORMAT_MESSAGE_FROM_SYSTEM | 
            FORMAT_MESSAGE_IGNORE_INSERTS,
            ptr::null(),
            error_code,
            0,
            &mut buffer as *mut *mut u16 as *mut u16,
            0,
            ptr::null_mut()
        );
        
        if chars == 0 {
            return format!("Failed to format error message: {}", error_code);
        }
        
        let slice = std::slice::from_raw_parts(buffer, chars as usize);
        let os_string = OsString::from_wide(slice);
        let result = os_string.to_string_lossy().trim_end_matches("\r\n").to_string();
        
        LocalFree(buffer as *mut _);
        
        result
    }
}
