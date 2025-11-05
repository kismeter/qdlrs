// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) Qualcomm Technologies, Inc. and/or its subsidiaries.

use anyhow::{bail, Result};
use std::fmt;
use std::process::Command;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    EDL,
    ADB,
    Fastboot,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceType::EDL => write!(f, "EDL"),
            DeviceType::ADB => write!(f, "ADB"),
            DeviceType::Fastboot => write!(f, "Fastboot"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_type: DeviceType,
    pub serial: String,
    pub name: String,
}

const USB_VID_QCOM: u16 = 0x05c6;
const USB_PID_EDL: [u16; 2] = [0x9008, 0x900e];

/// Detect all connected devices (EDL, ADB, Fastboot)
pub fn detect_devices() -> Result<Vec<DeviceInfo>> {
    let mut devices = Vec::new();

    // Detect EDL devices via USB
    if let Ok(edl_devices) = detect_edl_devices() {
        devices.extend(edl_devices);
    }

    // Detect ADB devices
    if let Ok(adb_devices) = detect_adb_devices() {
        devices.extend(adb_devices);
    }

    // Detect Fastboot devices
    if let Ok(fastboot_devices) = detect_fastboot_devices() {
        devices.extend(fastboot_devices);
    }

    Ok(devices)
}

/// Detect EDL devices via USB
fn detect_edl_devices() -> Result<Vec<DeviceInfo>> {
    let mut devices = Vec::new();

    match rusb::devices() {
        Ok(usb_devices) => {
            for device in usb_devices.iter() {
                if let Ok(desc) = device.device_descriptor() {
                    if desc.vendor_id() == USB_VID_QCOM && USB_PID_EDL.contains(&desc.product_id())
                    {
                        let serial = match device.open() {
                            Ok(handle) => {
                                match handle.read_product_string_ascii(&desc) {
                                    Ok(prod_str) => {
                                        // Extract serial number from product string
                                        if let Some(sn_pos) = prod_str.find("_SN:") {
                                            prod_str[sn_pos + 4..].to_string()
                                        } else {
                                            format!(
                                                "{:04x}:{:04x}",
                                                desc.vendor_id(),
                                                desc.product_id()
                                            )
                                        }
                                    }
                                    Err(_) => format!(
                                        "{:04x}:{:04x}",
                                        desc.vendor_id(),
                                        desc.product_id()
                                    ),
                                }
                            }
                            Err(_) => format!("{:04x}:{:04x}", desc.vendor_id(), desc.product_id()),
                        };

                        devices.push(DeviceInfo {
                            device_type: DeviceType::EDL,
                            serial: serial.clone(),
                            name: "Qualcomm EDL Device".to_string(),
                        });
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to enumerate USB devices: {}", e);
        }
    }

    Ok(devices)
}

/// Detect ADB devices
fn detect_adb_devices() -> Result<Vec<DeviceInfo>> {
    let mut devices = Vec::new();

    // Try to find adb in common locations
    let adb_paths = if cfg!(target_os = "windows") {
        vec!["adb.exe", "adb"]
    } else {
        vec!["adb", "/usr/bin/adb", "/usr/local/bin/adb"]
    };

    for adb_path in adb_paths {
        if let Ok(output) = Command::new(adb_path).arg("devices").output() {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                for line in output_str.lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[1] == "device" {
                        devices.push(DeviceInfo {
                            device_type: DeviceType::ADB,
                            serial: parts[0].to_string(),
                            name: "Android Device".to_string(),
                        });
                    }
                }
                break; // Found working adb, no need to try other paths
            }
        }
    }

    Ok(devices)
}

/// Detect Fastboot devices
fn detect_fastboot_devices() -> Result<Vec<DeviceInfo>> {
    let mut devices = Vec::new();

    // Try to find fastboot in common locations
    let fastboot_paths = if cfg!(target_os = "windows") {
        vec!["fastboot.exe", "fastboot"]
    } else {
        vec!["fastboot", "/usr/bin/fastboot", "/usr/local/bin/fastboot"]
    };

    for fastboot_path in fastboot_paths {
        if let Ok(output) = Command::new(fastboot_path).arg("devices").output() {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                for line in output_str.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[1] == "fastboot" {
                        devices.push(DeviceInfo {
                            device_type: DeviceType::Fastboot,
                            serial: parts[0].to_string(),
                            name: "Fastboot Device".to_string(),
                        });
                    }
                }
                break; // Found working fastboot, no need to try other paths
            }
        }
    }

    Ok(devices)
}

/// Reboot ADB device to EDL mode
pub fn reboot_adb_to_edl(adb_path: &str, serial: &str) -> Result<()> {
    let adb = if adb_path.is_empty() { "adb" } else { adb_path };

    let output = Command::new(adb)
        .args(["-s", serial, "reboot", "edl"])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        bail!("ADB command failed: {}", error);
    }

    Ok(())
}

/// Reboot Fastboot device to EDL mode
pub fn reboot_fastboot_to_edl(fastboot_path: &str, serial: &str) -> Result<()> {
    let fastboot = if fastboot_path.is_empty() {
        "fastboot"
    } else {
        fastboot_path
    };

    let output = Command::new(fastboot)
        .args(["-s", serial, "oem", "edl"])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        bail!("Fastboot command failed: {}", error);
    }

    Ok(())
}
