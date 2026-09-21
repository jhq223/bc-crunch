// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 jhq223 and contributors.
use bc_crunch::{Error, Format, compress, decompress_into};
const FORMATS: [Format; 4] = [Format::Bc1, Format::Bc3, Format::Bc4, Format::Bc5];
#[test]
fn upstream_streams_decode_exactly() {
    let fixtures: [(&[u8], &[u8]); 4] = [
        (
            include_bytes!("data/bc1.bc"),
            include_bytes!("data/bc1.bcc"),
        ),
        (
            include_bytes!("data/bc3.bc"),
            include_bytes!("data/bc3.bcc"),
        ),
        (
            include_bytes!("data/bc4.bc"),
            include_bytes!("data/bc4.bcc"),
        ),
        (
            include_bytes!("data/bc5.bc"),
            include_bytes!("data/bc5.bcc"),
        ),
    ];
    for (fmt, (raw, encoded)) in FORMATS.into_iter().zip(fixtures) {
        let mut out = vec![0xcd; raw.len()];
        decompress_into(64, 64, fmt, encoded, &mut out).unwrap();
        assert_eq!(out, raw, "{fmt:?}");
    }
}
#[test]
fn roundtrip_all_formats_and_edge_dimensions() {
    let mut seed = 193u32;
    for fmt in FORMATS {
        for (w, h) in [
            (4, 4),
            (12, 20),
            (64, 64),
            (1024, 4),
            (4, 1024),
            (2048, 8),
            (8, 2048),
        ] {
            for pattern in 0..3 {
                let raw: Vec<_> = (0..w * h / 16 * fmt.block_bytes())
                    .map(|i| {
                        seed ^= seed << 13;
                        seed ^= seed >> 17;
                        seed ^= seed << 5;
                        match pattern {
                            0 => 0,
                            1 => (i % 13) as u8,
                            _ => seed as u8,
                        }
                    })
                    .collect();
                let stream = compress(w as u32, h as u32, fmt, &raw).unwrap();
                let mut out = vec![0xa5; raw.len()];
                decompress_into(w as u32, h as u32, fmt, &stream, &mut out).unwrap();
                assert_eq!(raw, out, "{fmt:?} {w}x{h} {pattern}");
            }
        }
    }
}
#[test]
fn bad_sizes_and_truncated_inputs_return_errors() {
    assert_eq!(
        compress(u32::MAX, 4, Format::Bc1, &[0; 8]),
        Err(Error::Dimensions)
    );
    assert_eq!(compress(4, 4, Format::Bc1, &[0; 7]), Err(Error::Length));
    let raw = include_bytes!("data/bc3.bc");
    let stream = compress(64, 64, Format::Bc3, raw).unwrap();
    let mut out = vec![0; raw.len()];
    for end in 0..stream.len().saturating_sub(3) {
        assert!(
            decompress_into(64, 64, Format::Bc3, &stream[..end], &mut out).is_err(),
            "prefix {end}"
        );
    }
}
#[test]
fn arbitrary_inputs_do_not_panic() {
    let mut seed = 713u32;
    for n in 0..500 {
        let bytes: Vec<_> = (0..n)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                seed as u8
            })
            .collect();
        for fmt in FORMATS {
            let mut out = vec![0; 16 * fmt.block_bytes()];
            let _ = decompress_into(16, 16, fmt, &bytes, &mut out);
        }
    }
}

#[test]
fn mutated_valid_streams_remain_bounded() {
    let raw = include_bytes!("data/bc3.bc");
    let stream = compress(64, 64, Format::Bc3, raw).unwrap();
    let mut out = vec![0; raw.len()];
    for i in (0..stream.len()).step_by(7) {
        for bit in [1, 128, 255] {
            let mut damaged = stream.clone();
            damaged[i] ^= bit;
            // A raw stream has no checksum, so a mutation may decode successfully.
            let _ = decompress_into(64, 64, Format::Bc3, &damaged, &mut out);
        }
    }
}

#[test]
fn independent_calls_can_run_concurrently() {
    let jobs: Vec<_> = FORMATS
        .into_iter()
        .map(|fmt| {
            std::thread::spawn(move || {
                let raw = vec![173; 128 * fmt.block_bytes()];
                let packed = compress(64, 32, fmt, &raw).unwrap();
                let mut out = vec![0; raw.len()];
                decompress_into(64, 32, fmt, &packed, &mut out).unwrap();
                assert_eq!(out, raw);
                assert_eq!(packed, compress(64, 32, fmt, &raw).unwrap());
            })
        })
        .collect();
    for job in jobs {
        job.join().unwrap();
    }
}
