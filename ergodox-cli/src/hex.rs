use anyhow::{bail, Context, Result};

/// A parsed segment of data at a specific address from an Intel HEX file.
#[derive(Debug, Clone)]
pub struct HexSegment {
    pub address: u32,
    pub data: Vec<u8>,
}

/// Parse an Intel HEX format string into address-data segments.
///
/// Supports record types:
/// - 00: Data
/// - 01: End of File
/// - 02: Extended Segment Address
pub fn parse_hex(input: &str) -> Result<Vec<HexSegment>> {
    let mut segments: Vec<HexSegment> = Vec::new();
    let mut base_address: u32 = 0;

    for (line_num, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(':') {
            bail!("line {}: missing start code ':'", line_num + 1);
        }

        let bytes = decode_hex_bytes(&line[1..])
            .with_context(|| format!("line {}: invalid hex data", line_num + 1))?;

        if bytes.len() < 5 {
            bail!("line {}: record too short", line_num + 1);
        }

        let byte_count = bytes[0] as usize;
        let address = u16::from_be_bytes([bytes[1], bytes[2]]);
        let record_type = bytes[3];
        let data = &bytes[4..4 + byte_count];

        if bytes.len() != 5 + byte_count {
            bail!(
                "line {}: expected {} data bytes, got {}",
                line_num + 1,
                byte_count,
                bytes.len() - 5
            );
        }

        // Verify checksum: sum of all bytes (including checksum) should be 0 mod 256
        let checksum: u8 = bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        if checksum != 0 {
            bail!("line {}: checksum mismatch", line_num + 1);
        }

        match record_type {
            0x00 => {
                // Data record
                let full_address = base_address + address as u32;

                // Try to extend the last segment if this data is contiguous
                if let Some(last) = segments.last_mut() {
                    let last_end = last.address + last.data.len() as u32;
                    if full_address == last_end {
                        last.data.extend_from_slice(data);
                        continue;
                    }
                }

                segments.push(HexSegment {
                    address: full_address,
                    data: data.to_vec(),
                });
            }
            0x01 => {
                // End of file
                break;
            }
            0x02 => {
                // Extended segment address
                if byte_count != 2 {
                    bail!("line {}: extended segment address must be 2 bytes", line_num + 1);
                }
                base_address = (u16::from_be_bytes([data[0], data[1]]) as u32) << 4;
            }
            other => {
                bail!("line {}: unsupported record type 0x{:02X}", line_num + 1, other);
            }
        }
    }

    Ok(segments)
}

/// Flatten parsed HEX segments into a contiguous firmware image.
/// Returns (base_address, data) where data is zero-filled for any gaps.
pub fn flatten_segments(segments: &[HexSegment]) -> Result<(u32, Vec<u8>)> {
    if segments.is_empty() {
        bail!("no data segments in HEX file");
    }

    let min_addr = segments.iter().map(|s| s.address).min().unwrap();
    let max_addr = segments
        .iter()
        .map(|s| s.address + s.data.len() as u32)
        .max()
        .unwrap();

    let total_size = (max_addr - min_addr) as usize;
    let mut image = vec![0xFFu8; total_size]; // 0xFF = erased flash

    for seg in segments {
        let offset = (seg.address - min_addr) as usize;
        image[offset..offset + seg.data.len()].copy_from_slice(&seg.data);
    }

    Ok((min_addr, image))
}

fn decode_hex_bytes(hex: &str) -> Result<Vec<u8>> {
    if hex.len() % 2 != 0 {
        bail!("odd number of hex characters");
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .with_context(|| format!("invalid hex at position {}", i))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_hex() {
        let hex = ":10000000000102030405060708090A0B0C0D0E0F78\n\
                   :00000001FF\n";
        let segments = parse_hex(hex).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].address, 0);
        assert_eq!(
            segments[0].data,
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    #[test]
    fn test_parse_extended_segment() {
        let hex = ":020000020100FB\n\
                   :10000000112233445566778899AABBCCDDEEFF00F8\n\
                   :00000001FF\n";
        let segments = parse_hex(hex).unwrap();
        assert_eq!(segments.len(), 1);
        // Base address = 0x0100 << 4 = 0x1000
        assert_eq!(segments[0].address, 0x1000);
    }

    #[test]
    fn test_checksum_error() {
        let hex = ":10000000000102030405060708090A0B0C0D0E0F00\n\
                   :00000001FF\n";
        assert!(parse_hex(hex).is_err());
    }

    #[test]
    fn test_contiguous_merge() {
        let hex = ":04000000AABBCCDDEE\n\
                   :04000400112233444E\n\
                   :00000001FF\n";
        let segments = parse_hex(hex).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].data, vec![0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44]);
    }

    #[test]
    fn test_flatten() {
        let segments = vec![
            HexSegment {
                address: 0x100,
                data: vec![0xAA, 0xBB],
            },
            HexSegment {
                address: 0x110,
                data: vec![0xCC, 0xDD],
            },
        ];
        let (base, image) = flatten_segments(&segments).unwrap();
        assert_eq!(base, 0x100);
        assert_eq!(image.len(), 0x12);
        assert_eq!(image[0], 0xAA);
        assert_eq!(image[1], 0xBB);
        // Gap should be 0xFF
        assert_eq!(image[2], 0xFF);
        assert_eq!(image[0x10], 0xCC);
        assert_eq!(image[0x11], 0xDD);
    }
}
