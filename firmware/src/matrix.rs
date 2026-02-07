//! Key matrix scanning for the ErgoDox keyboard.
//!
//! The ErgoDox has a 6×14 matrix split across two halves:
//! - Right half: directly wired to Teensy 2.0 GPIO pins
//! - Left half: connected via MCP23018 I2C I/O expander (see i2c.rs)
//!
//! Scanning drives one column LOW at a time and reads which rows are
//! pulled LOW through the key switch + diode. The result is stored as
//! `state[row][col]` with active-low convention (true = not pressed).

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

// ── Right half pin mapping (Teensy 2.0 / ATmega32U4) ────────────────
//
// Column drive pins — directly wired to matrix columns (active-low outputs):
//   Drive 0 → PB0  (col 7)       PORTB mask: 0x0F = PB0..PB3
//   Drive 1 → PB1  (col 8)       PORTD mask: 0x0C = PD2..PD3
//   Drive 2 → PB2  (col 9)
//   Drive 3 → PB3  (col 10)
//   Drive 4 → PD2  (col 11)
//   Drive 5 → PD3  (col 12)
//
// Row read pins — directly wired to matrix rows (inputs with pull-ups):
//   Read 0 → PF0  (row 0)        PORTF mask: 0xF3 = PF0,PF1,PF4..PF7
//   Read 1 → PF1  (row 1)        PORTB mask: 0x40 = PB6
//   Read 2 → PF4  (row 2)
//   Read 3 → PF5  (row 3)
//   Read 4 → PF6  (row 4)
//   Read 5 → PF7  (row 5)
//   Read 6 → PB6  (unused, no physical row 6)
//
// Other pins:
//   PD0 = I2C SCL (to left half via TRRS)
//   PD1 = I2C SDA (to left half via TRRS)
//   PD6 = onboard LED

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
/// Right half: 6 drive pins → 6 columns, 7 read pins → 6 rows (7th unused).
/// Left half: GPIOA drives 7 columns, GPIOB reads 6 rows.
/// Both stored as state[row][col] with active-low convention.
pub fn scan(dp: &Peripherals, mcp: &mut Mcp23018) -> MatrixState {
    let twi = &dp.TWI;
    let mut state = [[true; COLS]; ROWS]; // true = not pressed

    // Right half (Teensy GPIO): 6 columns via drive pins
    for col in 0..ROWS {
        drive_pin(dp, col);
        tiny_delay();
        let reads = read_pins(dp);

        for row in 0..ROWS {
            // Drive pin = column, read pin = row
            // Right half columns offset by COLS_PER_HALF
            state[row][COLS_PER_HALF + col] = (reads >> row) & 1 != 0;
        }
    }

    // Deactivate right half drive pins
    let portb = &dp.PORTB;
    let portd = &dp.PORTD;
    portb.portb.modify(|r, w| unsafe { w.bits(r.bits() | 0x0F) });
    portd.portd.modify(|r, w| unsafe { w.bits(r.bits() | 0x0C) });

    // Left half (MCP23018): 7 columns via GPIOA
    for col in 0..COLS_PER_HALF {
        let reads = mcp.scan_column(twi, col as u8);

        for row in 0..ROWS {
            // GPIOB bit = row, GPIOA pin = column
            state[row][col] = (reads >> row) & 1 != 0;
        }
    }
    mcp.deactivate(twi);

    state
}

/// Short delay for pin settling (~5us at 16MHz).
#[inline(always)]
fn tiny_delay() {
    for _ in 0..20u8 {
        unsafe { core::arch::asm!("nop") };
    }
}
