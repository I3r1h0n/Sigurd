use std::time::Duration;
use std::{mem, thread};
use std::ptr::null_mut;
use winapi::um::winsvc::{DeleteService, SC_MANAGER_ALL_ACCESS, SC_MANAGER_CREATE_SERVICE, SERVICE_STOP};
use winapi::um::{
    winnt::{SERVICE_AUTO_START, SERVICE_ERROR_NORMAL, SERVICE_KERNEL_DRIVER}, 
    winsvc::{
        CloseServiceHandle, ControlService, CreateServiceW, OpenSCManagerW, OpenServiceW, QueryServiceStatus, StartServiceW,
        SERVICE_ALL_ACCESS, SERVICE_CONTROL_STOP, SERVICE_QUERY_STATUS, 
        SERVICE_STATUS
    }
};

use crate::utils::error::SigurdError;
use crate::utils::to_wstring;
use crate::warn;

#[allow(non_camel_case_types)]
#[derive(PartialEq, Debug)]
pub enum ServiceState {
    CONTINUE_PENDING,
    PAUSE_PENDING,
    PAUSED,
    RUNNING,
    START_PENDING,
    STOP_PENDING,
    STOPPED,
    ERROR
}

impl From<u32> for ServiceState {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::STOPPED,
            2 => Self::START_PENDING,
            3 => Self::STOP_PENDING,
            4 => Self::RUNNING,
            5 => Self::CONTINUE_PENDING,
            6 => Self::PAUSE_PENDING,
            7 => Self::PAUSED,
            _ => Self::ERROR,
        }
    }
}

/// Returns true if service status is RUNNING
pub fn sc_status(driver_name: &str) -> Result<bool, SigurdError> {
    unsafe {
        let sc_manager = OpenSCManagerW(null_mut(), null_mut(), 0x0001);
        if sc_manager.is_null() { 
            return Err(SigurdError::last("Can't get sc manager"));
        }
        
        let service = OpenServiceW(sc_manager, to_wstring(driver_name).as_ptr(), SERVICE_QUERY_STATUS);
        if service.is_null() {
            CloseServiceHandle(sc_manager);
            return Err(SigurdError::last("Can't open service"));
        }
        
        let mut status: SERVICE_STATUS = mem::zeroed();
        let result = QueryServiceStatus(service, &mut status) != 0;

        CloseServiceHandle(service);
        CloseServiceHandle(sc_manager);

        if result {
            match ServiceState::from(status.dwCurrentState) {
                ServiceState::RUNNING => {
                    return Ok(true);
                }
                _ => {
                    return Ok(false);
                }
            }
        } else {
            return Err(SigurdError::default("Wrong service status"));
        }
    }
}

/// Creates and starts service
/// Will return true is service was created, and false it existed 
pub fn sc_create(driver_name: &str, driver_path: &str) -> Result<bool, SigurdError> {
    unsafe {
        // Will remain true if service was created by us
        let mut created = true;

        let sc_manager = OpenSCManagerW(null_mut(), null_mut(), SC_MANAGER_CREATE_SERVICE);
        if sc_manager.is_null() { 
            return Err(SigurdError::last("Can't get sc manager"));
        }

        let mut service = OpenServiceW(sc_manager, to_wstring(driver_name).as_ptr(), SERVICE_QUERY_STATUS);
        service = if service.is_null() {
            CreateServiceW(
                sc_manager, 
                to_wstring(&driver_name).as_ptr(),
                to_wstring(&driver_name).as_ptr(),
                SERVICE_ALL_ACCESS, 
                SERVICE_KERNEL_DRIVER, 
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                to_wstring(&driver_path).as_ptr(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        } else {
            warn!("Service already exists");
            if sc_status(&driver_name)? {
                return Ok(false);
            } else {
                created = false;
            }
            service
        };

        if service.is_null() {
            CloseServiceHandle(sc_manager);
            return Err(SigurdError::last("Unable to create service"));
        } 

        let result = StartServiceW(
            service, 
            0, 
            null_mut()
        ) != 0;

        CloseServiceHandle(service);
        CloseServiceHandle(sc_manager);

        if result {
            return Ok(created);
        } else {
            return Err(SigurdError::last("Unable to start service"));
        }
    }
}

/// Stops a service
pub fn sc_stop(driver_name: &str) -> Result<(), SigurdError> {
    unsafe {
        let sc_manager = OpenSCManagerW(null_mut(), null_mut(), SC_MANAGER_ALL_ACCESS);
        if sc_manager.is_null() { 
            return Err(SigurdError::last( "Can't get sc manager"));
        }
        
        let service = OpenServiceW(
            sc_manager, 
            to_wstring(driver_name).as_ptr(), 
            SERVICE_STOP | SERVICE_QUERY_STATUS
        );
        
        if service.is_null() {
            CloseServiceHandle(sc_manager);
            return Err(SigurdError::last("Service not found"));
        }
        
        let mut status: SERVICE_STATUS = mem::zeroed();
        if QueryServiceStatus(service, &mut status) == 0 {
            CloseServiceHandle(service);
            CloseServiceHandle(sc_manager);
            return Err(SigurdError::last("Can't get service status"));
        }
        
        let current_state = ServiceState::from(status.dwCurrentState);
        
        match current_state {
            ServiceState::RUNNING | ServiceState::START_PENDING | ServiceState::PAUSED | ServiceState::PAUSE_PENDING => {
                let result = ControlService(
                        service, 
                        SERVICE_CONTROL_STOP, 
                        &mut status
                );
                
                if result == 0 {
                    CloseServiceHandle(service);
                    CloseServiceHandle(sc_manager);
                    return Err(SigurdError::last("Failed to stop service"));
                }
                
                for _ in 0..30 {
                    thread::sleep(Duration::from_secs(1));
                    
                    let mut new_status: SERVICE_STATUS = mem::zeroed();
                    if QueryServiceStatus(service, &mut new_status) == 0 {
                        break;
                    }
                    
                    if ServiceState::from(new_status.dwCurrentState) == ServiceState::STOPPED {
                        break;
                    }
                }
                
                CloseServiceHandle(service);
                CloseServiceHandle(sc_manager);
                Ok(())
            }
            ServiceState::STOPPED => {
                CloseServiceHandle(service);
                CloseServiceHandle(sc_manager);
                warn!("Service already stopped"); // Replace with warning
                Ok(())
            }
            ServiceState::STOP_PENDING => {
                for _ in 0..30 {
                    thread::sleep(Duration::from_secs(1));
                    
                    let mut new_status: SERVICE_STATUS = mem::zeroed();
                    if QueryServiceStatus(service, &mut new_status) == 0 {
                        break;
                    }
                    
                    if ServiceState::from(new_status.dwCurrentState) == ServiceState::STOPPED {
                        break;
                    }
                }
                
                CloseServiceHandle(service);
                CloseServiceHandle(sc_manager);
                Ok(())
            }
            _ => {
                CloseServiceHandle(service);
                CloseServiceHandle(sc_manager);
                Err(SigurdError::default(&format!("Service in unexpected state: {:?}", current_state)))
            }
        }
    }
}

/// Deletes a service
pub fn sc_delete(driver_name: &str) -> Result<(), SigurdError> {
    unsafe {
        let sc_manager = OpenSCManagerW(null_mut(), null_mut(), 0x0001);
        if sc_manager.is_null() { 
            return Err(SigurdError::last("Can't get sc manager"));
        }
        
        let service = OpenServiceW(
            sc_manager, 
            to_wstring(driver_name).as_ptr(), 
            0x00010000 | SERVICE_QUERY_STATUS  // DELETE access
        );
        
        if service.is_null() {
            CloseServiceHandle(sc_manager);
            return Err(SigurdError::last("Service not found"));
        }
        
        let mut status: SERVICE_STATUS = mem::zeroed();
        if QueryServiceStatus(service, &mut status) != 0 {
            let current_state = ServiceState::from(status.dwCurrentState);
            
            if current_state != ServiceState::STOPPED {
                CloseServiceHandle(service);
                CloseServiceHandle(sc_manager);
                return Err(SigurdError::default("Cannot delete service: Service is not stopped"));
            }
        } else {
            warn!("Could not query service status before deletion"); // Replace with warn
        }
        
        let result = DeleteService(service);
        
        CloseServiceHandle(service);
        CloseServiceHandle(sc_manager);
        
        if result != 0 {
            Ok(())
        } else {
            Err(SigurdError::last( "Failed to delete service"))
        }
    }
}
