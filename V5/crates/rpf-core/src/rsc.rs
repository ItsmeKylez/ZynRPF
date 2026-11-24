//! RSC7 resource container (page flags, virtual pointers, decode and pack).
//!
//! DERIVED FROM the original Zyn-RPF / tlk-rpf core (`core/src/rsc.rs`, GPL-3.0, Copyright (c) 2026 Talkdedsec),
//! translated to English identifiers and adapted to this crate's own DEFLATE code. See LICENSE-GPL-NOTICE.md.
use crate::deflate::{deflate_stored, inflate};

pub const RSC7: u32 = 0x3743_5352;
const SYSTEM_BASE: u64 = 0x5000_0000;
const GRAPHICS_BASE: u64 = 0x6000_0000;
/// Upper bound for the decoded resource (decompression-bomb guard).
pub const MAX_DECODED: usize = 512 << 20;

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

/// (system flags, graphics flags) when `v` starts with an RSC7 header.
pub fn header(v: &[u8]) -> Option<(u32, u32)> {
    if u32_at(v, 0)? != RSC7 {
        return None;
    }
    Some((u32_at(v, 8)?, u32_at(v, 12)?))
}

const BUCKET_SHIFT: [u32; 9] = [4, 5, 7, 11, 17, 24, 25, 26, 27];
const BUCKET_MASK: [u32; 9] = [0x1, 0x3, 0xF, 0x3F, 0x7F, 0x1, 0x1, 0x1, 0x1];

fn bucket_counts(f: u32) -> [u32; 9] {
    std::array::from_fn(|i| (f >> BUCKET_SHIFT[i]) & BUCKET_MASK[i])
}

/// Section size from page flags: base page 0x200 << shift, nine buckets of page counts.
pub fn section_size(f: u32) -> usize {
    let base = 0x200usize << (f & 0xF);
    bucket_counts(f).iter().enumerate().map(|(i, &n)| n as usize * (base << (8 - i))).sum()
}

#[derive(Clone)]
pub struct Resource {
    pub version: u32,
    pub system_flags: u32,
    pub graphics_flags: u32,
    pub system_size: usize,
    pub data: Vec<u8>,
}

impl Resource {
    /// Decode a whole resource entry (16-byte RSC7 header followed by raw DEFLATE).
    pub fn decode(file: &[u8]) -> Result<Resource, String> {
        let (system_flags, graphics_flags) = header(file).ok_or("This is not an RSC7 resource.")?;
        let version = u32_at(file, 4).ok_or("The resource header is too short.")?;
        let system_size = section_size(system_flags);
        let total = system_size.checked_add(section_size(graphics_flags)).ok_or("The resource sizes overflow.")?;
        if total > MAX_DECODED {
            return Err("The resource is too large to decode here.".into());
        }
        let mut data = inflate(&file[16..], total.max(1)).map_err(|e| format!("The resource could not be decompressed: {e}"))?;
        if data.len() < system_size {
            return Err("The resource data is incomplete.".into());
        }
        data.resize(total, 0);
        Ok(Resource { version, system_flags, graphics_flags, system_size, data })
    }

    /// Header plus DEFLATE data. The data is written as stored blocks (valid, but not smaller than the input).
    pub fn pack(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.data.len() + 64);
        for x in [RSC7, self.version, self.system_flags, self.graphics_flags] {
            out.extend_from_slice(&x.to_le_bytes());
        }
        out.extend_from_slice(&deflate_stored(&self.data));
        out
    }

    /// Virtual pointer to a flat buffer offset.
    pub fn offset(&self, p: u64) -> Option<usize> {
        let o = (p & 0x0FFF_FFFF) as usize;
        let abs = match p & !0x0FFF_FFFF {
            SYSTEM_BASE if o < self.system_size => o,
            GRAPHICS_BASE => self.system_size + o,
            _ => return None,
        };
        (abs < self.data.len()).then_some(abs)
    }
}
