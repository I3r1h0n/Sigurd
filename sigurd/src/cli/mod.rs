use std::fs;

use crate::{Config, cli::tui::Tui, utils::error::SigurdError};

pub mod log;
pub mod tui;

pub fn tui_start(config: Option<Config>, driver_names: Vec<String>) -> Result<Config, SigurdError> {
    let mut tui = Tui::new()?;
    tui.print_header()?;

    let result = match config {
        Some(c) => {
            tui.println("Config loaded!")?;
            let patch = tui.get_yes_no("Do you want to update current config?")?;

            if patch {
                tui.clear_content()?;

                hand_patch(&mut tui, c, driver_names)?
            } else {
                c
            }
        }
        None => {
            tui.println("No config found! Hand config mode")?;
            let cfg: Config = hand_config(&mut tui, driver_names)?;

            let save_to_disk = tui.get_yes_no("Wanna save config to disk?")?;
            if save_to_disk {
                let mut save_path = tui.get_input("Enter path to save(default ./Config.toml):")?;
                if save_path == "" {
                    save_path = String::from("./Config.toml");
                } 

                let config_string = cfg.to_toml_string()?;
                fs::write(save_path, config_string)?;

                cfg
            } else {
                cfg
            }
        }
    };

    if tui.get_yes_no("Start?")? {
        return Ok(result);
    } else {
        std::process::exit(0);
    }
}

pub fn hand_patch(tui: &mut Tui, config: Config, driver_names: Vec<String>) -> Result<Config, SigurdError> {
    let mut patched_config = config;

    if tui.get_yes_no(&format!("Current driver is {}. Change?", patched_config.driver_name))? {
        patched_config.driver_name = {
            let num = tui.select_from_list("Choose an driver to use:", &driver_names)?;
            driver_names[num.unwrap()].clone()
        };
    }
    
    if tui.get_yes_no(&format!("Installation path: {}. Change?", patched_config.installation_path))? {
        patched_config.installation_path = {
            let mut ipath = tui.get_input("Enter installation path(default C:\\ProgramData):")?;
            if ipath == "" {
                ipath = "C:\\ProgramData".to_string();
            }
            ipath
        };
    }

    loop {
        tui.println(&format!("Current victims: {:?}", patched_config.victim_processes))?;
        let new_v = tui.get_input("Enter new victim(to finish leave empty):")?;
        if new_v == "" {
            tui.clear_content()?;
            break;
        } else {
            patched_config.victim_processes.push(new_v);
            tui.clear_content()?;
        }
    }

    patched_config.continuous = {
        tui.get_yes_no("Run continuous?")?
    };

    patched_config.uninstall = {
        tui.get_yes_no("Uninstall on finish?")?
    };

    tui.clear_content()?;

    return Ok(patched_config);
}


pub fn hand_config(tui: &mut Tui, driver_names: Vec<String>) -> Result<Config, SigurdError> {
    let driver_name = {
        let num = tui.select_from_list("Choose an driver to use:", &driver_names)?;
        driver_names[num.unwrap()].clone()
    };

    let installation_path = {
        let mut ipath = tui.get_input("Enter installation path(default C:\\ProgramData):")?;
        if ipath == "" {
            ipath = "C:\\ProgramData".to_string();
        }
        ipath
    };

    let victim_processes = {
        let mut vp_vec: Vec<String> = Vec::new();

        loop {
            tui.println(&format!("Current victims: {:?}", vp_vec))?;
            let new_v = tui.get_input("Enter new victim(to finish leave empty):")?;
            if new_v == "" {
                tui.clear_content()?;
                break;
            } else {
                vp_vec.push(new_v);
                tui.clear_content()?;
            }
        }

        vp_vec
    };

    let continuous = {
        tui.get_yes_no("Run continuous?")?
    };

    let uninstall = {
        tui.get_yes_no("Uninstall on finish?")?
    };

    tui.clear_content()?;

    return Ok(Config { driver_name, installation_path, victim_processes, continuous, uninstall });
}
