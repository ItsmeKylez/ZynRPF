//! Validated, immutable archive index. Only the index (header, entry table, names) is held in memory;
//! payload bytes are never loaded here.
use crate::error::Code;
use crate::format::*;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_entries: u32,
    pub max_names_bytes: u32,
    pub max_depth: u32,
}
impl Default for Limits {
    fn default() -> Self {
        Limits { max_entries: 1_000_000, max_names_bytes: 65_536, max_depth: 128 }
    }
}

pub struct Index {
    entries: Vec<Entry>,
    parent: Vec<u32>,
    names: Vec<u8>,
    file_len: u64,
    table_end: u64,
    orphans: u32,
}

/// Plain description of one entry for API output.
#[derive(Debug, Clone)]
pub struct EntryInfo {
    pub id: u32,
    pub name: String,
    pub kind: &'static str, // "directory" | "file" | "resource"
    pub stored_size: u64,
    pub uncompressed_size: u64,
    pub compressed: bool,
    pub encryption_tag: Option<u32>,
    pub offset: u64,
    pub child_count: Option<u32>,
    pub sys_flags: Option<u32>,
    pub gfx_flags: Option<u32>,
    pub large_resource: bool,
}

fn check_name(names: &[u8], off: u32, is_root: bool) -> Result<(), Code> {
    let off = off as usize;
    if off >= names.len() {
        return Err(Code::NameOffsetOob);
    }
    let rest = &names[off..];
    let end = rest.iter().position(|&c| c == 0).ok_or(Code::NameUnterminated)?;
    let n = &rest[..end];
    if n.is_empty() {
        return if is_root { Ok(()) } else { Err(Code::NameInvalid) };
    }
    if n == b"." || n == b".." {
        return Err(Code::NameInvalid);
    }
    std::str::from_utf8(n).map_err(|_| Code::NameInvalid)?;
    if n.iter().any(|&c| c < 0x20 || c == 0x7f || c == b'/' || c == b'\\' || c == b':') {
        return Err(Code::NameInvalid);
    }
    Ok(())
}

fn span_of(k: &Kind) -> Option<(u64, u64)> {
    match *k {
        Kind::Bin { csize, block, usize_, .. } => Some((block as u64 * BLOCK, if csize == 0 { usize_ } else { csize } as u64)),
        Kind::Res { size, block, .. } if size != LARGE_RESOURCE => Some((block as u64 * BLOCK, size as u64)),
        _ => None,
    }
}

impl Index {
    pub fn parse<R: Read + Seek>(r: &mut R, lim: &Limits) -> Result<Index, Code> {
        let io = |_| Code::Io;
        let file_len = r.seek(SeekFrom::End(0)).map_err(io)?;
        if file_len < 16 {
            return Err(Code::TooShort);
        }
        r.seek(SeekFrom::Start(0)).map_err(io)?;
        let mut h = [0u8; 16];
        r.read_exact(&mut h).map_err(io)?;
        let g = |o: usize| u32::from_le_bytes([h[o], h[o + 1], h[o + 2], h[o + 3]]);
        let (magic, count, names_len, enc) = (g(0), g(4), g(8), g(12));
        if magic != MAGIC {
            return Err(Code::BadMagic);
        }
        if enc != OPEN {
            return Err(Code::EncryptedUnsupported);
        }
        if count > lim.max_entries {
            return Err(Code::TooManyEntries);
        }
        if names_len > lim.max_names_bytes {
            return Err(Code::NamesTooLarge);
        }
        let table_end = 16 + count as u64 * 16 + names_len as u64;
        if table_end > file_len {
            return Err(Code::TableOutOfBounds); // checked BEFORE allocating
        }
        let mut tbl = vec![0u8; count as usize * 16];
        r.read_exact(&mut tbl).map_err(io)?;
        let mut names = vec![0u8; names_len as usize];
        r.read_exact(&mut names).map_err(io)?;

        let n = count as usize;
        let mut entries = Vec::with_capacity(n);
        let mut name_ok = vec![false; names.len()];
        for i in 0..n {
            let e = decode(&tbl[i * 16..i * 16 + 16]);
            if i == 0 {
                check_name(&names, e.name, true)?;
            } else if (e.name as usize) >= names.len() || !name_ok[e.name as usize] {
                check_name(&names, e.name, false)?;
                name_ok[e.name as usize] = true;
            }
            entries.push(e);
        }
        drop(tbl);
        if n == 0 || !matches!(entries[0].kind, Kind::Dir { .. }) {
            return Err(Code::NoRoot);
        }

        let mut visited = vec![false; n];
        let mut parent = vec![u32::MAX; n];
        let mut depth = vec![0u32; n];
        visited[0] = true;
        let mut reached: u32 = 1;
        let mut stack = vec![0u32];
        while let Some(i) = stack.pop() {
            let k = entries[i as usize].kind;
            if let Kind::Dir { first, count } = k {
                if first as u64 + count as u64 > n as u64 {
                    return Err(Code::DirRangeOob);
                }
                for c in first..first + count {
                    if visited[c as usize] {
                        let mut a = i;
                        loop {
                            if a == c {
                                return Err(Code::TableCycle);
                            }
                            let p = parent[a as usize];
                            if p == u32::MAX {
                                break;
                            }
                            a = p;
                        }
                        return Err(Code::SharedChild);
                    }
                    visited[c as usize] = true;
                    reached += 1;
                    parent[c as usize] = i;
                    depth[c as usize] = depth[i as usize] + 1;
                    if depth[c as usize] > lim.max_depth {
                        return Err(Code::DepthExceeded);
                    }
                    stack.push(c);
                }
            } else if let Some((block, len)) = span_of(&k).map(|(o, l)| (o, l)) {
                if len > 0 {
                    if block < table_end {
                        return Err(Code::DataInTable);
                    }
                    if block.checked_add(len).map_or(true, |e| e > file_len) {
                        return Err(Code::DataOob);
                    }
                }
            }
        }
        Ok(Index { entries, parent, names, file_len, table_end, orphans: n as u32 - reached })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn file_len(&self) -> u64 {
        self.file_len
    }
    pub fn table_end(&self) -> u64 {
        self.table_end
    }
    /// Entries not reachable from the root (accepted, but reported).
    pub fn orphans(&self) -> u32 {
        self.orphans
    }
    pub fn entry(&self, id: u32) -> Option<&Entry> {
        self.entries.get(id as usize)
    }
    pub fn parent(&self, id: u32) -> Option<u32> {
        match self.parent.get(id as usize) {
            Some(&p) if p != u32::MAX => Some(p),
            _ => None,
        }
    }
    pub fn name(&self, id: u32) -> &str {
        let Some(e) = self.entries.get(id as usize) else { return "" };
        let rest = &self.names[(e.name as usize).min(self.names.len())..];
        let end = rest.iter().position(|&c| c == 0).unwrap_or(rest.len());
        std::str::from_utf8(&rest[..end]).unwrap_or("")
    }
    /// Children as (first, count) when `id` is a folder.
    pub fn children(&self, id: u32) -> Option<(u32, u32)> {
        match self.entries.get(id as usize)?.kind {
            Kind::Dir { first, count } => Some((first, count)),
            _ => None,
        }
    }
    /// Slash-separated path from the root ("/" for the root).
    pub fn path(&self, id: u32) -> String {
        let mut parts = Vec::new();
        let mut cur = id;
        let mut guard = 0;
        while let Some(p) = self.parent(cur) {
            parts.push(self.name(cur));
            cur = p;
            guard += 1;
            if guard > 1024 {
                break;
            }
        }
        if parts.is_empty() {
            return "/".to_string();
        }
        parts.reverse();
        format!("/{}", parts.join("/"))
    }
    /// (offset, length) of the stored payload bytes in the archive file.
    pub fn stored_span(&self, id: u32) -> Option<(u64, u64)> {
        span_of(&self.entries.get(id as usize)?.kind)
    }
    pub fn info(&self, id: u32) -> Option<EntryInfo> {
        let e = self.entries.get(id as usize)?;
        let name = self.name(id).to_string();
        let mut i = EntryInfo {
            id, name, kind: "directory", stored_size: 0, uncompressed_size: 0, compressed: false,
            encryption_tag: None, offset: 0, child_count: None, sys_flags: None, gfx_flags: None, large_resource: false,
        };
        match e.kind {
            Kind::Dir { count, .. } => i.child_count = Some(count),
            Kind::Bin { csize, block, usize_, enc } => {
                i.kind = "file";
                i.compressed = csize != 0;
                i.stored_size = if csize == 0 { usize_ } else { csize } as u64;
                i.uncompressed_size = usize_ as u64;
                i.encryption_tag = Some(enc);
                i.offset = block as u64 * BLOCK;
            }
            Kind::Res { size, block, sys, gfx } => {
                i.kind = "resource";
                i.large_resource = size == LARGE_RESOURCE;
                i.stored_size = if i.large_resource { 0 } else { size as u64 };
                i.offset = block as u64 * BLOCK;
                i.sys_flags = Some(sys);
                i.gfx_flags = Some(gfx);
                i.compressed = true; // resource payloads are stored compressed
            }
        }
        Some(i)
    }
    /// Case-insensitive name search; returns entry ids in table order, at most `limit`.
    pub fn search(&self, q: &str, limit: usize) -> Vec<u32> {
        let q = q.to_lowercase();
        let mut out = Vec::new();
        if q.is_empty() {
            return out;
        }
        for id in 1..self.entries.len() as u32 {
            if self.name(id).to_lowercase().contains(&q) {
                out.push(id);
                if out.len() >= limit {
                    break;
                }
            }
        }
        out
    }
}
