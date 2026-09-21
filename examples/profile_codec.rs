// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 jhq223 and contributors.
use bc_crunch::{Format, compress, decompress_into};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if !(5..=6).contains(&args.len()) {
        return Err(
            "usage: profile_codec WIDTH HEIGHT bc1|bc3|bc4|bc5 INPUT.bc [OUTPUT.bcc]".into(),
        );
    }
    let width = args[1].parse()?;
    let height = args[2].parse()?;
    let format = match args[3].as_str() {
        "bc1" => Format::Bc1,
        "bc3" => Format::Bc3,
        "bc4" => Format::Bc4,
        "bc5" => Format::Bc5,
        _ => return Err("format must be bc1, bc3, bc4 or bc5".into()),
    };
    let raw = std::fs::read(&args[4])?;
    // Validate before allocating the decoded buffer.
    let mut stream = compress(width, height, format, &raw)?;
    let mut output = vec![0; format.decoded_len(width, height)?];
    decompress_into(width, height, format, &stream, &mut output)?;
    let mut encode_times = [0.0; 5];
    let mut decode_times = [0.0; 5];
    for (encode_time, decode_time) in encode_times.iter_mut().zip(&mut decode_times) {
        let start = Instant::now();
        let next = compress(width, height, format, &raw)?;
        *encode_time = start.elapsed().as_secs_f64() * 1000.0;
        stream = next;
        let start = Instant::now();
        decompress_into(width, height, format, &stream, &mut output)?;
        *decode_time = start.elapsed().as_secs_f64() * 1000.0;
        if output != raw {
            return Err("decompressed blocks differ from the input".into());
        }
    }
    encode_times.sort_by(f64::total_cmp);
    decode_times.sort_by(f64::total_cmp);
    println!(
        "bytes={} compressed={} encode_median_ms={:.3} decode_median_ms={:.3} samples=5",
        raw.len(),
        stream.len(),
        encode_times[2],
        decode_times[2]
    );
    if let Some(path) = args.get(5) {
        std::fs::write(path, stream)?;
    }
    Ok(())
}
