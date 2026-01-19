use std::{
    fs,
    path::PathBuf,
    {ffi::OsStr, path::Path},
    os::windows::ffi::OsStrExt
};

use rand::distr::{Alphanumeric, Distribution};
use winapi::um::fileapi::SetFileAttributesW;

use crate::utils::error::SigurdError;

pub fn path_drop_filename(s: &str) -> &str {
    match s.rfind(|c| c == '/' || c == '\\') {
        Some(idx) if idx > 0 => &s[..idx],
        Some(_) => &s[..1],
        None => "",
    }
}

pub fn hidden_storage(target_folder: &str) -> Result<PathBuf, SigurdError> {    
    let rng = rand::rng();
    let random_name: String = Alphanumeric
        .sample_iter(rng)
        .take(12)
        .map(char::from)
        .collect();
    
    let folder_name = format!(".{}", random_name);
    let mut path = PathBuf::from(target_folder);
    path.push(folder_name);
    
    fs::create_dir_all(&path)?;
    
    let wide_path: Vec<u16> = OsStr::new(&path)
        .encode_wide()
        .chain(Some(0))
        .collect();
    
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
    
    unsafe {
        if SetFileAttributesW(wide_path.as_ptr(), FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) == 0 {
            return Err(SigurdError::last("Can't set file attributes"));
        }
    }
    
    Ok(path.canonicalize()?)
}

pub fn find_storage(filename: &str, target_folder: &str) -> Result<PathBuf, SigurdError> {
    let program_data = Path::new(target_folder);
    
    if !program_data.exists() {
        return Err(SigurdError::default("ProgramData directory not found"));
    }
    
    let entries = fs::read_dir(program_data)
        .map_err(|e| SigurdError::default(&format!("Failed to read ProgramData: {}", e)))?;
    
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue, 
        };
        
        let path = entry.path();
        
        if path.is_dir() {
            let target_file = path.join(filename);
            if target_file.exists() {
                return Ok(path);
            }
        }
    }
    
    Err(SigurdError::default(&format!("File '{}' not found in any ProgramData subdirectory", filename)))
}
