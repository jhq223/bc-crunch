// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 jhq223 and contributors.
use crate::block::Layout;
use crate::{
    Result,
    block::read_u16,
    range::{Decoder, Encoder, Model},
};
const ZIG: [usize; 16] = [0, 1, 2, 3, 7, 6, 5, 4, 8, 9, 10, 11, 15, 14, 13, 12];
const MASK: u64 = (1 << 48) - 1;
fn packed(b: &[u8], at: usize) -> u64 {
    (read_u16(b, at + 2) as u64) << 32
        | (read_u16(b, at + 4) as u64) << 16
        | read_u16(b, at + 6) as u64
}
fn linear(b: &[u8], at: usize) -> u64 {
    (read_u16(b, at + 6) as u64) << 32
        | (read_u16(b, at + 4) as u64) << 16
        | read_u16(b, at + 2) as u64
}
fn swap_selector_words(v: u64) -> u64 {
    (v & 65535) << 32 | (v & 0xffff0000) | ((v >> 32) & 65535)
}
fn store(b: &mut [u8], at: usize, v: u64) {
    b[at + 2..at + 8].copy_from_slice(&v.to_le_bytes()[..6]);
}
fn bucket(a: u8, b: u8) -> usize {
    let decoder = a.abs_diff(b);
    if decoder < 8 {
        0
    } else if decoder < 32 {
        8
    } else {
        16
    }
}
fn reference(
    data: &[u8],
    at: usize,
    columns: usize,
    stride: usize,
    x: usize,
    y: usize,
    previous: i32,
) -> u8 {
    let mut r = previous;
    if y > 0 {
        let up = data[at - columns * stride] as i32;
        r = if x > 0 {
            r + up - data[at - columns * stride - stride] as i32
        } else {
            up
        };
    }
    r.clamp(0, 255) as u8
}
fn promote(dictionary: &mut [u64; 256], i: usize) {
    if i > 0 {
        let v = dictionary[i];
        let to = i / 2;
        dictionary.copy_within(to..i, to + 1);
        dictionary[to] = v;
    }
}
fn insert(dictionary: &mut [u64; 256], v: u64) {
    dictionary.copy_within(128..255, 129);
    dictionary[128] = v;
}
struct Models {
    colors: [Model; 2],
    first: Model,
    use_dict: Model,
    reference: Model,
    indices: [Model; 24],
    delta: [Model; 16],
}
impl Models {
    fn new<const DECODE: bool>() -> Self {
        Self {
            colors: std::array::from_fn(|_| Model::new::<DECODE>(256)),
            first: Model::new::<DECODE>(8),
            use_dict: Model::new::<DECODE>(2),
            reference: Model::new::<DECODE>(256),
            indices: std::array::from_fn(|_| Model::new::<DECODE>(8)),
            delta: std::array::from_fn(|_| Model::new::<DECODE>(8)),
        }
    }
}
// Only dictionary matches with fewer than five changed bits are emitted.
fn nearest(dictionary: &[u64; 256], value: u64) -> Option<usize> {
    let mut best = None;
    let mut distance = 5;
    let mut candidate = 0;
    for (index, &entry) in dictionary.iter().enumerate() {
        let error = ((value ^ entry) & MASK).count_ones();
        if error < distance || (error == distance && best.is_some() && entry > candidate) {
            best = Some(index);
            distance = error;
            candidate = entry;
            // The sentinel shares its low 48 bits with the all-ones pattern.
            // Preserve its priority when both have zero distance.
            if error == 0 && (value != MASK || entry == u64::MAX) {
                break;
            }
        }
    }
    best
}
pub(crate) fn encode(encoder: &mut Encoder, input: &[u8], layout: Layout) {
    let Layout {
        columns,
        rows,
        stride,
        offset,
    } = layout;
    let mut models = Models::new::<false>();
    let mut dictionary = [u64::MAX; 256];
    let mut prev = 0;
    for y in 0..rows {
        for x in 0..columns {
            let at = (y * columns + x) * stride + offset;
            let a = input[at];
            let b = input[at + 1];
            let r = reference(input, at, columns, stride, x, y, prev);
            encoder.put(&mut models.colors[0], a.wrapping_sub(r) as u32);
            encoder.put(&mut models.colors[1], b.wrapping_sub(a) as u32);
            let v = packed(input, at);
            let found = if y * columns + x > 32 {
                nearest(&dictionary, v)
            } else {
                None
            };
            if let Some(i) = found {
                encoder.put(&mut models.use_dict, 1);
                encoder.put(&mut models.reference, i as u32);
                let delta = dictionary[i] ^ v;
                for j in 0..16 {
                    encoder.put(&mut models.delta[j], ((delta >> (j * 3)) & 7) as u32);
                }
                promote(&mut dictionary, i);
            } else {
                insert(&mut dictionary, v);
                encoder.put(&mut models.use_dict, 0);
                let v = linear(input, at);
                let mut last = (v & 7) as usize;
                encoder.put(&mut models.first, last as u32);
                let base = bucket(a, b);
                for &z in &ZIG[1..] {
                    let next = ((v >> (z * 3)) & 7) as usize;
                    encoder.put(&mut models.indices[base + last], (last ^ next) as u32);
                    last = next;
                }
            }
            prev = a as i32;
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
    let mut models = Models::new::<true>();
    let mut dictionary = [u64::MAX; 256];
    let mut prev = 0;
    for y in 0..rows {
        for x in 0..columns {
            let at = (y * columns + x) * stride + offset;
            let r = reference(out, at, columns, stride, x, y, prev);
            let a = r.wrapping_add(decoder.get(&mut models.colors[0])? as u8);
            let b = a.wrapping_add(decoder.get(&mut models.colors[1])? as u8);
            out[at] = a;
            out[at + 1] = b;
            if decoder.get(&mut models.use_dict)? != 0 {
                let i = decoder.get(&mut models.reference)? as usize;
                let mut v = 0u64;
                for j in 0..16 {
                    v |= (decoder.get(&mut models.delta[j])? as u64) << (j * 3);
                }
                v ^= dictionary[i];
                store(out, at, swap_selector_words(v));
                promote(&mut dictionary, i);
            } else {
                let mut last = decoder.get(&mut models.first)? as usize;
                let mut v = last as u64;
                let base = bucket(a, b);
                for &z in &ZIG[1..] {
                    let next = last ^ decoder.get(&mut models.indices[base + last])? as usize;
                    v |= (next as u64) << (z * 3);
                    last = next;
                }
                store(out, at, v);
                insert(&mut dictionary, swap_selector_words(v));
            }
            prev = a as i32;
        }
    }
    Ok(())
}
