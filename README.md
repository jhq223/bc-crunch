# bc-crunch

English | [简体中文](README.zh-CN.md)

Lossless compression of BC1, BC3, BC4 and BC5 texture blocks. Written in safe Rust,
with no dependencies. Decompression restores BC blocks directly into caller-owned
storage.

## Installation

```toml
[dependencies]
bc-crunch = "0.1"
```

## Usage

```rust
use bc_crunch::{compress, decompress_into, Format};

fn main() -> Result<(), bc_crunch::Error> {
    let blocks = vec![0u8; Format::Bc1.decoded_len(8, 8)?];
    let packed = compress(8, 8, Format::Bc1, &blocks)?;
    let mut restored = vec![0u8; blocks.len()];
    decompress_into(8, 8, Format::Bc1, &packed, &mut restored)?;
    assert_eq!(restored, blocks);
    Ok(())
}
```

## Data contract

Input and output blocks use row-major order. Width and height must be
nonzero multiples of four; buffer lengths must match exactly.
`Format::decoded_len` validates dimensions and calculates the BC output size before allocation. For images with
partial edge blocks, pass the dimensions of the padded block grid. BC1 and BC4
use eight bytes per block; BC3 and BC5 use sixteen.

Compression preserves every BC byte, including endpoint ordering and selectors.
Compressed streams may be larger than their input. Applications can store the
smaller of the compressed stream and the original BC blocks.

The container supplies the stream's format, dimensions, codec version and checksum.
Validate output size before allocation and integrity before decoding. Discard
partial output on decoding errors.

Encoder and decoder state is local to each call. Applications can schedule
independent textures or tiles concurrently. Encoding uses
temporary storage proportional to block count. Decoder models and dictionaries
use fixed storage without heap allocations.

## Interoperability

The stream uses its own range-coded format. CRN and Huffman streams require their
respective decoders.
The encoder appends three arithmetic lookahead bytes. When importing a stream
whose encoder omitted these bytes from its reported length, append three zero
bytes explicitly. Input reads stay within the supplied slice. Select the decoder
using the container's codec version.

## Development

Requires Rust 1.98.1 or newer.

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo run --release --example profile_codec -- 1024 1024 bc3 tile.bc
```

Tests cover external fixtures for all four formats, rectangular and large block
grids, truncation and malformed inputs. The profiler reads a raw BC block array,
warms up, and reports the median of five runs. Every decoded
result is verified. Append an output path to save the compressed stream.

## License

The Rust implementation is licensed under [MPL-2.0](./LICENSE). Original third-party
notices remain in [licenses/](licenses/README.md).

## Acknowledgements

Based on [Geolm's bc_crunch](https://github.com/Geolm/bc_crunch), version 1.5.2,
commit `88f0a344acc1b2ce3cc1a8393f422aa1033c0539`. The arithmetic coding work includes
contributions by Amir Said and William A. Pearlman.
