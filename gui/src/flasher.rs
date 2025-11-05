// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) Qualcomm Technologies, Inc. and/or its subsidiaries.

use anyhow::Result;
use qdl::firehose_get_default_sector_size;
use qdl::parsers::{firehose_parser_ack_nak, firehose_parser_configure_response};
use qdl::sahara::{sahara_run, SaharaCmdModeCmd, SaharaMode};
use qdl::types::FirehoseResetMode;
use qdl::types::{FirehoseConfiguration, FirehoseStorageType, QdlBackend, QdlDevice};
use qdl::{firehose_configure, firehose_read, firehose_reset, setup_target_device};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use xmltree::Element;

/// Flash a device with ROM files
pub fn flash_device(
    device_serial: String,
    loader_path: PathBuf,
    rom_dir: PathBuf,
    storage_type: String,
    log_messages: Arc<Mutex<Vec<String>>>,
) {
    let add_log = |msg: String| {
        log::info!("{}", msg);
        if let Ok(mut logs) = log_messages.lock() {
            logs.push(format!(
                "[{}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                msg
            ));
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

    // Perform the actual flashing
    if let Err(e) = flash_internal(
        device_serial,
        loader_path,
        rawprogram_files,
        patch_files,
        storage_type,
        Arc::clone(&log_messages),
    ) {
        add_log(format!("Flash operation failed: {}", e));
    } else {
        add_log("Flash operation completed successfully!".to_string());
    }
}

fn flash_internal(
    device_serial: String,
    loader_path: PathBuf,
    rawprogram_files: Vec<PathBuf>,
    patch_files: Vec<PathBuf>,
    storage_type: String,
    log_messages: Arc<Mutex<Vec<String>>>,
) -> Result<()> {
    let add_log = |msg: String| {
        log::info!("{}", msg);
        if let Ok(mut logs) = log_messages.lock() {
            logs.push(format!(
                "[{}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                msg
            ));
        }
    };

    // Load the MBN loader binary
    let mbn_loader = fs::read(&loader_path)?;
    add_log(format!("Loaded programmer: {}", loader_path.display()));

    // Set up the device connection
    let backend = QdlBackend::Usb;
    add_log("Connecting to device...".to_string());

    let rw_channel = setup_target_device(backend, Some(device_serial.clone()), None)?;

    let storage_type_enum = FirehoseStorageType::from_str(&storage_type)?;
    let sector_size = firehose_get_default_sector_size(&storage_type)
        .ok_or_else(|| anyhow::anyhow!("Unknown storage type"))?;

    let mut qdl_dev = QdlDevice {
        rw: rw_channel,
        fh_cfg: FirehoseConfiguration {
            hash_packets: false,
            read_back_verify: false,
            storage_type: storage_type_enum,
            storage_sector_size: sector_size,
            storage_slot: 0,
            bypass_storage: false,
            backend,
            skip_firehose_log: true,
            verbose_firehose: false,
            ..Default::default()
        },
        reset_on_drop: false,
    };

    add_log("Device connected".to_string());

    // Get device serial number
    let sn = sahara_run(
        &mut qdl_dev,
        SaharaMode::Command,
        Some(SaharaCmdModeCmd::ReadSerialNum),
        &mut [],
        vec![],
        false,
    )?;
    let sn = u32::from_le_bytes([sn[0], sn[1], sn[2], sn[3]]);
    add_log(format!("Chip serial number: 0x{:x}", sn));

    // Send the loader
    add_log("Uploading programmer...".to_string());
    sahara_run(
        &mut qdl_dev,
        SaharaMode::WaitingForImage,
        None,
        &mut [mbn_loader],
        vec![],
        false,
    )?;

    // Activate the Firehose reset-on-drop listener
    qdl_dev.reset_on_drop = true;

    // Get welcome logs
    firehose_read(&mut qdl_dev, firehose_parser_ack_nak)?;

    // Configure the device
    add_log("Configuring device...".to_string());
    firehose_configure(&mut qdl_dev, false)?;
    firehose_read(&mut qdl_dev, firehose_parser_configure_response)?;

    // Process rawprogram and patch files
    let all_files: Vec<PathBuf> = rawprogram_files.into_iter().chain(patch_files).collect();

    for (idx, file_path) in all_files.iter().enumerate() {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        add_log(format!(
            "Processing {}/{}: {}",
            idx + 1,
            all_files.len(),
            file_name
        ));

        let program_file = fs::read(file_path)?;
        let _xml = Element::parse(&program_file[..])?;

        // TODO: Full XML parsing and execution using parse_program_xml from CLI
        // For now, this is a basic implementation that sets up the device
        // Full implementation would require integrating the programfile module
        add_log(format!("Parsed XML file: {}", file_name));
    }

    add_log("WARNING: Full flash implementation is incomplete - this is a demonstration only".to_string());

    // Reset the device
    add_log("Resetting device...".to_string());
    qdl_dev.reset_on_drop = false;
    firehose_reset(&mut qdl_dev, &FirehoseResetMode::Reset, 0)?;

    Ok(())
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
