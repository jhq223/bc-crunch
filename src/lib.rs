// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 jhq223 and contributors.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod alpha;
mod block;
mod codec;
mod color;
mod range;
mod types;

pub use codec::{compress, decompress_into};
pub use types::{Error, Format, Result};
