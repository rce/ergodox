# ErgoDox Keyboard Firmware — build and flash commands

IMAGE := ergodox-firmware
DOCKER_RUN := docker run --rm -v $(CURDIR):/build $(IMAGE)

.PHONY: docker build-firmware build-cli build hex flash detect layout test clean

# Build the Docker image
docker:
	docker build -t $(IMAGE) .

# Build the firmware for AVR (in Docker)
build-firmware: docker
	$(DOCKER_RUN) sh -c 'cd firmware && cargo +nightly build --release'

# Convert ELF to Intel HEX format (in Docker)
hex: build-firmware
	$(DOCKER_RUN) avr-objcopy -O ihex target/avr-none/release/firmware.elf firmware.hex

# Build the CLI tool (native)
build-cli:
	cargo build --release -p ergodox-cli

# Build everything
build: build-cli hex

# Flash firmware to Teensy (press reset button first)
flash: hex build-cli
	cargo run --release -p ergodox-cli -- flash firmware.hex

# Check if Teensy is in bootloader mode
detect:
	cargo run --release -p ergodox-cli -- detect

# Generate HTML layout visualization
layout: build-cli
	cargo run --release -p ergodox-cli -- layout > layout.html
	@echo "Generated layout.html"

# Run CLI tests
test:
	cargo test -p ergodox-cli

# Clean all build artifacts
clean:
	cargo clean
	rm -f firmware.hex
