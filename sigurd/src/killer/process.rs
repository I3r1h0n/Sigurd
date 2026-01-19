use std::{ffi::CStr, mem};
use winapi::um::{
    handleapi::{CloseHandle, INVALID_HANDLE_VALUE}, 
    tlhelp32::{
        CreateToolhelp32Snapshot, 
        PROCESSENTRY32, 
        Process32First, 
        Process32Next,
        TH32CS_SNAPPROCESS
    }
};

use crate::utils::error::SigurdError;


pub fn is_running(name: &str) -> Result<bool, SigurdError> {
    match get_pid(name) {
        Ok(_pid) => return Ok(true),
        Err(e) => {
            if e.code == 1 {
                return Ok(false);
            } else {
                return Err(e);
            }
        }
    }
}

pub fn get_pid(name: &str) -> Result<u32, SigurdError> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE { 
            return Err(SigurdError::last("Can't get process snapshot")); 
        }
        
        let mut entry: PROCESSENTRY32 = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32>() as u32;
        
        if Process32First(snap, &mut entry) != 0 {
            loop {
                let current = CStr::from_ptr(entry.szExeFile.as_ptr()).to_string_lossy();
                if current.to_lowercase() == name.to_lowercase() {
                    return Ok(entry.th32ProcessID);
                }
                if Process32Next(snap, &mut entry) == 0 { break; }
            }
        }
        CloseHandle(snap);
        return Err(SigurdError::default(&format!("Can't find process: {}", name))); 
    }
}
