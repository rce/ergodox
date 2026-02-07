mod halfkay;
mod hex;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;

#[derive(Parser)]
#[command(name = "ergodox-cli")]
#[command(about = "ErgoDox keyboard firmware flasher")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Flash a .hex firmware file to Teensy via HalfKay bootloader
    Flash {
        /// Path to the Intel HEX firmware file
        firmware: String,
    },
    /// Detect if a Teensy is connected in bootloader mode
    Detect,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Flash { firmware } => {
            let contents =
                fs::read_to_string(&firmware).with_context(|| format!("reading {}", firmware))?;

            let segments = hex::parse_hex(&contents).context("parsing Intel HEX file")?;
            let (base_address, data) =
                hex::flatten_segments(&segments).context("flattening HEX segments")?;

            println!(
                "Firmware: {} bytes at base address 0x{:04X}",
                data.len(),
                base_address
            );

            if !halfkay::detect()? {
                eprintln!("Teensy bootloader not detected.");
                eprintln!("Press the reset button on the Teensy and try again.");
                std::process::exit(1);
            }

            halfkay::flash(base_address, &data)?;
        }
        Command::Detect => {
            if halfkay::detect()? {
                println!("Teensy bootloader detected (HalfKay mode).");
            } else {
                println!("Teensy bootloader not detected.");
                println!("Press the reset button on the Teensy to enter bootloader mode.");
            }
        }
    }

    Ok(())
}
