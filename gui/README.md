# QDL GUI - Graphical Interface for QDL Flash Tool

A cross-platform GUI application for flashing Qualcomm devices in EDL mode.

![QDL GUI Screenshot](screenshot.png)
*Note: Screenshot to be added*

## Features

- **Multi-device support**: Automatically detect EDL, ADB, and Fastboot devices
- **Device switching**: Automatically reboot ADB or Fastboot devices to EDL mode
- **ROM flashing**: Select ROM directory containing rawprogram*.xml and patch*.xml files
- **Progress tracking**: Real-time progress bar and log window
- **Configuration**: Persistent settings for ADB/Fastboot paths
- **Cross-platform**: Supports macOS, Windows, and Linux

## Usage

1. **Launch the application**:
   ```bash
   cargo run --bin qdl-gui --release
   ```

2. **Configure paths** (optional):
   - Set the path to your `adb` and `fastboot` executables in the bottom panel
   - These settings are saved automatically

3. **Detect devices**:
   - Click "Refresh Devices" to scan for connected devices
   - The app will detect devices in EDL, ADB, or Fastboot mode

4. **Switch to EDL mode** (if needed):
   - Select a device that's in ADB or Fastboot mode
   - Click "Switch to EDL" to reboot it to EDL mode
   - Wait a moment and refresh to see the device in EDL mode

5. **Select files**:
   - Choose the programmer/loader file (typically `prog_firehose_ddr.elf` or `xbl_s_devprg_ns.melf`)
   - Choose the ROM directory containing the flash files
   - Select the storage type (UFS, eMMC, NVMe, or NAND)

6. **Start flashing**:
   - Click "Start Flash" to begin the flashing process
   - Monitor progress in the progress bar and log window

## Requirements

- **For EDL mode**: USB device in EDL mode (9008)
- **For ADB**: `adb` binary in PATH or configured path
- **For Fastboot**: `fastboot` binary in PATH or configured path
- **Loader file**: Appropriate programmer for your device
- **ROM files**: Directory containing rawprogram*.xml and patch*.xml files

## Building from Source

```bash
cd gui
cargo build --release
```

The binary will be available at `../target/release/qdl-gui` (or `qdl-gui.exe` on Windows).

## Platform-Specific Notes

### Linux
- May require udev rules for USB device access
- Run with sudo or add appropriate udev rules for your device

### Windows
- Requires appropriate USB drivers (WinUSB or device-specific drivers)
- Serial backend is used by default

### macOS
- USB support available
- May require permission dialogs for USB access

## Troubleshooting

- **Device not detected**: Ensure proper drivers are installed
- **Permission errors**: Run with elevated privileges or configure udev rules
- **Connection timeout**: Check USB cable and try different USB ports
- **Flash fails**: Verify the loader and ROM files match your device

## License

BSD-3-Clause - See LICENSE file
