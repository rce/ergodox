//! Key matrix scanning for the ErgoDox keyboard.
//!
//! The ErgoDox has a 6×14 matrix split across two halves:
//! - Left half: directly wired to Teensy GPIO pins
//! - Right half: connected via MCP23018 I2C I/O expander
//!
//! Matrix layout: 6 rows × 7 columns per half (14 columns total).

use avr_device::atmega32u4::Peripherals;

use crate::i2c::Mcp23018;

/// Number of rows in the matrix.
pub const ROWS: usize = 6;
/// Number of columns per half.
pub const COLS_PER_HALF: usize = 7;
/// Total number of columns.
pub const COLS: usize = COLS_PER_HALF * 2;

/// Complete matrix state: one byte per row, lower 14 bits represent columns.
/// Bit = 0 means key pressed (active low), bit = 1 means released.
pub type MatrixState = [[bool; COLS]; ROWS];

/// Initialize the left-half GPIO pins for matrix scanning.
///
/// Left half pin mapping on Teensy 2.0 (ATmega32U4):
///   Rows (active-low outputs): PB0, PB1, PB2, PB3, PD2, PD3
///   Columns (inputs w/ pull-up): PF0, PF1, PF4, PF5, PF6, PF7, PB6
pub fn init_left(dp: &Peripherals) {
    let portb = &dp.PORTB;
    let portd = &dp.PORTD;
    let portf = &dp.PORTF;

    // Row pins as outputs, initially high (inactive)
    // PB0-PB3: set as output, drive high
    portb.ddrb.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x0F) // PB0-PB3 output
    });
    portb.portb.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x0F) // PB0-PB3 high
    });

    // PD2-PD3: set as output, drive high
    portd.ddrd.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x0C) // PD2, PD3 output
    });
    portd.portd.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x0C) // PD2, PD3 high
    });

    // Column pins as inputs with pull-ups
    // PF0, PF1, PF4-PF7: input with pull-up
    portf.ddrf.modify(|r, w| unsafe {
        w.bits(r.bits() & !(0x03 | 0xF0)) // Clear direction bits
    });
    portf.portf.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x03 | 0xF0) // Enable pull-ups
    });

    // PB6: input with pull-up
    portb.ddrb.modify(|r, w| unsafe {
        w.bits(r.bits() & !0x40) // PB6 input
    });
    portb.portb.modify(|r, w| unsafe {
        w.bits(r.bits() | 0x40) // PB6 pull-up
    });
}

/// Drive a specific left-half row low. All other rows high.
fn drive_left_row(dp: &Peripherals, row: usize) {
    let portb = &dp.PORTB;
    let portd = &dp.PORTD;

    // Start with all rows high
    portb.portb.modify(|r, w| unsafe { w.bits(r.bits() | 0x0F) });
    portd.portd.modify(|r, w| unsafe { w.bits(r.bits() | 0x0C) });

    // Drive the selected row low
    match row {
        0 => portb.portb.modify(|r, w| unsafe { w.bits(r.bits() & !0x01) }), // PB0
        1 => portb.portb.modify(|r, w| unsafe { w.bits(r.bits() & !0x02) }), // PB1
        2 => portb.portb.modify(|r, w| unsafe { w.bits(r.bits() & !0x04) }), // PB2
        3 => portb.portb.modify(|r, w| unsafe { w.bits(r.bits() & !0x08) }), // PB3
        4 => portd.portd.modify(|r, w| unsafe { w.bits(r.bits() & !0x04) }), // PD2
        5 => portd.portd.modify(|r, w| unsafe { w.bits(r.bits() & !0x08) }), // PD3
        _ => {}
    }
}

/// Read the left-half column pins. Returns 7 bits, one per column (active low).
fn read_left_columns(dp: &Peripherals) -> u8 {
    let pinf = dp.PORTF.pinf.read().bits();
    let pinb = dp.PORTB.pinb.read().bits();

    // Map physical pins to column indices:
    // Col 0 = PF0, Col 1 = PF1, Col 2 = PF4, Col 3 = PF5,
    // Col 4 = PF6, Col 5 = PF7, Col 6 = PB6
    let col0 = (pinf >> 0) & 1;
    let col1 = (pinf >> 1) & 1;
    let col2 = (pinf >> 4) & 1;
    let col3 = (pinf >> 5) & 1;
    let col4 = (pinf >> 6) & 1;
    let col5 = (pinf >> 7) & 1;
    let col6 = (pinb >> 6) & 1;

    col0 | (col1 << 1) | (col2 << 2) | (col3 << 3) | (col4 << 4) | (col5 << 5) | (col6 << 6)
}

/// Scan the entire matrix (left + right halves).
pub fn scan(dp: &Peripherals, mcp: &Mcp23018) -> MatrixState {
    let twi = &dp.TWI;
    let mut state = [[true; COLS]; ROWS]; // true = not pressed

    for row in 0..ROWS {
        // Scan left half
        drive_left_row(dp, row);
        tiny_delay();
        let left_cols = read_left_columns(dp);

        for col in 0..COLS_PER_HALF {
            // Active low: bit clear = key pressed
            state[row][col] = (left_cols >> col) & 1 != 0;
        }

        // Scan right half via MCP23018
        let right_cols = mcp.read_row(twi, row as u8);

        for col in 0..COLS_PER_HALF {
            state[row][COLS_PER_HALF + col] = (right_cols >> col) & 1 != 0;
        }
    }

    // Deactivate all rows
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
