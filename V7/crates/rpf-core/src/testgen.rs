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


/// Text inside the `fixed.txt` entry of `gen_compressed_to` (stored compressed with real fixed-Huffman DEFLATE made by zlib).
pub const FIXED_TEXT: &str = "hello hello hello, RPF editor! hello hello\n";
/// Text inside the `stored.txt` entry (compressed with stored DEFLATE blocks).
pub const STORED_TEXT: &str = "line one\nline two\nline three\n";
const FIXED_DEFLATE_HEX: &str = "cb48cdc9c957c840903a0a41016e0aa9299925f9458ac8125c00";

/// Writes a tiny valid archive with three compressed files in the root folder, for testing compressed reads:
/// `bomb.bin` (data inflates to 100 bytes but the table says 10), `fixed.txt` (FIXED_TEXT) and `stored.txt` (STORED_TEXT).
pub fn gen_compressed_to<W: Write>(w: &mut W) -> io::Result<()> {
    let fixed: Vec<u8> = (0..FIXED_DEFLATE_HEX.len() / 2)
        .map(|i| u8::from_str_radix(&FIXED_DEFLATE_HEX[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    let payloads: [(Vec<u8>, u32); 3] = [
        (crate::deflate::deflate_stored(&[7u8; 100]), 10),
        (fixed, FIXED_TEXT.len() as u32),
        (crate::deflate::deflate_stored(STORED_TEXT.as_bytes()), STORED_TEXT.len() as u32),
    ];
    let names: &[u8] = b"\0bomb.bin\0fixed.txt\0stored.txt\0";
    let name_offs = [1u32, 10, 20];
    let table_end = 16 + 4 * 16 + names.len() as u64;
    let mut cur = align(table_end);
    let mut entries = vec![Entry { name: 0, kind: Kind::Dir { first: 1, count: 3 } }];
    let mut starts = Vec::new();
    for (i, (p, u)) in payloads.iter().enumerate() {
        starts.push(cur);
        entries.push(Entry { name: name_offs[i], kind: Kind::Bin { csize: p.len() as u32, block: (cur / BLOCK) as u32, usize_: *u, enc: 0 } });
        cur += align(p.len() as u64);
    }
    let mut h = [0u8; 16];
    h[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    h[4..8].copy_from_slice(&4u32.to_le_bytes());
    h[8..12].copy_from_slice(&(names.len() as u32).to_le_bytes());
    h[12..16].copy_from_slice(&OPEN.to_le_bytes());
    w.write_all(&h)?;
    for e in &entries {
        w.write_all(&encode(e))?;
    }
    w.write_all(names)?;
    let mut pos = table_end;
    for (i, (p, _)) in payloads.iter().enumerate() {
        w.write_all(&vec![0u8; (starts[i] - pos) as usize])?;
        w.write_all(p)?;
        pos = starts[i] + p.len() as u64;
    }
    w.write_all(&vec![0u8; (align(pos) - pos) as usize])?;
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

// ---- synthetic particle archive (M6 particle editor) ----
const S: u64 = 0x5000_0000;

fn put64(d: &mut [u8], o: usize, v: u64) { d[o..o + 8].copy_from_slice(&v.to_le_bytes()); }
fn put32(d: &mut [u8], o: usize, v: u32) { d[o..o + 4].copy_from_slice(&v.to_le_bytes()); }
fn put16(d: &mut [u8], o: usize, v: u16) { d[o..o + 2].copy_from_slice(&v.to_le_bytes()); }

/// A synthetic particle dictionary: one effect rule with one property of two keyframes.
pub fn sample_ypt_resource() -> Vec<u8> {
    let mut d = vec![0u8; 0x20000];
    d[0x1000..0x1008].copy_from_slice(b"test_fx\0");
    d[0x1100..0x1105].copy_from_slice(b"eff1\0");
    put64(&mut d, 0x10, S + 0x1000);
    put64(&mut d, 0x48, S + 0x2000);
    put64(&mut d, 0x2000 + 0x30, S + 0x3000);
    put16(&mut d, 0x2000 + 0x38, 1);
    put64(&mut d, 0x3000, S + 0x4000);
    put64(&mut d, 0x4000 + 0x20, S + 0x1100);
    let p = 0x4000 + 0xC0;
    put32(&mut d, p + 0x68, 0xd7c1e22b);
    put64(&mut d, p + 0x70, S + 0x5000);
    put16(&mut d, p + 0x78, 2);
    for (i, v) in [[0.0f32, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]].iter().enumerate() {
        for (j, x) in v.iter().enumerate() { d[0x5000 + i * 0x20 + j * 4..][..4].copy_from_slice(&x.to_le_bytes()); }
        for (j, x) in [0.5f32 + i as f32, 0.25, 0.125, 1.0].iter().enumerate() { d[0x5000 + i * 0x20 + 16 + j * 4..][..4].copy_from_slice(&x.to_le_bytes()); }
    }
    let r = crate::rsc::Kaynak { surum: 0x0000_0042, sistem: 0x10, grafik: 0, sistem_boyutu: crate::rsc::bolum_boyutu(0x10), veri: d };
    r.paketle()
}


/// Archive with one resource entry "test.ypt" (a synthetic particle dictionary).
pub fn gen_ypt_to<W: Write>(w: &mut W) -> io::Result<()> {
    let res = sample_ypt_resource();
    let names: &[u8] = b"\0test.ypt\0";
    let table_end = 16 + 2 * 16 + names.len() as u64;
    let start = align(table_end);
    let entries = [
        Entry { name: 0, kind: Kind::Dir { first: 1, count: 1 } },
        Entry { name: 1, kind: Kind::Res { size: res.len() as u32, block: (start / BLOCK) as u32, sys: 0x10, gfx: 0 } },
    ];
    let mut h = [0u8; 16];
    h[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    h[4..8].copy_from_slice(&2u32.to_le_bytes());
    h[8..12].copy_from_slice(&(names.len() as u32).to_le_bytes());
    h[12..16].copy_from_slice(&OPEN.to_le_bytes());
    w.write_all(&h)?;
    for e in &entries {
        w.write_all(&encode(e))?;
    }
    w.write_all(names)?;
    w.write_all(&vec![0u8; (start - table_end) as usize])?;
    w.write_all(&res)?;
    let end = start + res.len() as u64;
    w.write_all(&vec![0u8; (align(end) - end) as usize])?;
    w.flush()
}

/// Text of the synthetic visualsettings.dat (CRLF, comment, indented line, repeated key, two colour triples).
pub const VS_TEXT: &str = "# visual settings\r\nmisc.DamageOverlay.Duration 750.0f\r\n  car.headlight.color.red 255 # note\r\ncar.headlight.color.green 200\r\ncar.headlight.color.blue 100\r\nsky.sun.r 0.5\r\nsky.sun.g 0.25\r\nsky.sun.b 1.0\r\nmisc.DamageOverlay.Duration 750.0f\r\nbloom.threshold 0.8\r\n";

/// Archive with "visualsettings.dat" (stored) and "zz_visualsettings.dat" (compressed, same text).
pub fn gen_vs_to<W: Write>(w: &mut W) -> io::Result<()> {
    let names: &[u8] = b"\0visualsettings.dat\0zz_visualsettings.dat\0";
    let plain = VS_TEXT.as_bytes();
    let comp = crate::deflate::deflate_stored(plain);
    let table_end = 16 + 3 * 16 + names.len() as u64;
    let s1 = align(table_end);
    let s2 = s1 + align(plain.len() as u64);
    let entries = [
        Entry { name: 0, kind: Kind::Dir { first: 1, count: 2 } },
        Entry { name: 1, kind: Kind::Bin { csize: 0, block: (s1 / BLOCK) as u32, usize_: plain.len() as u32, enc: 0 } },
        Entry { name: 20, kind: Kind::Bin { csize: comp.len() as u32, block: (s2 / BLOCK) as u32, usize_: plain.len() as u32, enc: 0 } },
    ];
    let mut h = [0u8; 16];
    h[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    h[4..8].copy_from_slice(&3u32.to_le_bytes());
    h[8..12].copy_from_slice(&(names.len() as u32).to_le_bytes());
    h[12..16].copy_from_slice(&OPEN.to_le_bytes());
    w.write_all(&h)?;
    for e in &entries {
        w.write_all(&encode(e))?;
    }
    w.write_all(names)?;
    w.write_all(&vec![0u8; (s1 - table_end) as usize])?;
    w.write_all(plain)?;
    w.write_all(&vec![0u8; (s2 - s1 - plain.len() as u64) as usize])?;
    w.write_all(&comp)?;
    let end = s2 + comp.len() as u64;
    w.write_all(&vec![0u8; (align(end) - end) as usize])?;
    w.flush()
}

/// A synthetic texture dictionary file (RSC7): "red_dxt1" 32x32 DXT1 (solid red) and "grad_dxt5" 64x64 DXT5 (horizontal gradient with alpha).
pub fn sample_ytd_file() -> Vec<u8> {
    use crate::doku::{self, Bicim};
    fn tex(name: &str, w: usize, h: usize, b: Bicim, code: u32, rgba: Vec<u8>) -> rage_formats::YtdTexture {
        let mut levels = 1;
        while (w >> levels) >= 4 && (h >> levels) >= 4 { levels += 1; }
        let stride = b.seviye_boyutu(w, h).unwrap() / h;
        let (mut v, mut sw, mut sh) = (rgba, w, h);
        let mut data = Vec::new();
        let mut game = stride * h;
        for m in 0..levels {
            if m > 0 { let (y, a, c) = doku::yarim(&v, sw, sh); v = y; sw = a; sh = c; }
            let mut k = doku::kodla(b, &v, sw, sh).unwrap();
            k.resize(game, 0);
            data.extend(k);
            game /= 4;
        }
        rage_formats::YtdTexture { name: name.into(), name_hash: crate::ytd::joaat(name), width: w as u16, height: h as u16, depth: 1, format: rage_formats::TextureFormat::from_u32(code), levels: levels as u8, stride: stride as u16, pixel_data: data }
    }
    let red: Vec<u8> = (0..32 * 32).flat_map(|_| [255u8, 0, 0, 255]).collect();
    let grad: Vec<u8> = (0..64 * 64).flat_map(|i| { let x = (i % 64) as u8; [x * 4, 255 - x * 4, 64, 128 + x] }).collect();
    let ts = vec![tex("red_dxt1", 32, 32, Bicim::Dxt1, 0x3154_5844, red), tex("grad_dxt5", 64, 64, Bicim::Dxt5, 0x3554_5844, grad)];
    rage_formats::serialize_ytd(&ts).unwrap()
}

/// Archive with one resource entry "test.ytd" (a synthetic texture dictionary).
pub fn gen_ytd_to<W: Write>(w: &mut W) -> io::Result<()> {
    let res = sample_ytd_file();
    let sys = u32::from_le_bytes(res[8..12].try_into().unwrap());
    let gfx = u32::from_le_bytes(res[12..16].try_into().unwrap());
    let names: &[u8] = b"\0test.ytd\0";
    let table_end = 16 + 2 * 16 + names.len() as u64;
    let start = align(table_end);
    let entries = [
        Entry { name: 0, kind: Kind::Dir { first: 1, count: 1 } },
        Entry { name: 1, kind: Kind::Res { size: res.len() as u32, block: (start / BLOCK) as u32, sys, gfx } },
    ];
    let mut h = [0u8; 16];
    h[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    h[4..8].copy_from_slice(&2u32.to_le_bytes());
    h[8..12].copy_from_slice(&(names.len() as u32).to_le_bytes());
    h[12..16].copy_from_slice(&OPEN.to_le_bytes());
    w.write_all(&h)?;
    for e in &entries { w.write_all(&encode(e))?; }
    w.write_all(names)?;
    w.write_all(&vec![0u8; (start - table_end) as usize])?;
    w.write_all(&res)?;
    let end = start + res.len() as u64;
    w.write_all(&vec![0u8; (align(end) - end) as usize])?;
    w.flush()
}
