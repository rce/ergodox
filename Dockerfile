FROM rust:slim-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc-avr avr-libc binutils-avr \
    && rm -rf /var/lib/apt/lists/*

RUN rustup toolchain install nightly --component rust-src

WORKDIR /build
