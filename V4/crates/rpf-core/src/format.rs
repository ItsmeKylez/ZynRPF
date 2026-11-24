//! Raw RPF7 constants and 16-byte entry encoding. PARTIAL knowledge: unverified on real archives.

pub const MAGIC: u32 = 0x5250_4637; // "RPF7"
pub const OPEN: u32 = 0x4E45_504F; // unencrypted marker
pub const DIR_MARK: u32 = 0x7FFF_FF00;
pub const BLOCK: u64 = 512;
/// Resource size field value meaning "size stored elsewhere" (large resource).
pub const LARGE_RESOURCE: u32 = 0xFF_FFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Dir { first: u32, count: u32 },
    Bin { csize: u32, block: u32, usize_: u32, enc: u32 },
    Res { size: u32, block: u32, sys: u32, gfx: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub name: u32,
    pub kind: Kind,
}

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn u24(b: &[u8], o: usize) -> u32 {
    b[o] as u32 | (b[o + 1] as u32) << 8 | (b[o + 2] as u32) << 16
}
fn put24(b: &mut [u8], v: u32) {
    b[0] = v as u8;
    b[1] = (v >> 8) as u8;
    b[2] = (v >> 16) as u8;
}

pub fn decode(b: &[u8]) -> Entry {
    let y = u32le(b, 0);
    if u32le(b, 4) == DIR_MARK {
        return Entry { name: y, kind: Kind::Dir { first: u32le(b, 8), count: u32le(b, 12) } };
    }
    let name = y & 0xFFFF;
    let size = u24(b, 2);
    let off = u24(b, 5);
    if off & 0x80_0000 != 0 {
        Entry { name, kind: Kind::Res { size, block: off & 0x7F_FFFF, sys: u32le(b, 8), gfx: u32le(b, 12) } }
    } else {
        Entry { name, kind: Kind::Bin { csize: size, block: off, usize_: u32le(b, 8), enc: u32le(b, 12) } }
    }
}

pub fn encode(e: &Entry) -> [u8; 16] {
    let mut b = [0u8; 16];
    match e.kind {
        Kind::Dir { first, count } => {
            b[0..4].copy_from_slice(&e.name.to_le_bytes());
            b[4..8].copy_from_slice(&DIR_MARK.to_le_bytes());
            b[8..12].copy_from_slice(&first.to_le_bytes());
            b[12..16].copy_from_slice(&count.to_le_bytes());
        }
        Kind::Bin { csize, block, usize_, enc } => {
            b[0..2].copy_from_slice(&(e.name as u16).to_le_bytes());
            put24(&mut b[2..5], csize);
            put24(&mut b[5..8], block);
            b[8..12].copy_from_slice(&usize_.to_le_bytes());
            b[12..16].copy_from_slice(&enc.to_le_bytes());
        }
        Kind::Res { size, block, sys, gfx } => {
            b[0..2].copy_from_slice(&(e.name as u16).to_le_bytes());
            put24(&mut b[2..5], size);
            put24(&mut b[5..8], block | 0x80_0000);
            b[8..12].copy_from_slice(&sys.to_le_bytes());
            b[12..16].copy_from_slice(&gfx.to_le_bytes());
        }
    }
    b
}
