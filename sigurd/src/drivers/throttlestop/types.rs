#![allow(unused)]
use winapi::shared::ntdef::PVOID;

#[repr(C)]
pub struct RTL_PROCESS_MODULE_INFORMATION {
    section: PVOID,
    mapped_base: PVOID,
    image_base: PVOID,
    image_size: u32,
    flags: u32,
    load_order_index: u16,
    init_order_index: u16,
    load_count: u16,
    offset_to_file_name: u16,
    full_path_name: [u8; 256],
}

#[repr(C)]
pub struct RTL_PROCESS_MODULES {
    number_of_modules: u32,
    modules: [RTL_PROCESS_MODULE_INFORMATION; 1],
}
