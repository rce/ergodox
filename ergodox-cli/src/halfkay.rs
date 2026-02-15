use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rusb::{DeviceHandle, GlobalContext};
use std::time::Duration;

/// Teensy 2.0 HalfKay bootloader USB identifiers.
const HALFKAY_VID: u16 = 0x16C0;
const HALFKAY_PID: u16 = 0x0478;

/// Running keyboard USB identifiers (must match firmware device descriptor).
const KEYBOARD_VID: u16 = 0x16C0;
const KEYBOARD_PID: u16 = 0x047E;

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

        let buf = build_page_buffer(address, chunk);
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

// HalfKay protocol constants — this is PJRC's standard bootloader protocol.
// It piggybacks on HID SET_REPORT control transfers to write flash pages.

/// HID class request: host-to-device, class, interface.
const HALFKAY_REQUEST_TYPE: u8 = 0x21;
/// HID SET_REPORT request code.
const HALFKAY_SET_REPORT: u8 = 0x09;
/// wValue for SET_REPORT: report type = Output (0x02), report ID = 0.
const HALFKAY_REPORT_VALUE: u16 = 0x0200;

/// The "reboot into application" sentinel address. Writing a page to this
/// address tells HalfKay to jump to the application code at address 0x0000.
const HALFKAY_REBOOT_ADDRESS: u16 = 0xFFFF;

/// Write a single page via HalfKay USB control transfer.
fn write_page(handle: &DeviceHandle<GlobalContext>, buf: &[u8]) -> Result<()> {
    handle
        .write_control(
            HALFKAY_REQUEST_TYPE,
            HALFKAY_SET_REPORT,
            HALFKAY_REPORT_VALUE,
            0,
            buf,
            USB_TIMEOUT,
        )
        .context("USB control transfer failed")?;
    Ok(())
}

/// Send reboot command to Teensy (write to address 0xFFFF).
fn reboot(handle: &DeviceHandle<GlobalContext>) -> Result<()> {
    let mut buf = vec![0u8; 2 + PAGE_SIZE];
    buf[0] = HALFKAY_REBOOT_ADDRESS as u8;
    buf[1] = (HALFKAY_REBOOT_ADDRESS >> 8) as u8;
    // Ignore errors on reboot — the device disconnects immediately
    let _ = handle.write_control(
        HALFKAY_REQUEST_TYPE,
        HALFKAY_SET_REPORT,
        HALFKAY_REPORT_VALUE,
        0,
        &buf,
        USB_TIMEOUT,
    );
    Ok(())
}

/// Vendor USB control request type: host-to-device, vendor, device recipient.
/// This is a standard USB bmRequestType value — it tells the device "this is a
/// custom vendor command", as opposed to a standard or class request.
const REBOOT_REQUEST_TYPE: u8 = 0x40;

/// Our custom bRequest value meaning "jump to bootloader". This is arbitrary —
/// we own the entire 0x00..=0xFF bRequest space under vendor request type 0x40.
/// We picked 0xFF but any value would work. The firmware matches on the
/// (bmRequestType, bRequest) pair (0x40, 0xFF) in its USB setup handler.
const REBOOT_REQUEST: u8 = 0xFF;

/// Try to find the running keyboard and send a vendor request to jump to bootloader.
/// Returns true if the keyboard was found and rebooted.
pub fn reboot_to_bootloader() -> Result<bool> {
    let devices = rusb::devices().context("failed to enumerate USB devices")?;
    for device in devices.iter() {
        let desc = device
            .device_descriptor()
            .context("failed to read device descriptor")?;
        if desc.vendor_id() == KEYBOARD_VID && desc.product_id() == KEYBOARD_PID {
            let handle = device
                .open()
                .context("failed to open keyboard device")?;
            let _ = handle.write_control(REBOOT_REQUEST_TYPE, REBOOT_REQUEST, 0, 0, &[], USB_TIMEOUT);
            return Ok(true);
        }
    }
    Ok(false)
}

/// Build the page buffer that HalfKay expects: 2-byte little-endian address
/// followed by PAGE_SIZE bytes of data. Unfilled bytes default to 0xFF
/// (matching erased flash), so short final pages are safe.
fn build_page_buffer(address: usize, data: &[u8]) -> Vec<u8> {
    assert!(data.len() <= PAGE_SIZE);
    let mut buf = vec![0xFFu8; 2 + PAGE_SIZE];
    buf[0] = (address & 0xFF) as u8;
    buf[1] = ((address >> 8) & 0xFF) as u8;
    buf[2..2 + data.len()].copy_from_slice(data);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // USB identity contracts
    //
    // The keyboard presents two different USB identities depending on its
    // state. The CLI uses these to distinguish "running firmware I can ask
    // to reboot" from "bootloader ready to receive firmware pages".
    //
    // Both share VID 0x16C0 (Van Ooijen Technische Informatica's shared
    // hobbyist pool). The PIDs are conventional for Teensy-based projects.
    // ========================================================================

    #[test]
    fn running_keyboard_and_bootloader_share_vid_but_differ_in_pid() {
        // Same vendor — both are "Van Ooijen Technische Informatica" shared IDs
        assert_eq!(KEYBOARD_VID, HALFKAY_VID);
        // Different product IDs so we can tell them apart on the bus
        assert_ne!(KEYBOARD_PID, HALFKAY_PID);
    }

    #[test]
    fn firmware_device_descriptor_matches_cli_expectations() {
        // The firmware's DEVICE_DESCRIPTOR (in firmware/src/hid.rs) encodes the
        // VID/PID as little-endian bytes at offsets 8-11. If someone changes
        // them in the firmware without updating the CLI, auto-reboot breaks
        // silently — the CLI just won't find the keyboard on the bus.
        //
        // This is the byte layout from the USB 2.0 spec, Table 9-8:
        //   offset 8-9:  idVendor  (little-endian)
        //   offset 10-11: idProduct (little-endian)
        let expected_vid_bytes = KEYBOARD_VID.to_le_bytes();
        assert_eq!(expected_vid_bytes, [0xC0, 0x16]);

        let expected_pid_bytes = KEYBOARD_PID.to_le_bytes();
        assert_eq!(expected_pid_bytes, [0x7E, 0x04]);
    }

    // ========================================================================
    // Vendor reboot request
    //
    // This is our custom protocol: a single USB control transfer that tells
    // the running firmware to jump into the HalfKay bootloader. The firmware
    // matches on the (bmRequestType, bRequest) pair in its SETUP handler.
    // ========================================================================

    #[test]
    fn reboot_request_type_is_vendor_device_out() {
        // USB bmRequestType is a bitfield (USB 2.0 spec, Table 9-2):
        //   bit 7:    direction     (0 = host-to-device)
        //   bits 6-5: type          (0b10 = vendor)
        //   bits 4-0: recipient     (0b00000 = device)
        //
        // 0b_0_10_00000 = 0x40
        let direction = (REBOOT_REQUEST_TYPE >> 7) & 1;
        let req_type = (REBOOT_REQUEST_TYPE >> 5) & 0b11;
        let recipient = REBOOT_REQUEST_TYPE & 0b11111;

        assert_eq!(direction, 0, "direction should be host-to-device");
        assert_eq!(req_type, 0b10, "type should be 'vendor'");
        assert_eq!(recipient, 0, "recipient should be 'device'");
    }

    #[test]
    fn reboot_request_code_is_in_vendor_space() {
        // Under bmRequestType=0x40 (vendor), the full bRequest range 0x00..=0xFF
        // is ours to define. Standard requests (GET_DESCRIPTOR=0x06, etc.) live
        // under bmRequestType=0x80 and don't collide. We picked 0xFF but it's
        // arbitrary — the firmware just needs to match.
        assert_eq!(REBOOT_REQUEST, 0xFF);
    }

    // ========================================================================
    // HalfKay page protocol
    //
    // HalfKay is PJRC's bootloader for Teensy boards. It receives firmware
    // as a series of flash pages via HID SET_REPORT control transfers.
    // Each transfer carries: [addr_lo, addr_hi, ...128 bytes of page data].
    // ========================================================================

    #[test]
    fn halfkay_uses_hid_set_report() {
        // HalfKay doesn't define its own protocol from scratch — it reuses the
        // standard HID class SET_REPORT request. This is clever because HID
        // devices don't need custom drivers on any OS.
        //
        // bmRequestType 0x21: host-to-device, class, interface
        // bRequest 0x09: SET_REPORT (HID spec section 7.2.2)
        // wValue 0x0200: report type = Output (0x02), report ID = 0
        assert_eq!(HALFKAY_REQUEST_TYPE, 0x21);
        assert_eq!(HALFKAY_SET_REPORT, 0x09);
        assert_eq!(HALFKAY_REPORT_VALUE, 0x0200);
    }

    #[test]
    fn page_buffer_is_two_byte_address_then_page_data() {
        // HalfKay page format: [address_lo, address_hi, data[0], data[1], ...]
        // Address is little-endian, matching the AVR's native byte order.
        let buf = build_page_buffer(0x1A00, &[0xDE, 0xAD]);

        assert_eq!(buf.len(), 2 + PAGE_SIZE, "always 2 + PAGE_SIZE bytes");
        assert_eq!(buf[0], 0x00, "address low byte");
        assert_eq!(buf[1], 0x1A, "address high byte");
        assert_eq!(buf[2], 0xDE, "first data byte");
        assert_eq!(buf[3], 0xAD, "second data byte");
        // Remaining bytes are 0xFF — matching erased flash, so short final
        // pages don't corrupt anything.
        assert!(buf[4..].iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn page_size_matches_atmega32u4_flash_page() {
        // The ATmega32U4 datasheet (section 28.5) specifies 128-byte flash
        // pages. HalfKay writes one page per USB transfer, so this must match.
        assert_eq!(PAGE_SIZE, 128);
    }

    #[test]
    fn flash_size_is_32kb() {
        // ATmega32U4 has 32KB of flash. The bootloader lives at the top
        // (0x7E00-0x7FFF for HalfKay), but we rely on the address check
        // rather than carving out the bootloader region explicitly.
        assert_eq!(FLASH_SIZE, 32 * 1024);
    }

    #[test]
    fn reboot_sentinel_is_0xffff() {
        // Writing to address 0xFFFF tells HalfKay "I'm done, jump to the
        // application." This address is outside the flash, so it can't be
        // confused with a real page write.
        assert_eq!(HALFKAY_REBOOT_ADDRESS, 0xFFFF);
        assert!(
            HALFKAY_REBOOT_ADDRESS as usize >= FLASH_SIZE,
            "reboot sentinel must be outside writable flash"
        );
    }

    #[test]
    fn all_0xff_pages_are_erased_flash() {
        // Erased NOR flash reads as all 0xFF. We skip these pages during
        // flashing because writing 0xFF to already-erased flash is a no-op
        // that just wastes time. This is why build_page_buffer pads with 0xFF.
        let buf = build_page_buffer(0x0000, &[]);
        // Data portion should be all 0xFF (erased)
        assert!(buf[2..].iter().all(|&b| b == 0xFF));
    }

    // ========================================================================
    // Cross-crate contract: firmware ↔ CLI
    //
    // The firmware (AVR target, can't run here) and CLI (host target) must
    // agree on several values. We can't import the firmware crate, but we
    // can document and assert the CLI's side of each contract.
    // ========================================================================

    #[test]
    fn vendor_request_pair_must_match_firmware_setup_handler() {
        // The firmware's handle_setup() in hid.rs matches on:
        //   (0x40, 0xFF) => jump_to_bootloader(dp)
        //
        // If either side changes, auto-reboot silently stops working — the
        // firmware STALLs the unknown request, and the CLI thinks it sent
        // the reboot successfully (write_control ignores errors).
        assert_eq!(
            (REBOOT_REQUEST_TYPE, REBOOT_REQUEST),
            (0x40, 0xFF),
            "must match firmware/src/hid.rs handle_setup() vendor request arm"
        );
    }

    #[test]
    fn device_descriptor_vid_pid_must_match_firmware() {
        // The firmware's DEVICE_DESCRIPTOR in hid.rs has these bytes at
        // offsets 8-11 (little-endian):
        //   [0xC0, 0x16, 0x7E, 0x04]
        //
        // If the firmware changes its VID/PID, the CLI won't find it on
        // the bus and will fall back to "press the reset button".
        assert_eq!(
            (KEYBOARD_VID, KEYBOARD_PID),
            (0x16C0, 0x047E),
            "must match firmware/src/hid.rs DEVICE_DESCRIPTOR idVendor/idProduct"
        );
    }
}
