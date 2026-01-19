use clap::Parser;
use std::{
    path::Path, 
    time::Duration,
    sync::{Arc, atomic::{AtomicBool, Ordering}}
};

use crate::{
    cli::tui_start, 
    drivers::get_drivers, 
    killer::Killer, 
    utils::{config::Config, is_elevated}
};

#[allow(non_snake_case)]
pub mod utils;
pub mod killer;
pub mod drivers;
#[macro_use]
pub mod cli;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to .toml config file
    #[arg(short, long, conflicts_with = "config_string")]
    config: Option<String>,
    
    /// TOML configuration as a quoted string
    #[arg(long = "config-string", conflicts_with = "config")]
    config_string: Option<String>,
    
    /// Run app without interface
    #[arg(short, long, action)]
    silent: bool,
}

fn main() {
    // Check if we are admin
    if !is_elevated() {
        error!("Sigurd requires admin rights to run!");
        return;
    }

    // Parse CLI
    let cli = Cli::parse();

    // Search for config
    let config = if let Some(ref config_path) = cli.config {
        Some(Config::from_file(config_path))
    } else if let Some(ref config_str) = cli.config_string {
        Some(Config::from_json_str(config_str))
    } else {
        if Path::new("./Config.toml").exists() {
            Some(Config::from_file("./Config.toml"))
        } else {
            None
        }
    };

    // Get drivers
    let drivers = match get_drivers() {
        Ok(d) => d,
        Err(e) => {
            error!("Can't load driver options: {}", e);
            return;
        }
    };

    // Get driver names
    let mut driver_names: Vec<String> = Vec::new();
    for driver in &drivers {
        driver_names.push(driver.name().to_string());
    }

    // Prepare running config
    let running_config: Config;

    if cli.silent {
        if config.is_some() {
            info!("Starting without terminal user interface");
            trace!("Current config: {:?}", config);
            match config.unwrap() {
                Ok(c) => {
                    running_config = c;
                }
                Err(e) => {
                    error!("{}", e);
                    return;
                }
            }
        } else {
            error!("Need to provide config to start in silent mode");
            return;
        }
    } else {
        let tui_arg = match config {
            Some(r) => {
                match r {
                    Ok(c) => {
                        Some(c)
                    },
                    Err(e) => {
                        error!("{}", e);
                        return;
                    }
                }
            }
            None => None
        };
        
        match tui_start(tui_arg, driver_names.clone()) {
            Ok(c) => {
                running_config = c;
            }
            Err(e) => {
                error!("{}", e);
                return;
            }
        };

        trace!("Avaliable drivers: {:?}", driver_names);
    }

    // Create the killer
    let mut killer = match Killer::new(running_config.clone(), drivers) {
        Ok(k) => k,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };
    
    // Install killer
    match killer.install() {
        Ok(_) => {
            success!("Killer installed");
        },
        Err(e) => {
            error!("{}", e);
            return;
        }
    }

    // Initialize the killer
    match killer.init() {
        Ok(_) => {
            success!("Killer initialized");
        },
        Err(e) => {
            error!("{}", e);
            return;
        }
    }

    // Run killer loop
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        warn!("Received Ctrl+C! Shutting down...");
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl+C handler");

    while running.load(Ordering::SeqCst) {
        match killer.kill() {
            Ok(_) => {},
            Err(e) => {
                error!("{}", e);
                break;
            }
        }

        if !running_config.continuous {
            break;
        } else {
            std::thread::sleep(Duration::from_secs(5));
        }
    }

    // Descruct
    info!("Killer destruction..");
    match killer.destruct() {
        Ok(_) => {
            success!("Killer destructed.");
        },
        Err(e) => {
            error!("{}", e);
        }
    }
    
    // Uninstall (if configured)
    if running_config.uninstall {
        info!("Uninstalling..");
        match killer.uninstall() {
            Ok(_) => {
                success!("Killer uninstalled");
            },
            Err(e) => {
                error!("{}", e);
            }
        }
    }

    success!("Finished");
}
