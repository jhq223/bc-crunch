// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 jhq223 and contributors.
// Little-endian reads within validated BC block storage.
pub(crate) fn read_u16(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}
pub(crate) fn read_u32(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}

/// A validated row-major grid and the eight-byte plane currently being coded.
#[derive(Clone, Copy)]
pub(crate) struct Layout {
    pub columns: usize,
    pub rows: usize,
    pub stride: usize,
    pub offset: usize,
}

impl Layout {
    pub fn new(
        width: u32,
        height: u32,
        format: crate::Format,
        length: usize,
    ) -> crate::Result<Self> {
        if length != format.decoded_len(width, height)? {
            return Err(crate::Error::Length);
        }
        Ok(Self {
            columns: width as usize / 4,
            rows: height as usize / 4,
            stride: format.block_bytes(),
            offset: 0,
        })
    }

    // Only BC3/BC5 have a second plane; call sites select it by format.
    pub fn second_plane(self) -> Self {
        debug_assert_eq!(self.stride, 16);
        Self { offset: 8, ..self }
    }
}
