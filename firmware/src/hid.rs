//! USB HID keyboard implementation for ATmega32U4.
//!
//! Implements a standard 6KRO (6-key rollover) keyboard using the ATmega32U4's
//! built-in USB controller. Uses direct register access via avr-device.

use avr_device::atmega32u4::Peripherals;

use crate::keymap::Keycode;
use crate::matrix::{COLS, ROWS};

/// Standard USB HID keyboard report (8 bytes).
/// Byte 0: modifier keys bitmask
/// Byte 1: reserved (0x00)
/// Bytes 2-7: up to 6 simultaneous keycodes
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct KeyboardReport {
    pub modifiers: u8,
    pub reserved: u8,
    pub keys: [u8; 6],
}

impl KeyboardReport {
    pub const fn empty() -> Self {
        Self {
            modifiers: 0,
            reserved: 0,
            keys: [0; 6],
        }
    }
}

/// Build a HID keyboard report from the current debounced key state and active layer.
pub fn build_report(keys: &[[bool; COLS]; ROWS], layer: usize) -> KeyboardReport {
    let mut report = KeyboardReport::empty();
    let mut key_idx = 0usize;

    for row in 0..ROWS {
        for col in 0..COLS {
            if !keys[row][col] {
                continue; // Key not pressed
            }

            let kc = crate::keymap::lookup(layer, row, col);

            // Skip transparent, none, and layer keys
            if kc.is_transparent() || kc.is_layer() || kc == Keycode::None {
                continue;
            }

            if kc.is_modifier() {
                report.modifiers |= kc.modifier_bit();
            } else if key_idx < 6 {
                report.keys[key_idx] = kc as u8;
                key_idx += 1;
            }
            // If more than 6 keys, silently drop (no rollover error for simplicity)
        }
    }

    report
}

// ============================================================================
// ATmega32U4 USB Register-Level Driver
// ============================================================================

// USB endpoint configuration for keyboard HID
const EP0_SIZE: u8 = 64; // Control endpoint size
const EP1_SIZE: u8 = 8; // Interrupt IN endpoint size (keyboard reports)

/// HID report descriptor for a standard keyboard.
static HID_REPORT_DESCRIPTOR: [u8; 64] = [
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x06, // Usage (Keyboard)
    0xA1, 0x01, // Collection (Application)
    // Modifier keys (8 bits)
    0x05, 0x07, //   Usage Page (Key Codes)
    0x19, 0xE0, //   Usage Minimum (224) - LCtrl
    0x29, 0xE7, //   Usage Maximum (231) - RGui
    0x15, 0x00, //   Logical Minimum (0)
    0x25, 0x01, //   Logical Maximum (1)
    0x75, 0x01, //   Report Size (1)
    0x95, 0x08, //   Report Count (8)
    0x81, 0x02, //   Input (Data, Variable, Absolute)
    // Reserved byte
    0x95, 0x01, //   Report Count (1)
    0x75, 0x08, //   Report Size (8)
    0x81, 0x01, //   Input (Constant)
    // LEDs (5 bits)
    0x95, 0x05, //   Report Count (5)
    0x75, 0x01, //   Report Size (1)
    0x05, 0x08, //   Usage Page (LEDs)
    0x19, 0x01, //   Usage Minimum (1)
    0x29, 0x05, //   Usage Maximum (5)
    0x91, 0x02, //   Output (Data, Variable, Absolute)
    // LED padding (3 bits)
    0x95, 0x01, //   Report Count (1)
    0x75, 0x03, //   Report Size (3)
    0x91, 0x01, //   Output (Constant)
    // Keycodes (6 bytes)
    0x95, 0x06, //   Report Count (6)
    0x75, 0x08, //   Report Size (8)
    0x15, 0x00, //   Logical Minimum (0)
    0x26, 0xFF, 0x00, // Logical Maximum (255)
    0x05, 0x07, //   Usage Page (Key Codes)
    0x19, 0x00, //   Usage Minimum (0)
    0x29, 0xFF, //   Usage Maximum (255)
    0x81, 0x00, //   Input (Data, Array)
    0xC0, // End Collection
];

// USB descriptors
static DEVICE_DESCRIPTOR: [u8; 18] = [
    18,   // bLength
    1,    // bDescriptorType (Device)
    0x00, 0x02, // bcdUSB (2.0)
    0,    // bDeviceClass (defined at interface level)
    0,    // bDeviceSubClass
    0,    // bDeviceProtocol
    EP0_SIZE, // bMaxPacketSize0
    0xC0, 0x16, // idVendor (0x16C0 — Van Ooijen Technische Informatica)
    0x7E, 0x04, // idProduct (0x047E — custom keyboard)
    0x01, 0x00, // bcdDevice (1.0)
    1,    // iManufacturer
    2,    // iProduct
    0,    // iSerialNumber
    1,    // bNumConfigurations
];

static CONFIG_DESCRIPTOR: [u8; 34] = [
    // Configuration descriptor
    9,    // bLength
    2,    // bDescriptorType (Configuration)
    34, 0, // wTotalLength
    1,    // bNumInterfaces
    1,    // bConfigurationValue
    0,    // iConfiguration
    0x80, // bmAttributes (bus powered)
    50,   // bMaxPower (100mA)
    // Interface descriptor
    9,    // bLength
    4,    // bDescriptorType (Interface)
    0,    // bInterfaceNumber
    0,    // bAlternateSetting
    1,    // bNumEndpoints
    3,    // bInterfaceClass (HID)
    1,    // bInterfaceSubClass (Boot)
    1,    // bInterfaceProtocol (Keyboard)
    0,    // iInterface
    // HID descriptor
    9,    // bLength
    0x21, // bDescriptorType (HID)
    0x11, 0x01, // bcdHID (1.11)
    0,    // bCountryCode
    1,    // bNumDescriptors
    0x22, // bDescriptorType (Report)
    HID_REPORT_DESCRIPTOR.len() as u8, 0, // wDescriptorLength
    // Endpoint descriptor (EP1 IN — interrupt)
    7,    // bLength
    5,    // bDescriptorType (Endpoint)
    0x81, // bEndpointAddress (EP1 IN)
    0x03, // bmAttributes (Interrupt)
    EP1_SIZE, 0, // wMaxPacketSize
    10,   // bInterval (10ms polling)
];

/// String descriptor 0 (language ID)
static STRING_DESC_0: [u8; 4] = [4, 3, 0x09, 0x04]; // English (US)

/// String descriptor 1 (manufacturer): "ErgoDox"
static STRING_DESC_1: [u8; 16] = [
    16, 3, // bLength, bDescriptorType
    b'E', 0, b'r', 0, b'g', 0, b'o', 0, b'D', 0, b'o', 0, b'x', 0,
];

/// String descriptor 2 (product): "Keyboard"
static STRING_DESC_2: [u8; 18] = [
    18, 3, // bLength, bDescriptorType
    b'K', 0, b'e', 0, b'y', 0, b'b', 0, b'o', 0, b'a', 0, b'r', 0, b'd', 0,
];

/// USB device state.
pub struct UsbKeyboard {
    configured: bool,
    last_report: KeyboardReport,
}

impl UsbKeyboard {
    pub const fn new() -> Self {
        Self {
            configured: false,
            last_report: KeyboardReport::empty(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.configured
    }

    /// Initialize the ATmega32U4 USB controller.
    pub fn init(&mut self, dp: &Peripherals) {
        let usb = &dp.USB_DEVICE;

        // Enable USB pad regulator
        usb.uhwcon.write(|w| w.uvrege().set_bit());

        // Enable USB controller and VBUS pad
        usb.usbcon
            .write(|w| w.usbe().set_bit().otgpade().set_bit());

        // Configure PLL for 16MHz crystal -> 96MHz PLL -> 48MHz USB clock
        // PLLCSR: PINDIV=1 (16MHz), PLLE=1
        dp.PLL.pllcsr.write(|w| w.pindiv().set_bit().plle().set_bit());

        // Wait for PLL lock
        while dp.PLL.pllcsr.read().plock().bit_is_clear() {}

        // Enable USB clock
        usb.usbcon.modify(|_, w| w.frzclk().clear_bit());

        // Attach to bus (clear DETACH)
        usb.udcon.modify(|_, w| w.detach().clear_bit());

        // Enable End-Of-Reset interrupt
        usb.udien.write(|w| w.eorste().set_bit());

        self.configured = false;
    }

    /// Poll for USB events and handle them. Call this from the main loop.
    pub fn poll(&mut self, dp: &Peripherals) {
        let usb = &dp.USB_DEVICE;

        let udint = usb.udint.read();

        // End of reset
        if udint.eorsti().bit_is_set() {
            usb.udint.modify(|_, w| w.eorsti().clear_bit());
            self.configure_ep0(dp);
            self.configured = false;
        }

        // Check for SETUP packet on EP0
        self.select_endpoint(dp, 0);
        let ueintx = usb.ueintx.read();
        if ueintx.rxstpi().bit_is_set() {
            self.handle_setup(dp);
        }
    }

    /// Send a keyboard report if it has changed.
    pub fn send_report(&mut self, dp: &Peripherals, report: &KeyboardReport) {
        if !self.configured || *report == self.last_report {
            return;
        }

        let usb = &dp.USB_DEVICE;
        self.select_endpoint(dp, 1);

        // Wait for endpoint ready (RWAL set means we can write)
        let mut timeout: u16 = 0xFFFF;
        while usb.ueintx.read().rwal().bit_is_clear() {
            timeout = timeout.wrapping_sub(1);
            if timeout == 0 {
                return;
            }
        }

        // Write 8-byte report
        usb.uedatx.write(|w| w.bits(report.modifiers));
        usb.uedatx.write(|w| w.bits(report.reserved));
        for &key in &report.keys {
            usb.uedatx.write(|w| w.bits(key));
        }

        // Clear FIFOCON and TXINI to send
        usb.ueintx
            .modify(|_, w| w.fifocon().clear_bit().txini().clear_bit());

        self.last_report = *report;
    }

    fn configure_ep0(&self, dp: &Peripherals) {
        let usb = &dp.USB_DEVICE;

        self.select_endpoint(dp, 0);
        // Enable EP0 as control endpoint, 64 bytes
        usb.ueconx.write(|w| w.epen().set_bit());
        usb.uecfg0x.write(|w| w.eptype().bits(0b00));
        usb.uecfg1x.write(|w| w.epsize().bits(0b011).alloc().set_bit());
    }

    fn configure_ep1(&self, dp: &Peripherals) {
        let usb = &dp.USB_DEVICE;

        self.select_endpoint(dp, 1);
        usb.ueconx.write(|w| w.epen().set_bit());
        // Interrupt IN endpoint
        usb.uecfg0x
            .write(|w| w.eptype().bits(0b11).epdir().set_bit());
        usb.uecfg1x.write(|w| w.epsize().bits(0b000).alloc().set_bit());
    }

    fn select_endpoint(&self, dp: &Peripherals, ep: u8) {
        dp.USB_DEVICE
            .uenum
            .write(|w| w.bits(ep & 0x07));
    }

    fn handle_setup(&mut self, dp: &Peripherals) {
        let usb = &dp.USB_DEVICE;

        // Read 8-byte SETUP packet
        let bm_request_type = usb.uedatx.read().bits();
        let b_request = usb.uedatx.read().bits();
        let w_value_l = usb.uedatx.read().bits();
        let w_value_h = usb.uedatx.read().bits();
        let w_index_l = usb.uedatx.read().bits();
        let _w_index_h = usb.uedatx.read().bits();
        let w_length_l = usb.uedatx.read().bits();
        let w_length_h = usb.uedatx.read().bits();

        // Acknowledge SETUP
        usb.ueintx.modify(|_, w| w.rxstpi().clear_bit());

        let w_length = (w_length_h as u16) << 8 | w_length_l as u16;
        let _ = w_index_l; // Used for some requests

        match (bm_request_type, b_request) {
            // GET_DESCRIPTOR
            (0x80, 0x06) => {
                let desc_type = w_value_h;
                let desc_index = w_value_l;
                match desc_type {
                    1 => self.send_descriptor(dp, &DEVICE_DESCRIPTOR, w_length),
                    2 => self.send_descriptor(dp, &CONFIG_DESCRIPTOR, w_length),
                    3 => {
                        // String descriptor
                        match desc_index {
                            0 => self.send_descriptor(dp, &STRING_DESC_0, w_length),
                            1 => self.send_descriptor(dp, &STRING_DESC_1, w_length),
                            2 => self.send_descriptor(dp, &STRING_DESC_2, w_length),
                            _ => self.stall(dp),
                        }
                    }
                    _ => self.stall(dp),
                }
            }

            // SET_ADDRESS
            (0x00, 0x05) => {
                // Send ZLP first, then set address
                usb.ueintx.modify(|_, w| w.txini().clear_bit());
                while usb.ueintx.read().txini().bit_is_clear() {}
                usb.udaddr
                    .write(|w| w.uadd().bits(w_value_l & 0x7F).adden().set_bit());
            }

            // SET_CONFIGURATION
            (0x00, 0x09) => {
                // Send ZLP
                usb.ueintx.modify(|_, w| w.txini().clear_bit());
                self.configure_ep1(dp);
                self.configured = true;
            }

            // GET_CONFIGURATION
            (0x80, 0x08) => {
                while usb.ueintx.read().txini().bit_is_clear() {}
                usb.uedatx
                    .write(|w| w.bits(if self.configured { 1 } else { 0 }));
                usb.ueintx.modify(|_, w| w.txini().clear_bit());
            }

            // HID GET_DESCRIPTOR (interface-level)
            (0x81, 0x06) => {
                let desc_type = w_value_h;
                match desc_type {
                    0x22 => self.send_descriptor(dp, &HID_REPORT_DESCRIPTOR, w_length),
                    _ => self.stall(dp),
                }
            }

            // HID SET_IDLE
            (0x21, 0x0A) => {
                // Send ZLP
                usb.ueintx.modify(|_, w| w.txini().clear_bit());
            }

            // HID SET_PROTOCOL
            (0x21, 0x0B) => {
                // Send ZLP
                usb.ueintx.modify(|_, w| w.txini().clear_bit());
            }

            // Vendor request: jump to bootloader
            (0x40, 0xFF) => {
                usb.ueintx.modify(|_, w| w.txini().clear_bit());
                jump_to_bootloader(dp);
            }

            _ => {
                self.stall(dp);
            }
        }
    }

    fn send_descriptor(&self, dp: &Peripherals, desc: &[u8], max_length: u16) {
        let usb = &dp.USB_DEVICE;
        let len = core::cmp::min(desc.len(), max_length as usize);
        let mut sent = 0;

        while sent < len {
            while usb.ueintx.read().txini().bit_is_clear() {}

            let chunk_end = core::cmp::min(sent + EP0_SIZE as usize, len);
            for &byte in &desc[sent..chunk_end] {
                usb.uedatx.write(|w| w.bits(byte));
            }

            usb.ueintx.modify(|_, w| w.txini().clear_bit());
            sent = chunk_end;
        }

        // Wait for status stage (host sends ZLP)
        while usb.ueintx.read().rxouti().bit_is_clear() {}
        usb.ueintx.modify(|_, w| w.rxouti().clear_bit());
    }

    fn stall(&self, dp: &Peripherals) {
        dp.USB_DEVICE
            .ueconx
            .modify(|_, w| w.stallrq().set_bit());
    }
}

/// Disable all peripherals and jump to the HalfKay bootloader at 0x7E00.
fn jump_to_bootloader(dp: &Peripherals) -> ! {
    // Disable interrupts
    avr_device::interrupt::disable();

    // Disconnect USB
    dp.USB_DEVICE.udcon.write(|w| w.detach().set_bit());
    dp.USB_DEVICE.usbcon.write(|w| w.frzclk().set_bit());

    // Short delay for host to notice disconnect
    for _ in 0..20000u16 {
        unsafe { core::arch::asm!("nop") };
    }

    // Disable peripherals
    dp.EXINT.eimsk.write(|w| w.bits(0));
    dp.SPI.spcr.write(|w| unsafe { w.bits(0) });
    dp.AC.acsr.write(|w| unsafe { w.bits(0) });
    dp.EEPROM.eecr.write(|w| unsafe { w.bits(0) });
    dp.ADC.adcsra.write(|w| unsafe { w.bits(0) });
    dp.TC0.timsk0.write(|w| unsafe { w.bits(0) });
    dp.TC1.timsk1.write(|w| unsafe { w.bits(0) });
    dp.TC3.timsk3.write(|w| unsafe { w.bits(0) });
    dp.TC4.timsk4.write(|w| unsafe { w.bits(0) });
    dp.USART1.ucsr1b.write(|w| unsafe { w.bits(0) });
    dp.TWI.twcr.write(|w| unsafe { w.bits(0) });

    // Reset all port directions and values
    dp.PORTB.ddrb.write(|w| unsafe { w.bits(0) });
    dp.PORTB.portb.write(|w| unsafe { w.bits(0) });
    dp.PORTC.ddrc.write(|w| unsafe { w.bits(0) });
    dp.PORTC.portc.write(|w| unsafe { w.bits(0) });
    dp.PORTD.ddrd.write(|w| unsafe { w.bits(0) });
    dp.PORTD.portd.write(|w| unsafe { w.bits(0) });
    dp.PORTE.ddre.write(|w| unsafe { w.bits(0) });
    dp.PORTE.porte.write(|w| unsafe { w.bits(0) });
    dp.PORTF.ddrf.write(|w| unsafe { w.bits(0) });
    dp.PORTF.portf.write(|w| unsafe { w.bits(0) });

    // Jump to bootloader
    unsafe { core::arch::asm!("jmp 0x7E00", options(noreturn)) }
}
