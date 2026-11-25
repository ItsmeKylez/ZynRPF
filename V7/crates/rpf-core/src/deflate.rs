//! Raw DEFLATE (RFC 1951) decoder and a "stored" encoder. Standard library only (clean-room, written from the RFC).
//!
//! Status: M5b step 1. The algorithm was checked in a Python port against zlib (4,800 round trips, hostile inputs),
//! but this Rust file had NOT been compiled or run when it was written (no Rust toolchain in that session).
//! Not wired into the parser or server yet. RPF entries are assumed to hold raw DEFLATE without a zlib header
//! (UNVERIFIED against real game files).
//!
//! Safety rules: every read is bounds-checked, output never exceeds `max_out`, no recursion, no panics on any input.

use std::fmt;

/// Why a DEFLATE stream was rejected. Mapping to the English error catalogue and new check codes comes later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InflateError {
    /// The input ended before the stream was complete.
    Truncated,
    /// A block header used the reserved type 3.
    BadBlockType,
    /// A stored block's length and its complement do not match.
    BadStoredLength,
    /// The code length tables are impossible (over-subscribed, repeat with no previous, missing end-of-block).
    BadCodeLengths,
    /// A bit pattern matched no code, or a reserved symbol was used.
    BadSymbol,
    /// A back-reference pointed before the start of the output.
    BadDistance,
    /// The output would be larger than the limit given by the caller.
    OutputLimit,
}

impl fmt::Display for InflateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            InflateError::Truncated => "The compressed data ends too early.",
            InflateError::BadBlockType => "The compressed data has an invalid block type.",
            InflateError::BadStoredLength => "A stored block in the compressed data has an invalid length.",
            InflateError::BadCodeLengths => "The compressed data has invalid code tables.",
            InflateError::BadSymbol => "The compressed data contains an invalid code.",
            InflateError::BadDistance => "The compressed data refers to data before its start.",
            InflateError::OutputLimit => "The data is larger than allowed once decompressed.",
        };
        f.write_str(s)
    }
}

impl std::error::Error for InflateError {}

const LBASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258,
];
const LEXT: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DBASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145,
    8193, 12289, 16385, 24577,
];
const DEXT: [u8; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
const ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    buf: u32,
    cnt: u32,
}

impl<'a> Bits<'a> {
    /// Read `n` bits (0 to 16), least significant first.
    fn need(&mut self, n: u32) -> Result<u32, InflateError> {
        while self.cnt < n {
            let byte = *self.data.get(self.pos).ok_or(InflateError::Truncated)?;
            self.buf |= (byte as u32) << self.cnt;
            self.pos += 1;
            self.cnt += 8;
        }
        let v = self.buf & ((1u32 << n) - 1);
        self.buf >>= n;
        self.cnt -= n;
        Ok(v)
    }
}

/// Canonical Huffman table (counts per length plus symbols in code order).
struct Huff {
    count: [u16; 16],
    symbol: Vec<u16>,
}

impl Huff {
    fn build(lengths: &[u8]) -> Result<Huff, InflateError> {
        let mut count = [0u16; 16];
        for &l in lengths {
            if l > 15 {
                return Err(InflateError::BadCodeLengths);
            }
            count[l as usize] += 1;
        }
        let mut left: i32 = 1;
        for len in 1..16 {
            left <<= 1;
            left -= count[len] as i32;
            if left < 0 {
                return Err(InflateError::BadCodeLengths);
            }
        }
        let mut offs = [0u16; 16];
        for len in 1..15 {
            offs[len + 1] = offs[len] + count[len];
        }
        let mut symbol = vec![0u16; lengths.len()];
        for (sym, &l) in lengths.iter().enumerate() {
            if l != 0 {
                let slot = offs[l as usize] as usize;
                symbol[slot] = sym as u16;
                offs[l as usize] += 1;
            }
        }
        Ok(Huff { count, symbol })
    }

    fn decode(&self, b: &mut Bits) -> Result<u16, InflateError> {
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: i32 = 0;
        for len in 1..16 {
            code |= b.need(1)? as i32;
            let count = self.count[len] as i32;
            if code - count < first {
                let at = (index + (code - first)) as usize;
                return self.symbol.get(at).copied().ok_or(InflateError::BadSymbol);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err(InflateError::BadSymbol)
    }
}

fn codes(b: &mut Bits, out: &mut Vec<u8>, lit: &Huff, dist: &Huff, max_out: usize) -> Result<(), InflateError> {
    loop {
        let sym = lit.decode(b)? as usize;
        if sym < 256 {
            if out.len() >= max_out {
                return Err(InflateError::OutputLimit);
            }
            out.push(sym as u8);
        } else if sym == 256 {
            return Ok(());
        } else {
            let s = sym - 257;
            if s >= 29 {
                return Err(InflateError::BadSymbol);
            }
            let len = LBASE[s] as usize + b.need(LEXT[s] as u32)? as usize;
            let ds = dist.decode(b)? as usize;
            if ds >= 30 {
                return Err(InflateError::BadDistance);
            }
            let d = DBASE[ds] as usize + b.need(DEXT[ds] as u32)? as usize;
            if d > out.len() {
                return Err(InflateError::BadDistance);
            }
            if out.len().saturating_add(len) > max_out {
                return Err(InflateError::OutputLimit);
            }
            let start = out.len() - d;
            for i in 0..len {
                let c = out[start + i];
                out.push(c);
            }
        }
    }
}

/// Decompress a raw DEFLATE stream. The result is never longer than `max_out` bytes; longer data gives `OutputLimit`.
/// Bytes after the final block are ignored.
pub fn inflate(data: &[u8], max_out: usize) -> Result<Vec<u8>, InflateError> {
    let mut b = Bits { data, pos: 0, buf: 0, cnt: 0 };
    let mut out: Vec<u8> = Vec::new();
    loop {
        let last = b.need(1)?;
        let ty = b.need(2)?;
        match ty {
            0 => {
                b.buf = 0;
                b.cnt = 0;
                if b.pos.saturating_add(4) > data.len() {
                    return Err(InflateError::Truncated);
                }
                let len = data[b.pos] as usize | (data[b.pos + 1] as usize) << 8;
                let nlen = data[b.pos + 2] as usize | (data[b.pos + 3] as usize) << 8;
                if len != (!nlen & 0xFFFF) {
                    return Err(InflateError::BadStoredLength);
                }
                b.pos += 4;
                if b.pos.saturating_add(len) > data.len() {
                    return Err(InflateError::Truncated);
                }
                if out.len().saturating_add(len) > max_out {
                    return Err(InflateError::OutputLimit);
                }
                out.extend_from_slice(&data[b.pos..b.pos + len]);
                b.pos += len;
            }
            1 => {
                let mut ll = [0u8; 288];
                for (i, l) in ll.iter_mut().enumerate() {
                    *l = if i < 144 { 8 } else if i < 256 { 9 } else if i < 280 { 7 } else { 8 };
                }
                let lit = Huff::build(&ll)?;
                let dist = Huff::build(&[5u8; 30])?;
                codes(&mut b, &mut out, &lit, &dist, max_out)?;
            }
            2 => {
                let nlen = b.need(5)? as usize + 257;
                let ndist = b.need(5)? as usize + 1;
                let ncode = b.need(4)? as usize + 4;
                if nlen > 286 || ndist > 30 {
                    return Err(InflateError::BadCodeLengths);
                }
                let mut cl = [0u8; 19];
                for i in 0..ncode {
                    cl[ORDER[i]] = b.need(3)? as u8;
                }
                let lc = Huff::build(&cl)?;
                let mut lens = [0u8; 320];
                let total = nlen + ndist;
                let mut idx = 0usize;
                while idx < total {
                    let sym = lc.decode(&mut b)?;
                    if sym < 16 {
                        lens[idx] = sym as u8;
                        idx += 1;
                    } else {
                        let (val, rep) = if sym == 16 {
                            if idx == 0 {
                                return Err(InflateError::BadCodeLengths);
                            }
                            (lens[idx - 1], 3 + b.need(2)? as usize)
                        } else if sym == 17 {
                            (0u8, 3 + b.need(3)? as usize)
                        } else {
                            (0u8, 11 + b.need(7)? as usize)
                        };
                        if idx + rep > total {
                            return Err(InflateError::BadCodeLengths);
                        }
                        for _ in 0..rep {
                            lens[idx] = val;
                            idx += 1;
                        }
                    }
                }
                if lens[256] == 0 {
                    return Err(InflateError::BadCodeLengths);
                }
                let lit = Huff::build(&lens[..nlen])?;
                let dist = Huff::build(&lens[nlen..total])?;
                codes(&mut b, &mut out, &lit, &dist, max_out)?;
            }
            _ => return Err(InflateError::BadBlockType),
        }
        if last == 1 {
            break;
        }
    }
    Ok(out)
}

/// Wrap `data` in valid raw DEFLATE using stored (uncompressed) blocks. Always correct, never smaller than the input.
/// Real compression (fixed or dynamic Huffman) is a later step.
pub fn deflate_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + (data.len() / 65535 + 1) * 5);
    if data.is_empty() {
        out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
        return out;
    }
    let mut chunks = data.chunks(65535).peekable();
    while let Some(c) = chunks.next() {
        let last = chunks.peek().is_none();
        out.push(if last { 1 } else { 0 });
        let l = c.len() as u16;
        out.extend_from_slice(&l.to_le_bytes());
        out.extend_from_slice(&(!l).to_le_bytes());
        out.extend_from_slice(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }
    fn dyn_input() -> Vec<u8> {
        (0..60).map(|i| format!("<Item><name>wheel_{}</name><value>{}.5</value></Item>\n", i % 7, i * 3)).collect::<String>().into_bytes()
    }

    // Vectors below were produced by Python's zlib (raw deflate, wbits -15): fixed = Z_FIXED, dynamic = default strategy, stored = level 0.
    const FIXED: &str = "cb48cdc9c957c840903a0a41016e0aa9299925f9458ac8125c00";
    const DYNAMIC: &str = "9dd5396a04311040d1dc8771ab7609c4e43e8599a0c1c18c232fd7378ca36e3b103f93403f7988aaf9f2b1df2ff3fd7adf2fdf6ffb7e7b6d737bdce6d7f5f6b95fda73ccedf738b7c7e3a7f9a79163632b8d1e9b5c69ecd88c95c68f8de84a14a72856a23c457d253a71ab006f7500ae05c4ad017233406e09c86d00725740ee01c8bd03f210401e0ec8a3007936409e06c833c9581980bc14905700f2ea80bc0b20ef0ec87b01f2d1c82837403e12908f41667953602e2d00bab40ed44504b08b38d9a15264896a23f26a445e93c8eb20f2a644de82c85b27f22e44de9dc87b11f968443e8cc84712f918443e95c86710f9ec44be84c89713f9fae76ffc00";
    const STORED: &str = "011c00e3ff73746f72656420626c6f636b20746573742030313233343536373839";
    const ZEROS_1MIB: &str = "edc13101000000c2a0f54f6d085fa000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003e03";

    #[test]
    fn fixed_block() {
        let out = inflate(&unhex(FIXED), 1 << 20).unwrap();
        assert_eq!(out, "hello hello hello, RPF editor! hello hello\n".as_bytes());
    }
    #[test]
    fn dynamic_block() {
        assert_eq!(inflate(&unhex(DYNAMIC), 1 << 20).unwrap(), dyn_input());
    }
    #[test]
    fn stored_block_from_zlib() {
        assert_eq!(inflate(&unhex(STORED), 1 << 20).unwrap(), b"stored block test 0123456789");
    }
    #[test]
    fn own_stored_encoder_round_trips() {
        for n in [0usize, 1, 100, 65535, 65536, 200_000] {
            let data: Vec<u8> = (0..n).map(|i| (i * 7 + i / 13) as u8).collect();
            assert_eq!(inflate(&deflate_stored(&data), n).unwrap(), data, "n={n}");
        }
    }
    #[test]
    fn long_zero_run_uses_overlapping_copies() {
        let out = inflate(&unhex(ZEROS_1MIB), 1 << 20).unwrap();
        assert_eq!(out.len(), 1 << 20);
        assert!(out.iter().all(|&b| b == 0));
    }
    #[test]
    fn output_limit_stops_a_bomb() {
        assert_eq!(inflate(&unhex(ZEROS_1MIB), 1000), Err(InflateError::OutputLimit));
        assert_eq!(inflate(&unhex(DYNAMIC), dyn_input().len() - 1), Err(InflateError::OutputLimit));
        assert_eq!(inflate(&deflate_stored(&[1, 2, 3]), 2), Err(InflateError::OutputLimit));
    }
    #[test]
    fn every_truncation_is_an_error_not_a_panic() {
        let z = unhex(DYNAMIC);
        for cut in 0..z.len() {
            let _ = inflate(&z[..cut], 1 << 20);
        }
        assert_eq!(inflate(&[], 10), Err(InflateError::Truncated));
    }
    #[test]
    fn reserved_block_type_and_bad_stored_length() {
        assert_eq!(inflate(&[0x07], 10), Err(InflateError::BadBlockType));
        assert_eq!(inflate(&[1, 3, 0, 0, 0, 1, 2, 3], 10), Err(InflateError::BadStoredLength));
    }
    #[test]
    fn garbage_never_panics() {
        let mut x: u32 = 12345;
        for _ in 0..20_000 {
            let n = (x % 41) as usize;
            let mut g = Vec::with_capacity(n);
            for _ in 0..n {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                g.push((x >> 24) as u8);
            }
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            let _ = inflate(&g, 1 << 16);
        }
    }
}
