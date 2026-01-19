use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::utils::error::SigurdError;

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Config {
    pub driver_name: String,
    pub installation_path: String,
    pub victim_processes: Vec<String>,
    pub continuous: bool,
    pub uninstall: bool,
}

impl Config {
    pub fn from_toml_str(toml_str: &str) -> Result<Self, SigurdError> {
        let config: Config = match toml::from_str(toml_str) {
            Ok(c) => c,
            Err(e) => {
                return Err(SigurdError::default(&format!("Invalid config: {}", e)));
            }
        };
        Ok(config)
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, SigurdError> {
        let config: Config = match serde_json::from_str(json_str) {
            Ok(c) => c,
            Err(e) => {
                return Err(SigurdError::default(&format!("Invalid config: {}", e)));
            }
        };
        Ok(config)
    }

    pub fn to_json_string(&self) -> Result<String, SigurdError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn to_toml_string(&self) -> Result<String, SigurdError> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn from_file(path: &str) -> Result<Self, SigurdError> {
        if !Path::new(path).extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("toml"))
            .unwrap_or(false) {
            return Err(SigurdError::default("Config file must have .toml extension"));
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return Err(SigurdError::default(&format!("Can't read file: {:?}", e)));
            }
        };
        
        Self::from_toml_str(&content)
    }
}
