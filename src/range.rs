// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 jhq223 and contributors.
// Third-party notices: ../licenses/README.md.
use crate::{Error, Result};
const MIN: u32 = 0x0100_0000;
pub(crate) struct Model {
    cdf: Vec<u32>,
    counts: Vec<u32>,
    lookup: Vec<usize>,
    shift: u32,
    total: u32,
    cycle: u32,
    left: u32,
}
impl Model {
    pub(crate) fn new<const DECODE: bool>(n: usize) -> Self {
        assert!((1..=256).contains(&n));
        let bits = if n > 16 {
            (usize::BITS - 1 - (n - 1).leading_zeros()).saturating_sub(1)
        } else {
            0
        };
        let mut m = Self {
            cdf: vec![0; n],
            counts: vec![1; n],
            lookup: if DECODE && bits > 0 {
                vec![0; (1 << bits) + 2]
            } else {
                vec![]
            },
            shift: 15 - bits,
            total: 0,
            cycle: n as u32,
            left: 0,
        };
        m.update();
        m.cycle = (n as u32 + 6) >> 1;
        m.left = m.cycle;
        m
    }
    fn update(&mut self) {
        self.total += self.cycle;
        if self.total > 32768 {
            self.total = 0;
            for x in &mut self.counts {
                *x = (*x + 1) >> 1;
                self.total += *x;
            }
        }
        let scale = 0x8000_0000u32 / self.total;
        let mut sum = 0u32;
        for (out, &count) in self.cdf.iter_mut().zip(&self.counts) {
            *out = scale.wrapping_mul(sum) >> 16;
            sum += count;
        }
        if !self.lookup.is_empty() {
            let mut at = 0;
            for (i, &value) in self.cdf.iter().enumerate() {
                let end = (value >> self.shift) as usize;
                while at < end {
                    at += 1;
                    self.lookup[at] = i.saturating_sub(1);
                }
            }
            self.lookup[0] = 0;
            while at + 1 < self.lookup.len() {
                at += 1;
                self.lookup[at] = self.cdf.len() - 1;
            }
        }
        self.cycle = ((5 * self.cycle) >> 2).min((self.counts.len() as u32 + 6) << 3);
        self.left = self.cycle;
    }
    fn record(&mut self, s: usize) {
        self.counts[s] += 1;
        self.left -= 1;
        if self.left == 0 {
            self.update();
        }
    }
}
pub(crate) struct Encoder {
    bytes: Vec<u8>,
    base: u32,
    len: u32,
}
impl Encoder {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(cap),
            base: 0,
            len: u32::MAX,
        }
    }
    fn carry(&mut self) {
        for b in self.bytes.iter_mut().rev() {
            if *b == 255 {
                *b = 0;
            } else {
                *b += 1;
                return;
            }
        }
        unreachable!("range coder carry without preceding byte");
    }
    fn renorm(&mut self) {
        loop {
            self.bytes.push((self.base >> 24) as u8);
            self.base <<= 8;
            self.len <<= 8;
            if self.len >= MIN {
                break;
            }
        }
    }
    pub(crate) fn put(&mut self, m: &mut Model, s: u32) {
        let s = s as usize;
        let before = self.base;
        let scale = self.len >> 15;
        let x = m.cdf[s] * scale;
        self.base = self.base.wrapping_add(x);
        self.len = if s + 1 == m.cdf.len() {
            self.len - x
        } else {
            m.cdf[s + 1] * scale - x
        };
        if before > self.base {
            self.carry();
        }
        if self.len < MIN {
            self.renorm();
        }
        m.record(s);
    }
    pub(crate) fn bits(&mut self, value: u32, bits: u32) {
        let before = self.base;
        self.len >>= bits;
        self.base = self.base.wrapping_add(value * self.len);
        if before > self.base {
            self.carry();
        }
        if self.len < MIN {
            self.renorm();
        }
    }
    pub(crate) fn finish(mut self) -> Vec<u8> {
        let before = self.base;
        if self.len > 2 * MIN {
            self.base = self.base.wrapping_add(MIN);
            self.len = MIN >> 1;
        } else {
            self.base = self.base.wrapping_add(MIN >> 1);
            self.len = MIN >> 9;
        }
        if before > self.base {
            self.carry();
        }
        self.renorm();
        self.bytes.extend([0; 3]);
        self.bytes
    }
}
pub(crate) struct Decoder<'a> {
    bytes: &'a [u8],
    at: usize,
    value: u32,
    len: u32,
}
impl<'a> Decoder<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Result<Self> {
        let head = bytes.get(..4).ok_or(Error::Truncated)?;
        Ok(Self {
            bytes,
            at: 4,
            value: u32::from_be_bytes(head.try_into().unwrap()),
            len: u32::MAX,
        })
    }
    fn renorm(&mut self) -> Result<()> {
        if self.len == 0 {
            return Err(Error::InvalidStream);
        }
        while self.len < MIN {
            self.value =
                (self.value << 8) | u32::from(*self.bytes.get(self.at).ok_or(Error::Truncated)?);
            self.at += 1;
            self.len <<= 8;
        }
        Ok(())
    }
    pub(crate) fn get(&mut self, m: &mut Model) -> Result<u32> {
        if self.value >= self.len {
            return Err(Error::InvalidStream);
        }
        let scale = self.len >> 15;
        if scale == 0 {
            return Err(Error::InvalidStream);
        }
        let dv = self.value / scale;
        let (mut lo, mut hi) = if m.lookup.is_empty() {
            (0, m.cdf.len())
        } else {
            let t = (dv >> m.shift) as usize;
            (
                *m.lookup.get(t).ok_or(Error::InvalidStream)?,
                m.lookup.get(t + 1).ok_or(Error::InvalidStream)? + 1,
            )
        };
        while hi > lo + 1 {
            let mid = (lo + hi) / 2;
            if m.cdf[mid] > dv {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let s = lo;
        let x = m.cdf[s] * scale;
        let y = if s + 1 == m.cdf.len() {
            self.len
        } else {
            m.cdf[s + 1] * scale
        };
        self.value -= x;
        self.len = y - x;
        self.renorm()?;
        m.record(s);
        Ok(s as u32)
    }
    pub(crate) fn bits(&mut self, bits: u32) -> Result<u32> {
        self.len >>= bits;
        if self.len == 0 {
            return Err(Error::InvalidStream);
        }
        let s = self.value / self.len;
        if s >= 1 << bits {
            return Err(Error::InvalidStream);
        }
        self.value -= self.len * s;
        self.renorm()?;
        Ok(s)
    }
}
