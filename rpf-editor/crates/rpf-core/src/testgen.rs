//! Synthetic archive generator for tests and demos (D-030: fixtures are generated, never real game assets).
use crate::format::*;
use std::io::{self, Write};

fn align(v: u64) -> u64 {
    (v + BLOCK - 1) / BLOCK * BLOCK
}

/// Writes a valid unencrypted synthetic archive: root, `dirs` folders, files split evenly across them.
/// Each file holds `total / files` bytes of deterministic pseudo-random data (all stored uncompressed).
/// File names come from a pool of at most 3,000 names (16-bit file name offsets limit the table).
pub fn gen_to<W: Write>(w: &mut W, n: u32, total: u64, dirs: Option<u32>) -> io::Result<()> {
    let d = dirs.unwrap_or((n / 1001).max(1));
    let f = n - 1 - d;
    let p = f.min(3000);
    let file_name_off = |j: u32| 1 + (j % p) * 10;
    let dir_name_off = |k: u32| 1 + p * 10 + k * 7;
    let mut names = vec![0u8];
    for j in 0..p {
        names.extend_from_slice(format!("f{:04}.bin\0", j).as_bytes());
    }
    for k in 0..d {
        names.extend_from_slice(format!("d{:05}\0", k).as_bytes());
    }
    let table_end = 16 + n as u64 * 16 + names.len() as u64;
    let data_start = align(table_end);
    let per = (total / f as u64).max(1);
    let mut entries = Vec::with_capacity(n as usize);
    entries.push(Entry { name: 0, kind: Kind::Dir { first: 1, count: d } });
    let (base, rem) = (f / d, f % d);
    let mut first = 1 + d;
    for k in 0..d {
        let cnt = base + if k < rem { 1 } else { 0 };
        entries.push(Entry { name: dir_name_off(k), kind: Kind::Dir { first, count: cnt } });
        first += cnt;
    }
    let mut cur = data_start;
    for j in 0..f {
        entries.push(Entry { name: file_name_off(j), kind: Kind::Bin { csize: 0, block: (cur / BLOCK) as u32, usize_: per as u32, enc: 0 } });
        cur += align(per);
    }
    let mut h = [0u8; 16];
    h[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    h[4..8].copy_from_slice(&n.to_le_bytes());
    h[8..12].copy_from_slice(&(names.len() as u32).to_le_bytes());
    h[12..16].copy_from_slice(&OPEN.to_le_bytes());
    w.write_all(&h)?;
    for e in &entries {
        w.write_all(&encode(e))?;
    }
    w.write_all(&names)?;
    let zeros = [0u8; 512];
    w.write_all(&zeros[..(data_start - table_end) as usize])?;
    let mut buf = vec![0u8; 65536];
    for j in 0..f {
        let mut s: u64 = (j as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut left = per as usize;
        while left > 0 {
            let m = left.min(buf.len());
            for c in buf[..m].chunks_mut(8) {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                let b = s.to_le_bytes();
                let l = c.len();
                c.copy_from_slice(&b[..l]);
            }
            w.write_all(&buf[..m])?;
            left -= m;
        }
        w.write_all(&zeros[..(align(per) - per) as usize])?;
    }
    w.flush()
}

/// Expected bytes of synthetic file number `j` (0-based among files), for tests.
pub fn expected_file_bytes(j: u32, len: usize) -> Vec<u8> {
    let mut s: u64 = (j as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut out = Vec::with_capacity(len + 8);
    while out.len() < len {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        out.extend_from_slice(&s.to_le_bytes());
    }
    out.truncate(len);
    out
}
