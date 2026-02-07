use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rusb::{DeviceHandle, GlobalContext};
use std::time::Duration;

/// Teensy 2.0 HalfKay bootloader USB identifiers.
const HALFKAY_VID: u16 = 0x16C0;
const HALFKAY_PID: u16 = 0x0478;

/// ATmega32U4 flash page size in bytes.
const PAGE_SIZE: usize = 128;

/// Total flash size of ATmega32U4 (32KB).
const FLASH_SIZE: usize = 32768;

/// USB control transfer timeout.
const USB_TIMEOUT: Duration = Duration::from_secs(2);

/// Delay after each page write to allow flash programming.
const PAGE_WRITE_DELAY: Duration = Duration::from_millis(5);

/// Detect whether a Teensy in HalfKay bootloader mode is connected.
pub fn detect() -> Result<bool> {
    let devices = rusb::devices().context("failed to enumerate USB devices")?;
    for device in devices.iter() {
        let desc = device
            .device_descriptor()
            .context("failed to read device descriptor")?;
        if desc.vendor_id() == HALFKAY_VID && desc.product_id() == HALFKAY_PID {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Open the Teensy HalfKay bootloader device.
fn open_device() -> Result<DeviceHandle<GlobalContext>> {
    let devices = rusb::devices().context("failed to enumerate USB devices")?;
    for device in devices.iter() {
        let desc = device
            .device_descriptor()
            .context("failed to read device descriptor")?;
        if desc.vendor_id() == HALFKAY_VID && desc.product_id() == HALFKAY_PID {
            let handle = device.open().context(
                "failed to open Teensy bootloader (may need root/sudo or udev rules)",
            )?;
            return Ok(handle);
        }
    }
    bail!("Teensy bootloader not found. Press the reset button on the Teensy and try again.");
}

/// Flash firmware data to the Teensy via HalfKay protocol.
///
/// `base_address` is the starting address of the firmware image.
/// `data` is the firmware binary, which will be split into 128-byte pages.
pub fn flash(base_address: u32, data: &[u8]) -> Result<()> {
    let handle = open_device()?;

    let end_address = base_address as usize + data.len();
    if end_address > FLASH_SIZE {
        bail!(
            "firmware too large: {} bytes at offset 0x{:04X} exceeds {} byte flash",
            data.len(),
            base_address,
            FLASH_SIZE
        );
    }

    let total_pages = (data.len() + PAGE_SIZE - 1) / PAGE_SIZE;
    let pb = ProgressBar::new(total_pages as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} pages")
            .unwrap()
            .progress_chars("=> "),
    );
    pb.set_message("Flashing");

    for (page_idx, chunk) in data.chunks(PAGE_SIZE).enumerate() {
        let address = base_address as usize + page_idx * PAGE_SIZE;

        // Skip pages that are all 0xFF (erased flash)
        if chunk.iter().all(|&b| b == 0xFF) {
            pb.inc(1);
            continue;
        }

        // Build the page buffer: 2 bytes address (little-endian) + page data
        let mut buf = vec![0u8; 2 + PAGE_SIZE];
        buf[0] = (address & 0xFF) as u8;
        buf[1] = ((address >> 8) & 0xFF) as u8;
        buf[2..2 + chunk.len()].copy_from_slice(chunk);
        // Remaining bytes stay 0xFF if chunk is shorter than PAGE_SIZE

        write_page(&handle, &buf)
            .with_context(|| format!("failed to write page at address 0x{:04X}", address))?;

        std::thread::sleep(PAGE_WRITE_DELAY);
        pb.inc(1);
    }

    pb.finish_with_message("Flashed");

    // Reboot the Teensy
    reboot(&handle)?;
    println!("Teensy rebooted. Firmware should be running.");

    Ok(())
}

/// Write a single page via HalfKay USB control transfer.
fn write_page(handle: &DeviceHandle<GlobalContext>, buf: &[u8]) -> Result<()> {
    // HalfKay uses HID SET_REPORT via control transfer
    // bmRequestType: 0x21 (host-to-device, class, interface)
    // bRequest: 0x09 (SET_REPORT)
    // wValue: 0x0200 (report type: output, report ID: 0)
    // wIndex: 0 (interface 0)
    handle
        .write_control(0x21, 0x09, 0x0200, 0, buf, USB_TIMEOUT)
        .context("USB control transfer failed")?;
    Ok(())
}

/// Send reboot command to Teensy (write to address 0xFFFF).
fn reboot(handle: &DeviceHandle<GlobalContext>) -> Result<()> {
    let mut buf = vec![0u8; 2 + PAGE_SIZE];
    buf[0] = 0xFF;
    buf[1] = 0xFF;
    // Ignore errors on reboot — the device disconnects immediately
    let _ = handle.write_control(0x21, 0x09, 0x0200, 0, &buf, USB_TIMEOUT);
    Ok(())
}
