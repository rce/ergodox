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

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    let dp = unsafe { Peripherals::steal() };

    dp.CPU.clkpr.write(|w| w.clkpce().set_bit());
    dp.CPU.clkpr.write(|w| unsafe { w.bits(0) });

    // LED on PD6
    dp.PORTD.ddrd.modify(|r, w| unsafe { w.bits(r.bits() | 0x40) });

    // Init right-half GPIO
    matrix::init_gpio(&dp);

    // Init left half via I2C
    delay_ms(100);
    let mut mcp = Mcp23018::new();
    mcp.init(&dp.TWI);

    // Init USB
    let mut usb = UsbKeyboard::new();
    usb.init(&dp);

    let mut debouncer = Debouncer::new();

    // LED on
    dp.PORTD.portd.modify(|r, w| unsafe { w.bits(r.bits() | 0x40) });

    loop {
        usb.poll(&dp);

        let raw_state = matrix::scan(&dp, &mut mcp);
        let debounced = debouncer.update(&raw_state);
        let layer = keymap::resolve_layer(debounced);
        let report = hid::build_report(debounced, layer);
        usb.send_report(&dp, &report);

        // LED reflects MCP status: ON = working, OFF = errored out
        if mcp.is_ok() {
            dp.PORTD.portd.modify(|r, w| unsafe { w.bits(r.bits() | 0x40) });
        } else {
            dp.PORTD.portd.modify(|r, w| unsafe { w.bits(r.bits() & !0x40) });
        }

        delay_ms(1);
    }
}

fn delay_ms(ms: u16) {
    for _ in 0..ms {
        for _ in 0..4000u16 {
            unsafe { core::arch::asm!("nop") };
        }
    }
}
