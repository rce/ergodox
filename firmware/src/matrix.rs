//! Key matrix scanning for the ErgoDox keyboard.
//!
//! The ErgoDox has a 6×14 matrix split across two halves:
//! - Right half: directly wired to Teensy GPIO pins
//! - Left half: connected via MCP23018 I2C I/O expander
//!
//! On the ErgoDox PCB, the 6 drive pins (PB0-PB3, PD2, PD3) connect to
//! physical columns, and the 7 read pins (PF0-PB6) connect to physical
//! rows. We transpose when storing into the state matrix to get the
//! correct [row][col] layout.

use avr_device::atmega32u4::Peripherals;

use crate::i2c::Mcp23018;

/// Number of rows in the matrix.
pub const ROWS: usize = 6;
/// Number of columns per half.
pub const COLS_PER_HALF: usize = 7;
/// Total number of columns.
pub const COLS: usize = COLS_PER_HALF * 2;

/// Complete matrix state.
pub type MatrixState = [[bool; COLS]; ROWS];

/// Initialize the Teensy GPIO pins for matrix scanning (right half).
///
/// Pin mapping on Teensy 2.0 (ATmega32U4):
///   Drive pins (active-low outputs): PB0, PB1, PB2, PB3, PD2, PD3
///   Read pins (inputs w/ pull-up):   PF0, PF1, PF4, PF5, PF6, PF7, PB6
pub fn init_gpio(dp: &Peripherals) {
    let portb = &dp.PORTB;
    let portd = &dp.PORTD;
    let portf = &dp.PORTF;

    // Drive pins as outputs, initially high (inactive)
    // PB0-PB3: output, drive high
    portb.ddrb.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x0F)
    });
    portb.portb.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x0F)
    });
    // PD2-PD3: output, drive high
    portd.ddrd.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x0C)
    });
    portd.portd.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x0C)
    });

    // Read pins as inputs with pull-ups
    // PF0, PF1, PF4-PF7: input with pull-up
    portf.ddrf.modify(|r, w| unsafe {
        w.bits(r.bits() & !(0x03 | 0xF0))
    });
    portf.portf.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x03 | 0xF0)
    });
    // PB6: input with pull-up
    portb.ddrb.modify(|r, w| unsafe {
        w.bits(r.bits() & !0x40)
    });
    portb.portb.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x40)
    });
}

/// Drive a specific pin low. All other drive pins high.
fn drive_pin(dp: &Peripherals, index: usize) {
    let portb = &dp.PORTB;
    let portd = &dp.PORTD;

    // All drive pins high first
    portb.portb.modify(|r, w| unsafe { w.bits(r.bits() | 0x0F) });
    portd.portd.modify(|r, w| unsafe { w.bits(r.bits() | 0x0C) });

    // Drive selected pin low
    match index {
        0 => portb.portb.modify(|r, w| unsafe { w.bits(r.bits() & !0x01) }), // PB0
        1 => portb.portb.modify(|r, w| unsafe { w.bits(r.bits() & !0x02) }), // PB1
        2 => portb.portb.modify(|r, w| unsafe { w.bits(r.bits() & !0x04) }), // PB2
        3 => portb.portb.modify(|r, w| unsafe { w.bits(r.bits() & !0x08) }), // PB3
        4 => portd.portd.modify(|r, w| unsafe { w.bits(r.bits() & !0x04) }), // PD2
        5 => portd.portd.modify(|r, w| unsafe { w.bits(r.bits() & !0x08) }), // PD3
        _ => {}
    }
}

/// Read the 7 input pins. Returns 7 bits (active low).
fn read_pins(dp: &Peripherals) -> u8 {
    let pinf = dp.PORTF.pinf.read().bits();
    let pinb = dp.PORTB.pinb.read().bits();

    // Pin 0 = PF0, Pin 1 = PF1, Pin 2 = PF4, Pin 3 = PF5,
    // Pin 4 = PF6, Pin 5 = PF7, Pin 6 = PB6
    let p0 = (pinf >> 0) & 1;
    let p1 = (pinf >> 1) & 1;
    let p2 = (pinf >> 4) & 1;
    let p3 = (pinf >> 5) & 1;
    let p4 = (pinf >> 6) & 1;
    let p5 = (pinf >> 7) & 1;
    let p6 = (pinb >> 6) & 1;

    p0 | (p1 << 1) | (p2 << 2) | (p3 << 3) | (p4 << 4) | (p5 << 5) | (p6 << 6)
}

/// Scan the entire matrix (right half via GPIO, left half via MCP23018).
///
/// The ErgoDox PCB wires drive pins to physical columns and read pins to
/// physical rows, so we transpose: state[read][drive + half_offset].
pub fn scan(dp: &Peripherals, mcp: &Mcp23018) -> MatrixState {
    let twi = &dp.TWI;
    let mut state = [[true; COLS]; ROWS]; // true = not pressed

    for drive in 0..ROWS {
        // Scan right half (Teensy GPIO)
        drive_pin(dp, drive);
        tiny_delay();
        let reads = read_pins(dp);

        for read in 0..COLS_PER_HALF {
            if read < ROWS {
                // Transposed: read pin = physical row, drive pin = physical column
                // GPIO is the right half → offset by COLS_PER_HALF
                state[read][COLS_PER_HALF + drive] = (reads >> read) & 1 != 0;
            }
        }

        // Scan left half (MCP23018)
        let mcp_reads = mcp.read_row(twi, drive as u8);

        for read in 0..COLS_PER_HALF {
            if read < ROWS {
                // Transposed: read = physical row, drive = physical column
                // MCP is the left half → no offset
                state[read][drive] = (mcp_reads >> read) & 1 != 0;
            }
        }
    }

    // Deactivate all drive pins
    let portb = &dp.PORTB;
    let portd = &dp.PORTD;
    portb.portb.modify(|r, w| unsafe { w.bits(r.bits() | 0x0F) });
    portd.portd.modify(|r, w| unsafe { w.bits(r.bits() | 0x0C) });

    state
}

/// Short delay for pin settling (~5us at 16MHz).
#[inline(always)]
fn tiny_delay() {
    for _ in 0..20u8 {
        unsafe { core::arch::asm!("nop") };
    }
}
