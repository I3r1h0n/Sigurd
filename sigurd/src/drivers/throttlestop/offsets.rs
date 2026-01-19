use std::ptr::null_mut;
use winapi::{
    shared::minwindef::DWORD, 
    um::{
        winnt::PVOID, 
        winreg::{
            HKEY_LOCAL_MACHINE, 
            RRF_RT_REG_SZ, 
            RegGetValueA
        }
    }
};

use crate::utils::error::SigurdError;

const EPROCESS_OFFSETS: [[DWORD; 3]; 5] = [
    [26100, 0x1d0, 0x1d8], // Change from 24H2
    [19041, 0x440, 0x448], // Change from 20H1
    [18362, 0x2e8, 0x2f0], // Change from 19H1
    [15063, 0x2e0, 0x2e8], // Offsets change starting from 1703
    [0,     0x2e8, 0x2f0], // Basic offsets beginning from 1507
];

fn get_system_build() -> DWORD {
    let mut value: [u8; 16] = [0; 16];
    let mut buffer_size: DWORD = 16;

    let status = unsafe {
        RegGetValueA(
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\0".as_ptr() as _,
            "CurrentBuild\0".as_ptr() as _,
            RRF_RT_REG_SZ,
            null_mut(),
            value.as_mut_ptr() as PVOID,
            &mut buffer_size,
        )
    };

    if status != 0 {
        return 0;
    }

    let null_pos = value.iter().position(|&c| c == 0).unwrap_or(value.len());
    let build_str = String::from_utf8_lossy(&value[..null_pos]);
    
    build_str.parse::<DWORD>().unwrap_or(0)
}

#[derive(Clone, Debug, Copy)]
pub struct EprocessOffsets {
    pub pid: DWORD,
    pub apl: DWORD
}

pub fn obtain_offsets() -> Result<EprocessOffsets, SigurdError> {
    let current_build = get_system_build();
    if current_build == 0 {
        return Err(SigurdError::default("Can't get CurrentBuild"));
    }

    for row in EPROCESS_OFFSETS.iter() {
        let build_number = row[0];
        if build_number > current_build {
            continue;
        }
        
        return Ok(EprocessOffsets{ pid: row[1], apl: row[2] });
    }

    let fallback_row = &EPROCESS_OFFSETS[3];
    return Ok(EprocessOffsets{ pid: fallback_row[1], apl: fallback_row[2] });
}