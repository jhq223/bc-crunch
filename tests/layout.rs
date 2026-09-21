// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 jhq223 and contributors.
use bc_crunch::{Error, Format, compress, decompress_into};

#[test]
fn decoded_size_is_checked_before_allocation() {
    for format in [Format::Bc1, Format::Bc3, Format::Bc4, Format::Bc5] {
        assert_eq!(format.decoded_len(12, 8), Ok(6 * format.block_bytes()));
        for (width, height) in [(0, 4), (4, 0), (5, 4), (4, 7), (u32::MAX, u32::MAX)] {
            assert_eq!(format.decoded_len(width, height), Err(Error::Dimensions));
        }
    }
    assert_eq!(
        Format::Bc5.decoded_len(u32::MAX - 3, u32::MAX - 3),
        Err(Error::Dimensions)
    );
}

#[test]
fn dictionary_search_preserves_the_encoded_stream() {
    let blocks = include_bytes!("data/bc5.bc");
    let expected = include_bytes!("data/dictionary-reference.bcc");
    let encoded = compress(64, 64, Format::Bc5, blocks).unwrap();
    assert_eq!(encoded, expected.as_slice());
    let mut output = vec![0; blocks.len()];
    decompress_into(64, 64, Format::Bc5, expected, &mut output).unwrap();
    assert_eq!(output, blocks.as_slice());
}
