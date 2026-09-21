// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 jhq223 and contributors.
/// Format of the uncompressed BC block data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Eight-byte RGB blocks.
    Bc1,
    /// Sixteen-byte RGBA blocks.
    Bc3,
    /// Eight-byte single-channel blocks.
    Bc4,
    /// Sixteen-byte two-channel blocks.
    Bc5,
}
impl Format {
    /// Bytes in one 4x4 BC block.
    pub const fn block_bytes(self) -> usize {
        match self {
            Self::Bc1 | Self::Bc4 => 8,
            _ => 16,
        }
    }
    /// Size of the decoded BC block buffer, in bytes.
    ///
    /// # Errors
    /// Dimensions must be nonzero multiples of four and fit addressable storage.
    pub fn decoded_len(self, width: u32, height: u32) -> Result<usize> {
        if width == 0 || height == 0 || !width.is_multiple_of(4) || !height.is_multiple_of(4) {
            return Err(Error::Dimensions);
        }
        (width as usize / 4)
            .checked_mul(height as usize / 4)
            .and_then(|n| n.checked_mul(self.block_bytes()))
            .filter(|&n| n <= isize::MAX as usize)
            .ok_or(Error::Dimensions)
    }
}
/// Invalid dimensions, buffer sizes or compressed data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Invalid block dimensions or overflowing storage size.
    Dimensions,
    /// Buffer length does not match the block grid or exceeds the input limit.
    Length,
    /// The stream ended before decoding completed.
    Truncated,
    /// The stream contains invalid coding data.
    InvalidStream,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Dimensions => "invalid or overflowing block grid dimensions",
            Self::Length => "buffer length does not match the block grid",
            Self::Truncated => "truncated compressed stream",
            Self::InvalidStream => "invalid compressed stream",
        })
    }
}
impl std::error::Error for Error {}
/// Result returned by the block stream codec.
pub type Result<T> = std::result::Result<T, Error>;
