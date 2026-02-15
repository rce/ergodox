# Design Notes

Internal details about how things work under the hood.

## Firmware Flashing

`make flash` can flash the keyboard without manually pressing the reset button.
This works through three phases:

### 1. Auto-reboot into bootloader

The CLI sends a **USB vendor control request** to the running keyboard:

- `bmRequestType = 0x40` — standard USB vendor-type request (host-to-device)
- `bRequest = 0xFF` — our custom "reboot into bootloader" command

This is the only custom part of the flashing protocol. The `bRequest` byte is
ours to define — values `0x00` through `0xFF` are available for custom vendor
commands, and each request also carries `wValue`, `wIndex` (both 16-bit), and
an optional data payload, so there's room for a whole command protocol if needed.

When the firmware receives this request (`firmware/src/hid.rs`), it:

1. Disables interrupts
2. Detaches USB and freezes the clock
3. Waits briefly for the host to notice the disconnect
4. Zeroes out all peripheral registers (SPI, TWI, timers, ADC, UART, all GPIO)
5. Jumps to `0x7E00` — the HalfKay bootloader entry point

The thorough peripheral cleanup is important: HalfKay expects a clean hardware
state, as if the chip just powered on.

### 2. Bootloader detection

After sending the reboot request, the CLI polls USB for up to 5 seconds waiting
for the HalfKay bootloader to appear. The running firmware and the bootloader
present different USB product IDs:

| State               | VID      | PID      |
|---------------------|----------|----------|
| Running firmware    | `0x16C0` | `0x047E` |
| HalfKay bootloader  | `0x16C0` | `0x0478` |

If the bootloader doesn't appear, the CLI falls back to asking you to press the
physical reset button on the Teensy.

### 3. HalfKay flashing

This phase is entirely standard — HalfKay is PJRC's bootloader for Teensy
boards. The CLI writes firmware data in 128-byte pages (the ATmega32U4's flash
page size) via HID SET_REPORT control transfers:

- Each page: 2-byte little-endian address + 128 bytes of data
- Pages that are all `0xFF` (erased flash) are skipped
- 5ms delay between pages for the flash write to complete
- Writing to address `0xFFFF` tells HalfKay to reboot into the new firmware

### USB vendor/product IDs

VID `0x16C0` belongs to Van Ooijen Technische Informatica, who provide a shared
pool of USB IDs for hobbyist and open-source projects. PID `0x047E` and `0x0478`
are from that shared pool — they're not unique to this keyboard. Any
Teensy-based keyboard project using the same convention would show identical IDs.
