// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 jhq223 and contributors.
use crate::block::Layout;
use crate::{
    Error, Result,
    block::{read_u16, read_u32},
    range::{Decoder, Encoder, Model},
};
fn rgb(c: u16) -> [i32; 3] {
    [(c >> 11) as i32, ((c >> 5) & 63) as i32, (c & 31) as i32]
}
fn delta(a: [i32; 3], b: [i32; 3]) -> i32 {
    (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs()
}
fn table(input: &[u8], stride: usize, offset: usize) -> Vec<u32> {
    // Sort the actual block patterns instead of clearing an 8 MiB hash table.
    let mut values: Vec<_> = input
        .chunks_exact(stride)
        .map(|b| read_u32(b, offset + 4))
        .collect();
    values.sort_unstable();
    let mut counts = Vec::new();
    let mut at = 0;
    while at < values.len() {
        let value = values[at];
        let end = at + values[at..].partition_point(|&x| x == value);
        counts.push((end - at, value));
        at = end;
    }
    counts.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    let mut top: Vec<_> = counts.into_iter().take(256).map(|(_, v)| v).collect();
    top.sort_unstable();
    top
}
pub(crate) fn encode(encoder: &mut Encoder, input: &[u8], layout: Layout) {
    let Layout {
        columns,
        rows,
        stride,
        offset,
    } = layout;
    let top = table(input, stride, offset);
    let mut entry = Model::new(256);
    encoder.bits(top.len() as u32 - 1, 8);
    for b in top[0].to_le_bytes() {
        encoder.bits(b as u32, 8);
    }
    for pair in top.windows(2) {
        for b in pair[1].wrapping_sub(pair[0]).to_le_bytes() {
            encoder.put(&mut entry, b as u32);
        }
    }
    let mut colors: [Model; 3] = std::array::from_fn(|_| Model::new(128));
    let (mut index, mut diff, mut mask, mut reference) = (
        Model::new(top.len()),
        Model::new(256),
        Model::new(16),
        Model::new(2),
    );
    let mut previous = [0u16; 2];
    for y in 0..rows {
        for x in 0..columns {
            let zx = if y & 1 != 0 { x } else { columns - x - 1 };
            let at = (y * columns + zx) * stride + offset;
            for (j, previous_color) in previous.iter_mut().enumerate() {
                let c = read_u16(input, at + j * 2);
                let curr = rgb(c);
                let mut prev = rgb(*previous_color);
                if y > 0 && x != 0 {
                    let up = rgb(read_u16(input, at - columns * stride + j * 2));
                    let use_up = delta(curr, up) < delta(curr, prev);
                    encoder.put(&mut reference, use_up as u32);
                    if use_up {
                        prev = up;
                    }
                }
                let dg = curr[1] - prev[1];
                encoder.put(&mut colors[1], (dg + 64) as u32);
                encoder.put(&mut colors[0], (curr[0] - prev[0] - dg / 2 + 64) as u32);
                encoder.put(&mut colors[2], (curr[2] - prev[2] - dg / 2 + 64) as u32);
                *previous_color = c;
            }
            let pattern = read_u32(input, at + 4);
            // Common selector patterns are already in the dictionary. An exact
            // match has zero error and needs no nearest-pattern search.
            let found = top.binary_search(&pattern).unwrap_or_else(|_| {
                top.iter()
                    .enumerate()
                    .min_by_key(|&(_, v)| ((pattern ^ v).count_ones(), std::cmp::Reverse(*v)))
                    .unwrap()
                    .0
            });
            encoder.put(&mut index, found as u32);
            let bytes = (pattern ^ top[found]).to_le_bytes();
            let bits = bytes
                .iter()
                .enumerate()
                .fold(0, |models, (j, &b)| models | ((b != 0) as u32) << j);
            encoder.put(&mut mask, bits);
            for b in bytes {
                if b != 0 {
                    encoder.put(&mut diff, b as u32);
                }
            }
        }
    }
}
pub(crate) fn decode(decoder: &mut Decoder<'_>, out: &mut [u8], layout: Layout) -> Result<()> {
    let Layout {
        columns,
        rows,
        stride,
        offset,
    } = layout;
    let n = decoder.bits(8)? as usize + 1;
    let mut top = [0u32; 256];
    let mut entry = Model::new(256);
    for j in 0..4 {
        top[0] |= decoder.bits(8)? << (j * 8);
    }
    for i in 1..n {
        let mut v = 0;
        for j in 0..4 {
            v |= decoder.get(&mut entry)? << (j * 8);
        }
        top[i] = top[i - 1].wrapping_add(v);
    }
    let mut colors: [Model; 3] = std::array::from_fn(|_| Model::new(128));
    let (mut index, mut diff, mut mask, mut reference) = (
        Model::new(n),
        Model::new(256),
        Model::new(16),
        Model::new(2),
    );
    let mut previous = [0u16; 2];
    for y in 0..rows {
        for x in 0..columns {
            let zx = if y & 1 != 0 { x } else { columns - x - 1 };
            let at = (y * columns + zx) * stride + offset;
            for (j, previous_color) in previous.iter_mut().enumerate() {
                let mut prev = rgb(*previous_color);
                if y > 0 && x != 0 && decoder.get(&mut reference)? != 0 {
                    prev = rgb(read_u16(out, at - columns * stride + j * 2));
                }
                let dg = decoder.get(&mut colors[1])? as i32 - 64;
                let r = prev[0] + decoder.get(&mut colors[0])? as i32 - 64 + dg / 2;
                let b = prev[2] + decoder.get(&mut colors[2])? as i32 - 64 + dg / 2;
                let g = prev[1] + dg;
                if !(0..32).contains(&r) || !(0..64).contains(&g) || !(0..32).contains(&b) {
                    return Err(Error::InvalidStream);
                }
                let c = ((r << 11) | (g << 5) | b) as u16;
                out[at + j * 2..at + j * 2 + 2].copy_from_slice(&c.to_le_bytes());
                *previous_color = c;
            }
            let found = decoder.get(&mut index)? as usize;
            let bits = decoder.get(&mut mask)?;
            let mut v = 0;
            for j in 0..4 {
                if bits & (1 << j) != 0 {
                    v |= decoder.get(&mut diff)? << (j * 8);
                }
            }
            out[at + 4..at + 8].copy_from_slice(&(v ^ top[found]).to_le_bytes());
        }
    }
    Ok(())
}
