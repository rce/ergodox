# ErgoDox Keyboard Firmware

## Build & Flash

`make flash` builds the firmware in Docker and flashes it to the Teensy.
You can run this unattended — the CLI auto-reboots the keyboard into
bootloader mode before flashing.

## Key Locations

- **Keymap / layout**: `firmware/src/keymap.rs` — layers, Nordic aliases, keycodes
- **Matrix wiring**: `firmware/src/matrix.rs` — GPIO pins, MCP23018 I2C, scan logic
- **Nordic key aliases**: `layout::nordic` module in `keymap.rs` maps Nordic ISO labels to HID keycodes
