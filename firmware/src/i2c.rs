//! MCP23018 I2C driver for the ErgoDox right half.
//!
//! The right half of the ErgoDox uses an MCP23018 I/O expander connected via I2C.
//! GPIOA pins are used as row outputs (directly driving rows low) and
//! GPIOB pins are used as column inputs with internal pull-ups.

use avr_device::atmega32u4::TWI;

/// MCP23018 I2C address (all address pins grounded).
const MCP23018_ADDR: u8 = 0x20;

// MCP23018 register addresses (IOCON.BANK = 0, default)
const IODIRA: u8 = 0x00; // I/O direction register A (rows)
const IODIRB: u8 = 0x01; // I/O direction register B (columns)
const GPPUB: u8 = 0x0D; // Pull-up resistor register B
const GPIOA: u8 = 0x12; // Port A register
const GPIOB: u8 = 0x13; // Port B register

/// TWI (I2C) clock prescaler and bit rate for ~100kHz at 16MHz CPU.
/// SCL freq = CPU_FREQ / (16 + 2 * TWBR * prescaler)
/// 100kHz = 16MHz / (16 + 2 * 72 * 1) => TWBR = 72
const TWBR_VALUE: u8 = 72;

/// TWI status codes
const TW_START: u8 = 0x08;
const TW_REP_START: u8 = 0x10;
const TW_MT_SLA_ACK: u8 = 0x18;
const TW_MT_DATA_ACK: u8 = 0x28;
const TW_MR_SLA_ACK: u8 = 0x40;
const TW_MR_DATA_NACK: u8 = 0x58;

pub struct Mcp23018 {
    initialized: bool,
}

impl Mcp23018 {
    pub const fn new() -> Self {
        Self { initialized: false }
    }

    /// Initialize the TWI hardware and configure the MCP23018.
    pub fn init(&mut self, twi: &TWI) {
        // Set TWI bit rate
        twi.twbr.write(|w| unsafe { w.bits(TWBR_VALUE) });
        // Prescaler = 1 (TWPS = 0)
        twi.twsr.write(|w| w.twps().prescaler_1());
        // Enable TWI
        twi.twcr.write(|w| w.twen().set_bit());

        // Configure MCP23018
        if self.configure(twi).is_ok() {
            self.initialized = true;
        }
    }

    /// Configure MCP23018 I/O direction and pull-ups.
    fn configure(&self, twi: &TWI) -> Result<(), ()> {
        // IODIRA = 0x00: all pins output (rows)
        self.write_register(twi, IODIRA, 0x00)?;
        // IODIRB = 0x7F: pins 0-6 input (columns), pin 7 unused
        self.write_register(twi, IODIRB, 0x7F)?;
        // GPPUB = 0x7F: enable pull-ups on column inputs
        self.write_register(twi, GPPUB, 0x7F)?;
        // Drive all rows high initially (inactive)
        self.write_register(twi, GPIOA, 0xFF)?;
        Ok(())
    }

    /// Try to re-initialize if the MCP23018 was not detected.
    pub fn try_reinit(&mut self, twi: &TWI) {
        if !self.initialized {
            if self.configure(twi).is_ok() {
                self.initialized = true;
            }
        }
    }

    /// Read the column states for a given row on the right half.
    /// Returns 7 bits of column data (active low), or 0x7F if not initialized.
    pub fn read_row(&self, twi: &TWI, row: u8) -> u8 {
        if !self.initialized {
            return 0x7F; // All keys up
        }

        // Drive the target row low, all others high
        let row_bits = !(1u8 << row) & 0x3F; // Only 6 rows
        if self.write_register(twi, GPIOA, row_bits).is_err() {
            return 0x7F;
        }

        // Small delay for signal settling
        tiny_delay();

        // Read column inputs
        match self.read_register(twi, GPIOB) {
            Ok(val) => val & 0x7F,
            Err(()) => 0x7F,
        }
    }

    fn write_register(&self, twi: &TWI, reg: u8, value: u8) -> Result<(), ()> {
        self.i2c_start(twi)?;
        self.i2c_write(twi, (MCP23018_ADDR << 1) | 0)?; // Write mode
        self.i2c_write(twi, reg)?;
        self.i2c_write(twi, value)?;
        self.i2c_stop(twi);
        Ok(())
    }

    fn read_register(&self, twi: &TWI, reg: u8) -> Result<u8, ()> {
        // Write register address
        self.i2c_start(twi)?;
        self.i2c_write(twi, (MCP23018_ADDR << 1) | 0)?;
        self.i2c_write(twi, reg)?;

        // Repeated start for read
        self.i2c_start(twi)?;
        self.i2c_write(twi, (MCP23018_ADDR << 1) | 1)?; // Read mode
        let data = self.i2c_read_nack(twi)?;
        self.i2c_stop(twi);
        Ok(data)
    }

    fn i2c_start(&self, twi: &TWI) -> Result<(), ()> {
        twi.twcr
            .write(|w| w.twint().set_bit().twsta().set_bit().twen().set_bit());
        self.wait_twint(twi);
        let status = twi.twsr.read().tws().bits();
        if status != TW_START && status != TW_REP_START {
            return Err(());
        }
        Ok(())
    }

    fn i2c_write(&self, twi: &TWI, data: u8) -> Result<(), ()> {
        twi.twdr.write(|w| unsafe { w.bits(data) });
        twi.twcr.write(|w| w.twint().set_bit().twen().set_bit());
        self.wait_twint(twi);
        let status = twi.twsr.read().tws().bits();
        if status != TW_MT_SLA_ACK && status != TW_MT_DATA_ACK && status != TW_MR_SLA_ACK {
            return Err(());
        }
        Ok(())
    }

    fn i2c_read_nack(&self, twi: &TWI) -> Result<u8, ()> {
        // Read one byte with NACK (last byte)
        twi.twcr.write(|w| w.twint().set_bit().twen().set_bit());
        self.wait_twint(twi);
        let status = twi.twsr.read().tws().bits();
        if status != TW_MR_DATA_NACK {
            return Err(());
        }
        Ok(twi.twdr.read().bits())
    }

    fn i2c_stop(&self, twi: &TWI) {
        twi.twcr
            .write(|w| w.twint().set_bit().twsto().set_bit().twen().set_bit());
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
