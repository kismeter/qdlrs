// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) Qualcomm Technologies, Inc. and/or its subsidiaries.

use anyhow::Result;
use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod config;
mod device;
mod flasher;

use config::AppConfig;
use device::{detect_devices, DeviceInfo, DeviceType};

fn main() -> Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("QDL Flash Tool"),
        ..Default::default()
    };

    eframe::run_native(
        "QDL Flash Tool",
        options,
        Box::new(|_cc| Ok(Box::new(QdlGuiApp::new()))),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {}", e))
}

#[derive(Default)]
struct QdlGuiApp {
    config: AppConfig,
    detected_devices: Vec<DeviceInfo>,
    selected_device: Option<usize>,
    rom_directory: Option<PathBuf>,
    loader_path: Option<PathBuf>,
    storage_type: String,
    is_flashing: bool,
    progress: f32,
    log_messages: Arc<Mutex<Vec<String>>>,
}

impl QdlGuiApp {
    fn new() -> Self {
        let config = AppConfig::load();
        let storage_type = config.last_storage_type.clone();

        Self {
            storage_type,
            config,
            ..Default::default()
        }
    }

    fn refresh_devices(&mut self) {
        match detect_devices() {
            Ok(devices) => {
                self.detected_devices = devices;
                self.add_log(format!("Found {} devices", self.detected_devices.len()));
            }
            Err(e) => {
                self.add_log(format!("Error detecting devices: {}", e));
            }
        }
    }

    fn add_log(&mut self, message: String) {
        log::info!("{}", message);
        if let Ok(mut logs) = self.log_messages.lock() {
            logs.push(format!(
                "[{}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                message
            ));
            if logs.len() > 1000 {
                logs.remove(0);
            }
        }
    }

    fn switch_to_edl(&mut self) {
        if let Some(idx) = self.selected_device {
            if let Some(device) = self.detected_devices.get(idx).cloned() {
                match device.device_type {
                    DeviceType::EDL => {
                        self.add_log("Device is already in EDL mode".to_string());
                    }
                    DeviceType::ADB => {
                        self.add_log("Rebooting to EDL via ADB...".to_string());
                        if let Err(e) =
                            device::reboot_adb_to_edl(&self.config.adb_path, &device.serial)
                        {
                            self.add_log(format!("Failed to reboot via ADB: {}", e));
                        } else {
                            self.add_log("Rebooted to EDL successfully".to_string());
                            // Wait a bit and refresh devices
                            std::thread::sleep(std::time::Duration::from_secs(2));
                            self.refresh_devices();
                        }
                    }
                    DeviceType::Fastboot => {
                        self.add_log("Rebooting to EDL via Fastboot...".to_string());
                        if let Err(e) = device::reboot_fastboot_to_edl(
                            &self.config.fastboot_path,
                            &device.serial,
                        ) {
                            self.add_log(format!("Failed to reboot via Fastboot: {}", e));
                        } else {
                            self.add_log("Rebooted to EDL successfully".to_string());
                            std::thread::sleep(std::time::Duration::from_secs(2));
                            self.refresh_devices();
                        }
                    }
                }
            }
        }
    }

    fn start_flash(&mut self) {
        if self.is_flashing {
            self.add_log("Already flashing...".to_string());
            return;
        }

        // Validate inputs
        if self.selected_device.is_none() {
            self.add_log("No device selected".to_string());
            return;
        }

        if self.rom_directory.is_none() {
            self.add_log("No ROM directory selected".to_string());
            return;
        }

        if self.loader_path.is_none() {
            self.add_log("No loader file selected".to_string());
            return;
        }

        let device_idx = self.selected_device.unwrap();
        if let Some(device) = self.detected_devices.get(device_idx).cloned() {
            if device.device_type != DeviceType::EDL {
                self.add_log("Device is not in EDL mode. Please switch to EDL first.".to_string());
                return;
            }

            self.is_flashing = true;
            self.progress = 0.0;
            self.add_log("Starting flash operation...".to_string());

            // Clone necessary data for the background thread
            let rom_dir = self.rom_directory.clone().unwrap();
            let loader_path = self.loader_path.clone().unwrap();
            let storage_type = self.storage_type.clone();
            let device_serial = device.serial.clone();
            let log_messages = Arc::clone(&self.log_messages);

            // Spawn background thread for flashing
            std::thread::spawn(move || {
                flasher::flash_device(
                    device_serial,
                    loader_path,
                    rom_dir,
                    storage_type,
                    log_messages,
                );
            });
        }
    }
}

impl eframe::App for QdlGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel - Menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Settings").clicked() {
                        // Open settings dialog
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        // Show about dialog
                    }
                });
            });
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("QDL Flash Tool");
            ui.add_space(10.0);

            // Device selection section
            ui.group(|ui| {
                ui.label("Device Selection");
                ui.horizontal(|ui| {
                    if ui.button("Refresh Devices").clicked() {
                        self.refresh_devices();
                    }
                    if ui.button("Switch to EDL").clicked() {
                        self.switch_to_edl();
                    }
                });

                ui.add_space(5.0);

                // Device dropdown
                egui::ComboBox::from_label("Select Device")
                    .selected_text(
                        self.selected_device
                            .and_then(|idx| self.detected_devices.get(idx))
                            .map(|d| format!("{} - {} ({})", d.device_type, d.serial, d.name))
                            .unwrap_or_else(|| "No device selected".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for (idx, device) in self.detected_devices.iter().enumerate() {
                            let label = format!(
                                "{} - {} ({})",
                                device.device_type, device.serial, device.name
                            );
                            ui.selectable_value(&mut self.selected_device, Some(idx), label);
                        }
                    });
            });

            ui.add_space(10.0);

            // File selection section
            ui.group(|ui| {
                ui.label("File Selection");

                ui.horizontal(|ui| {
                    ui.label("Loader file:");
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("ELF files", &["elf", "melf"])
                            .pick_file()
                        {
                            self.loader_path = Some(path.clone());
                            self.add_log(format!("Selected loader: {}", path.display()));
                        }
                    }
                    if let Some(ref path) = self.loader_path {
                        ui.label(path.file_name().unwrap().to_string_lossy().to_string());
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("ROM directory:");
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.rom_directory = Some(path.clone());
                            self.add_log(format!("Selected ROM directory: {}", path.display()));
                        }
                    }
                    if let Some(ref path) = self.rom_directory {
                        ui.label(path.file_name().unwrap().to_string_lossy().to_string());
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Storage type:");
                    egui::ComboBox::from_label("")
                        .selected_text(&self.storage_type)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.storage_type, "ufs".to_string(), "UFS");
                            ui.selectable_value(&mut self.storage_type, "emmc".to_string(), "eMMC");
                            ui.selectable_value(&mut self.storage_type, "nvme".to_string(), "NVMe");
                            ui.selectable_value(&mut self.storage_type, "nand".to_string(), "NAND");
                        });
                });
            });

            ui.add_space(10.0);

            // Flash button and progress
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!self.is_flashing, egui::Button::new("Start Flash"))
                        .clicked()
                    {
                        self.start_flash();
                    }

                    if self.is_flashing {
                        ui.spinner();
                        ui.label(format!("Flashing... {:.0}%", self.progress * 100.0));
                    }
                });

                if self.is_flashing {
                    ui.add(egui::ProgressBar::new(self.progress).show_percentage());
                }
            });

            ui.add_space(10.0);

            // Log window
            ui.group(|ui| {
                ui.label("Log");
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if let Ok(logs) = self.log_messages.lock() {
                            for msg in logs.iter() {
                                ui.label(msg);
                            }
                        }
                    });
            });
        });

        // Bottom panel - Settings paths
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("ADB:");
                ui.text_edit_singleline(&mut self.config.adb_path);
                ui.label("Fastboot:");
                ui.text_edit_singleline(&mut self.config.fastboot_path);
            });
        });

        // Request repaint for progress updates
        ctx.request_repaint();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save config before exiting
        self.config.last_loader_path = self.loader_path.clone();
        self.config.last_rom_directory = self.rom_directory.clone();
        self.config.last_storage_type = self.storage_type.clone();
        let _ = self.config.save();
    }
}
