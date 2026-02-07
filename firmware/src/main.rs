//! ErgoDox keyboard firmware for ATmega32U4 (Teensy 2.0).
//!
//! This is a minimal but functional firmware implementing:
//! - Matrix scanning for both halves (left via GPIO, right via MCP23018 I2C)
//! - Per-key debouncing
//! - Two-layer keymap with momentary layer switching
//! - USB HID keyboard reports (6KRO)

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(asm_experimental_arch)]

mod debounce;
mod hid;
mod i2c;
mod keymap;
mod matrix;

use avr_device::atmega32u4::Peripherals;

use debounce::Debouncer;
use hid::UsbKeyboard;
use i2c::Mcp23018;

/// Panic handler — on AVR we just loop forever.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// Main entry point.
#[no_mangle]
pub extern "C" fn main() -> ! {
    let dp = unsafe { Peripherals::steal() };

    // Configure system clock (should already be 16MHz from Teensy bootloader fuses)
    // Disable clock prescaler (CLKPR)
    dp.CPU.clkpr.write(|w| w.clkpce().set_bit());
    dp.CPU.clkpr.write(|w| unsafe { w.bits(0) }); // Prescaler = 1

    // Initialize LED on PD6 (Teensy on-board LED) for diagnostics
    dp.PORTD.ddrd.modify(|r, w| unsafe { w.bits(r.bits() | 0x40) }); // PD6 output

    // Initialize right-half matrix GPIO (Teensy side)
    matrix::init_gpio(&dp);

    // Initialize I2C and MCP23018 for right half
    let mut mcp = Mcp23018::new();
    mcp.init(&dp.TWI);

    // Initialize USB
    let mut usb = UsbKeyboard::new();
    usb.init(&dp);

    // Initialize debouncer
    let mut debouncer = Debouncer::new();

    // LED on to indicate firmware is running
    dp.PORTD
        .portd
        .modify(|r, w| unsafe { w.bits(r.bits() | 0x40) });

    // Re-init counter for MCP23018 (retry every ~1 second)
    let mut reinit_counter: u16 = 0;

    loop {
        // Poll USB (handle enumeration, control requests)
        usb.poll(&dp);

        // Periodically attempt to re-initialize MCP23018 if it wasn't found
        reinit_counter = reinit_counter.wrapping_add(1);
        if reinit_counter == 0 {
            mcp.try_reinit(&dp.TWI);
        }

        // Scan key matrix
        let raw_state = matrix::scan(&dp, &mcp);

        // Debounce
        let debounced = debouncer.update(&raw_state);

        // Resolve active layer
        let layer = keymap::resolve_layer(debounced);

        // Build HID report
        let report = hid::build_report(debounced, layer);

        // Send report if changed
        usb.send_report(&dp, &report);

        // ~1ms delay between scans
        delay_ms(1);
    }
}

/// Busy-wait delay in milliseconds (approximate, at 16MHz).
fn delay_ms(ms: u16) {
    for _ in 0..ms {
        // ~1ms at 16MHz: 16000 cycles / 4 cycles per loop iteration
        for _ in 0..4000u16 {
            unsafe { core::arch::asm!("nop") };
        }
    }
}
