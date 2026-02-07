# ErgoDox Keyboard Firmware

Custom firmware for the ErgoDox split mechanical keyboard, written in Rust.
Targets the Teensy 2.0 (ATmega32U4) with MCP23018 I/O expander on the left half.

PCB: designed by Dominic Beauchamp, revision 2012-08-02.

## Hardware

The ErgoDox is a split keyboard connected by a TRRS cable carrying I2C + power:

```
 Left half                TRRS cable              Right half
┌──────────────┐    ┌──────────────────┐    ┌──────────────┐
│  MCP23018    │◄───┤ SDA (PD1)        │───►│  Teensy 2.0  │
│  I/O expander│◄───┤ SCL (PD0)        │───►│  ATmega32U4  │
│              │◄───┤ VCC              │───►│              │
│              │◄───┤ GND              │───►│  USB to host │
└──────────────┘    └──────────────────┘    └──────────────┘
```

### Key matrix

6 rows x 14 columns (7 per half). Each key sits at a row/column intersection
with a diode. Scanning drives one column LOW at a time and reads which rows
are pulled LOW (= key pressed).

```
        Left half (cols 0-6)           Right half (cols 7-13)
       col0 col1 col2 col3 col4 col5 col6  col7 col8 col9 col10 col11 col12 col13
row 0:  =    1    2    3    4    5   Esc     -    6    7    8     9     0     ---
row 1: Tab   Q    W    E    R    T    [      ]    Y    U    I     O     P      \
row 2: Ctrl  A    S    D    F    G   ---    ---   H    J    K     L     ;      '
row 3: Shft  Z    X    C    V    B   Ly1    Ly1  N    M    ,     .     /    RShft
row 4:  `   Alt  Gui  ---  ---  ---  ---    --- ---  ---   ---  RAlt  ---    ---
row 5: ---  --- Bksp  Del  ---  ---  ---    --- ---  ---   Ent   Spc  ---    ---
```

### Right half — Teensy 2.0 pin assignments

Column drive pins (active-low outputs, accent one column at a time):

| Drive index | ATmega32U4 pin | Port/bit |
|-------------|----------------|----------|
| 0 (col 7)   | PB0            | PORTB.0  |
| 1 (col 8)   | PB1            | PORTB.1  |
| 2 (col 9)   | PB2            | PORTB.2  |
| 3 (col 10)  | PB3            | PORTB.3  |
| 4 (col 11)  | PD2            | PORTD.2  |
| 5 (col 12)  | PD3            | PORTD.3  |

Row read pins (inputs with internal pull-ups):

| Read index | ATmega32U4 pin | Port/bit | Matrix row |
|------------|----------------|----------|------------|
| 0          | PF0            | PINF.0   | row 0      |
| 1          | PF1            | PINF.1   | row 1      |
| 2          | PF4            | PINF.4   | row 2      |
| 3          | PF5            | PINF.5   | row 3      |
| 4          | PF6            | PINF.6   | row 4      |
| 5          | PF7            | PINF.7   | row 5      |
| 6          | PB6            | PINB.6   | (unused)   |

Other Teensy pins:

| Function    | Pin | Port/bit |
|-------------|-----|----------|
| I2C SCL     | PD0 | PORTD.0  |
| I2C SDA     | PD1 | PORTD.1  |
| Onboard LED | PD6 | PORTD.6  |
| USB D+/D-   | —   | built-in |

### Left half — MCP23018 pin assignments

I2C address: 0x20 (A0-A2 tied to GND).

GPIOA — column drive outputs (active-low, accent one column at a time):

| GPIOA pin | Matrix column |
|-----------|---------------|
| GPA0      | col 0         |
| GPA1      | col 1         |
| GPA2      | col 2         |
| GPA3      | col 3         |
| GPA4      | col 4         |
| GPA5      | col 5         |
| GPA6      | col 6         |
| GPA7      | (unused)      |

GPIOB — row read inputs (internal pull-ups enabled):

| GPIOB pin | Matrix row |
|-----------|------------|
| GPB0      | row 0      |
| GPB1      | row 1      |
| GPB2      | row 2      |
| GPB3      | row 3      |
| GPB4      | row 4      |
| GPB5      | row 5      |
| GPB6      | (unused)   |
| GPB7      | (unused)   |

MCP23018 register configuration:

| Register | Value | Meaning                              |
|----------|-------|--------------------------------------|
| IODIRA   | 0x00  | All Port A pins = outputs (columns)  |
| IODIRB   | 0xFF  | All Port B pins = inputs (rows)      |
| GPPUB    | 0xFF  | Pull-ups on all Port B inputs        |
| GPIOA    | 0xFF  | All columns HIGH initially (inactive)|

## Building and flashing

Requires Docker (for AVR cross-compilation) and a native Rust toolchain (for the CLI).

```sh
make flash     # build firmware + CLI, then flash to keyboard
make build     # build everything without flashing
make hex       # build firmware only (produces firmware.hex)
make detect    # check if Teensy bootloader is detected
```

The CLI automatically reboots the keyboard into its bootloader before flashing.
If the keyboard is unresponsive, press the reset button on the Teensy manually.
