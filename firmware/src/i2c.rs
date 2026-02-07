//! MCP23018 I2C driver for the ErgoDox left half.
//!
//! The left half of the ErgoDox uses an MCP23018 I/O expander connected
//! to the Teensy via I2C over the TRRS cable (SCL=PD0, SDA=PD1).
//!
//! # Left half pin mapping (MCP23018)
//!
//! I2C address: 0x20 (A0-A2 tied to GND on PCB).
//!
//! GPIOA — column outputs (active-low, one driven at a time):
//!   GPA0 → col 0    IODIRA = 0x00 (all output)
//!   GPA1 → col 1
//!   GPA2 → col 2
//!   GPA3 → col 3
//!   GPA4 → col 4
//!   GPA5 → col 5
//!   GPA6 → col 6
//!   GPA7 → (unused)
//!
//! GPIOB — row inputs (internal pull-ups enabled):
//!   GPB0 → row 0    IODIRB = 0xFF (all input), GPPUB = 0xFF (pull-ups)
//!   GPB1 → row 1
//!   GPB2 → row 2
//!   GPB3 → row 3
//!   GPB4 → row 4
//!   GPB5 → row 5
//!   GPB6 → (unused)
//!   GPB7 → (unused)

use avr_device::atmega32u4::TWI;

/// MCP23018 I2C address. A0-A2 pins are tied to GND on the ErgoDox PCB.
const MCP23018_BASE_ADDR: u8 = 0x20;

// MCP23018 register addresses (IOCON.BANK = 0, the power-on default)
const IODIRA: u8 = 0x00; // I/O direction A: 0=output, 1=input
const IODIRB: u8 = 0x01; // I/O direction B: 0=output, 1=input
const GPPUB: u8 = 0x0D;  // Pull-up enable B: 1=enabled
const GPIOA: u8 = 0x12;  // Port A data (write to drive columns)
const GPIOB: u8 = 0x13;  // Port B data (read to get row states)

/// TWI (I2C) clock prescaler and bit rate for ~100kHz at 16MHz CPU.
/// SCL freq = CPU_FREQ / (16 + 2 * TWBR * prescaler)
/// 100kHz = 16MHz / (16 + 2 * 72 * 1) => TWBR = 72
const TWBR_VALUE: u8 = 72;

/// TWI status codes (raw TWSR values with prescaler bits masked)
const TW_START: u8 = 0x08;
const TW_REP_START: u8 = 0x10;
const TW_MT_SLA_ACK: u8 = 0x18;
const TW_MT_DATA_ACK: u8 = 0x28;
const TW_MR_SLA_ACK: u8 = 0x40;
const TW_MR_DATA_NACK: u8 = 0x58;

pub struct Mcp23018 {
    addr: u8,
    initialized: bool,
    errors: u8,
}

/// Read the TWI status register, masking out the prescaler bits.
#[inline(always)]
fn twi_status(twi: &TWI) -> u8 {
    twi.twsr.read().bits() & 0xF8
}

impl Mcp23018 {
    pub const fn new() -> Self {
        Self {
            addr: MCP23018_BASE_ADDR,
            initialized: false,
            errors: 0,
        }
    }

    /// Initialize the TWI hardware, scan for the MCP23018, and configure it.
    /// Returns the detected address (0x20-0x27), or None if not found.
    pub fn init(&mut self, twi: &TWI) -> Option<u8> {
        // Set TWI bit rate
        twi.twbr.write(|w| w.bits(TWBR_VALUE));
        // Prescaler = 1 (TWPS = 0)
        twi.twsr.write(|w| w.twps().prescaler_1());
        // Enable TWI
        twi.twcr.write(|w| w.twen().set_bit());

        // Scan all possible MCP23018 addresses (0x20-0x27)
        for offset in 0..8u8 {
            let candidate = MCP23018_BASE_ADDR + offset;
            self.addr = candidate;
            if self.probe(twi) {
                if self.configure(twi).is_ok() {
                    self.initialized = true;
                    return Some(candidate);
                }
            }
        }
        None
    }

    /// Probe whether a device ACKs at the current address.
    /// Always sends STOP to leave the bus clean for the next attempt.
    fn probe(&self, twi: &TWI) -> bool {
        let ok = self.i2c_start(twi).is_ok()
            && self.i2c_write(twi, (self.addr << 1) | 0).is_ok();
        self.i2c_stop(twi);
        // Wait for STOP to complete
        let mut timeout: u16 = 0xFFFF;
        while twi.twcr.read().twsto().bit_is_set() {
            timeout = timeout.wrapping_sub(1);
            if timeout == 0 { break; }
        }
        ok
    }

    /// Return the TWI status byte from attempting a START + address write.
    /// Used for diagnostics. Returns (start_status, addr_status) as raw TWSR values.
    pub fn debug_status(&self, twi: &TWI) -> (u8, u8) {
        // Attempt START
        twi.twcr.write(|w| w.twint().set_bit().twsta().set_bit().twen().set_bit());
        self.wait_twint(twi);
        let start_status = twi_status(twi);

        // Attempt SLA+W
        twi.twdr.write(|w| w.bits((self.addr << 1) | 0));
        twi.twcr.write(|w| w.twint().set_bit().twen().set_bit());
        self.wait_twint(twi);
        let addr_status = twi_status(twi);

        // Always STOP
        twi.twcr.write(|w| w.twint().set_bit().twsto().set_bit().twen().set_bit());
        let mut timeout: u16 = 0xFFFF;
        while twi.twcr.read().twsto().bit_is_set() {
            timeout = timeout.wrapping_sub(1);
            if timeout == 0 { break; }
        }

        (start_status, addr_status)
    }

    /// Configure MCP23018 I/O direction and pull-ups.
    /// Original ErgoDox wiring: GPIOA = columns (outputs), GPIOB = rows (inputs).
    fn configure(&self, twi: &TWI) -> Result<(), ()> {
        // IODIRA = 0x00: all pins output (drive columns)
        self.write_register(twi, IODIRA, 0x00)?;
        // IODIRB = 0xFF: all pins input (read rows)
        self.write_register(twi, IODIRB, 0xFF)?;
        // GPPUB = 0xFF: enable pull-ups on row inputs
        self.write_register(twi, GPPUB, 0xFF)?;
        // Drive all column outputs high initially (inactive)
        self.write_register(twi, GPIOA, 0xFF)?;
        Ok(())
    }

    /// Whether the MCP23018 is currently initialized and scanning.
    pub fn is_ok(&self) -> bool {
        self.initialized
    }

    /// Try to re-initialize if the MCP23018 was not detected.
    pub fn try_reinit(&mut self, twi: &TWI) {
        if !self.initialized {
            self.errors = 0;
            if self.configure(twi).is_ok() {
                self.initialized = true;
            }
        }
    }

    /// Drive one column low on GPIOA and read rows from GPIOB.
    /// Returns 8 bits of row data (active low), or 0xFF if not initialized/errored.
    pub fn scan_column(&mut self, twi: &TWI, col: u8) -> u8 {
        if !self.initialized {
            return 0xFF; // All keys up
        }

        // Drive the target column low on GPIOA, all others high
        if self.write_register(twi, GPIOA, !(1u8 << col)).is_err() {
            self.mark_error();
            return 0xFF;
        }

        // Small delay for signal settling
        tiny_delay();

        // Read row inputs from GPIOB
        match self.read_register(twi, GPIOB) {
            Ok(val) => {
                self.errors = 0;
                val
            }
            Err(()) => {
                self.mark_error();
                0xFF
            }
        }
    }

    /// After 10 consecutive I2C errors, disable scanning to avoid phantom keys.
    fn mark_error(&mut self) {
        self.errors = self.errors.saturating_add(1);
        if self.errors >= 10 {
            self.initialized = false;
        }
    }

    /// Deactivate all column outputs (set high).
    pub fn deactivate(&self, twi: &TWI) {
        if self.initialized {
            let _ = self.write_register(twi, GPIOA, 0xFF);
        }
    }

    fn write_register(&self, twi: &TWI, reg: u8, value: u8) -> Result<(), ()> {
        self.i2c_start(twi)?;
        self.i2c_write(twi, (self.addr << 1) | 0)?; // Write mode
        self.i2c_write(twi, reg)?;
        self.i2c_write(twi, value)?;
        self.i2c_stop(twi);
        Ok(())
    }

    fn read_register(&self, twi: &TWI, reg: u8) -> Result<u8, ()> {
        // Write register address
        self.i2c_start(twi)?;
        self.i2c_write(twi, (self.addr << 1) | 0)?;
        self.i2c_write(twi, reg)?;

        // Repeated start for read
        self.i2c_start(twi)?;
        self.i2c_write(twi, (self.addr << 1) | 1)?; // Read mode
        let data = self.i2c_read_nack(twi)?;
        self.i2c_stop(twi);
        Ok(data)
    }

    fn i2c_start(&self, twi: &TWI) -> Result<(), ()> {
        twi.twcr
            .write(|w| w.twint().set_bit().twsta().set_bit().twen().set_bit());
        self.wait_twint(twi);
        let status = twi_status(twi);
        if status != TW_START && status != TW_REP_START {
            return Err(());
        }
        Ok(())
    }

    fn i2c_write(&self, twi: &TWI, data: u8) -> Result<(), ()> {
        twi.twdr.write(|w| w.bits(data));
        twi.twcr.write(|w| w.twint().set_bit().twen().set_bit());
        self.wait_twint(twi);
        let status = twi_status(twi);
        if status != TW_MT_SLA_ACK && status != TW_MT_DATA_ACK && status != TW_MR_SLA_ACK {
            return Err(());
        }
        Ok(())
    }

    fn i2c_read_nack(&self, twi: &TWI) -> Result<u8, ()> {
        // Read one byte with NACK (last byte)
        twi.twcr.write(|w| w.twint().set_bit().twen().set_bit());
        self.wait_twint(twi);
        let status = twi_status(twi);
        if status != TW_MR_DATA_NACK {
            return Err(());
        }
        Ok(twi.twdr.read().bits())
    }

    fn i2c_stop(&self, twi: &TWI) {
        twi.twcr
            .write(|w| w.twint().set_bit().twsto().set_bit().twen().set_bit());
        // Wait for STOP to complete before allowing the next START
        let mut timeout: u16 = 0xFFFF;
        while twi.twcr.read().twsto().bit_is_set() {
            timeout = timeout.wrapping_sub(1);
            if timeout == 0 { break; }
        }
    }

    fn wait_twint(&self, twi: &TWI) {
        // Busy-wait for TWI interrupt flag with a timeout counter
        let mut timeout: u16 = 0xFFFF;
        while twi.twcr.read().twint().bit_is_clear() {
            timeout = timeout.wrapping_sub(1);
            if timeout == 0 {
                return;
            }
        }
    }
}

/// Very short delay (~10us) for I/O settling.
#[inline(always)]
fn tiny_delay() {
    for _ in 0..40u8 {
        unsafe { core::arch::asm!("nop") };
    }
}
