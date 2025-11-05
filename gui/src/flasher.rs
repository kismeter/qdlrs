// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) Qualcomm Technologies, Inc. and/or its subsidiaries.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Flash a device with ROM files
pub fn flash_device(
    _device_serial: String,
    _loader_path: PathBuf,
    rom_dir: PathBuf,
    _storage_type: String,
    log_messages: Arc<Mutex<Vec<String>>>,
) {
    let add_log = |msg: String| {
        if let Ok(mut logs) = log_messages.lock() {
            logs.push(format!("[{}] {}", chrono::Local::now().format("%H:%M:%S"), msg));
        }
    };
    
    add_log("Starting flash operation...".to_string());
    
    // Find rawprogram and patch XML files
    let rawprogram_files = find_xml_files(&rom_dir, "rawprogram");
    let patch_files = find_xml_files(&rom_dir, "patch");
    
    if rawprogram_files.is_empty() {
        add_log("Error: No rawprogram XML files found in ROM directory".to_string());
        return;
    }
    
    add_log(format!("Found {} rawprogram files", rawprogram_files.len()));
    add_log(format!("Found {} patch files", patch_files.len()));
    
    // Here we would normally call the actual flashing logic from the qdl library
    // For now, this is a placeholder that demonstrates the structure
    
    add_log("Flash operation completed (placeholder)".to_string());
}

/// Find XML files matching a pattern in a directory
fn find_xml_files(dir: &PathBuf, pattern: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.contains(pattern) && name.ends_with(".xml") {
                            files.push(entry.path());
                        }
                    }
                }
            }
        }
    }
    
    files.sort();
    files
}
