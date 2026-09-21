// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 jhq223 and contributors.
use crate::block::Layout;
use crate::range::{Decoder, Encoder};
use crate::{Error, Format, Result, alpha, color};

/// Encode ordinary row-major BC blocks. Dimensions must be nonzero multiples of four.
/// Includes three arithmetic lookahead bytes.
///
/// # Errors
/// Rejects invalid dimensions and input lengths.
pub fn compress(width: u32, height: u32, format: Format, input: &[u8]) -> Result<Vec<u8>> {
    let layout = Layout::new(width, height, format, input.len())?;
    let mut encoder = Encoder::new(input.len() / 2);
    match format {
        Format::Bc1 => color::encode(&mut encoder, input, layout),
        Format::Bc3 => {
            alpha::encode(&mut encoder, input, layout);
            color::encode(&mut encoder, input, layout.second_plane());
        }
        Format::Bc4 => alpha::encode(&mut encoder, input, layout),
        Format::Bc5 => {
            alpha::encode(&mut encoder, input, layout);
            alpha::encode(&mut encoder, input, layout.second_plane());
        }
    }
    Ok(encoder.finish())
}
/// Decode into caller-owned BC storage. Input reads and output dimensions are
/// checked; errors may leave partial output, which the caller must discard.
/// No allocations proportional to decoded pixels are made by the decoder.
///
/// # Errors
/// Rejects invalid dimensions, buffer lengths, truncated or invalid streams.
/// The format has no checksum and cannot detect every corrupted stream.
pub fn decompress_into(
    width: u32,
    height: u32,
    format: Format,
    input: &[u8],
    output: &mut [u8],
) -> Result<()> {
    let layout = Layout::new(width, height, format, output.len())?;
    if input.len() > output.len().saturating_mul(2).saturating_add(4096) {
        return Err(Error::Length);
    }
    let mut decoder = Decoder::new(input)?;
    match format {
        Format::Bc1 => color::decode(&mut decoder, output, layout)?,
        Format::Bc3 => {
            alpha::decode(&mut decoder, output, layout)?;
            color::decode(&mut decoder, output, layout.second_plane())?;
        }
        Format::Bc4 => alpha::decode(&mut decoder, output, layout)?,
        Format::Bc5 => {
            alpha::decode(&mut decoder, output, layout)?;
            alpha::decode(&mut decoder, output, layout.second_plane())?;
        }
    }
    Ok(())
}
