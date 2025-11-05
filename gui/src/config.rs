// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) Qualcomm Technologies, Inc. and/or its subsidiaries.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub adb_path: String,
    pub fastboot_path: String,
    pub last_loader_path: Option<PathBuf>,
    pub last_rom_directory: Option<PathBuf>,
    pub last_storage_type: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        // Try to find adb and fastboot in PATH
        let adb_path = if cfg!(target_os = "windows") {
            "adb.exe".to_string()
        } else {
            "adb".to_string()
        };

        let fastboot_path = if cfg!(target_os = "windows") {
            "fastboot.exe".to_string()
        } else {
            "fastboot".to_string()
        };

        Self {
            adb_path,
            fastboot_path,
            last_loader_path: None,
            last_rom_directory: None,
            last_storage_type: "ufs".to_string(),
        }
    }
}

impl AppConfig {
    /// Load configuration from file
    pub fn load() -> Self {
        if let Some(config_path) = Self::config_file_path() {
            if let Ok(contents) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Save configuration to file
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(config_path) = Self::config_file_path() {
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let contents = serde_json::to_string_pretty(self)?;
            std::fs::write(&config_path, contents)?;
        }
        Ok(())
    }

    /// Get the configuration file path
    fn config_file_path() -> Option<PathBuf> {
        dirs::config_dir().map(|config_dir| config_dir.join("qdl-gui").join("config.json"))
    }
}
