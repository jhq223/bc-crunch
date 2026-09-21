// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 jhq223 and contributors.
use bc_crunch::{Format, compress, decompress_into};

#[test]
fn frequency_cutoff_preserves_dictionary_ties_and_stream_bytes() {
    // 300 distinct selector patterns compete for 256 dictionary slots.
    // Frequencies tie on both sides of the cutoff.
    let mut blocks = Vec::new();
    for block in 0..1024u32 {
        blocks.extend_from_slice(&(block.wrapping_mul(31) as u16).to_le_bytes());
        blocks.extend_from_slice(&(block.wrapping_mul(17).wrapping_add(9) as u16).to_le_bytes());
        blocks.extend_from_slice(&(block % 300).wrapping_mul(0x0102_0305).to_le_bytes());
    }
    let reference = include_bytes!("data/color-dictionary.bcc");
    let packed = compress(128, 128, Format::Bc1, &blocks).unwrap();
    assert_eq!(packed, reference.as_slice());
    let mut output = vec![0; blocks.len()];
    decompress_into(128, 128, Format::Bc1, &packed, &mut output).unwrap();
    assert_eq!(output, blocks);
}
