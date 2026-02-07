# ErgoDox Keyboard Firmware — build and flash commands

# Build the firmware for AVR
build-firmware:
    cd firmware && cargo build --release

# Build the CLI tool
build-cli:
    cargo build --release -p ergodox-cli

# Build everything
build: build-cli build-firmware

# Convert ELF to Intel HEX format
hex: build-firmware
    avr-objcopy -O ihex firmware/target/avr-unknown-gnu-atmega32u4/release/firmware firmware.hex

# Flash firmware to Teensy (press reset button first)
flash: hex
    cargo run --release -p ergodox-cli -- flash firmware.hex

# Check if Teensy is in bootloader mode
detect:
    cargo run --release -p ergodox-cli -- detect

# Run CLI tests
test:
    cargo test -p ergodox-cli

# Clean all build artifacts
clean:
    cargo clean
    rm -f firmware.hex
