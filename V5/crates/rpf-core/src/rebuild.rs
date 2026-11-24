//! Edit operations and archive rebuild (M3).
//!
//! Operations are applied to an in-memory tree of the folder structure (names and entry descriptors only;
//! payload bytes are never loaded). Saving plans a new layout and streams it out: untouched payloads are
//! copied byte-for-byte from the source archive (D-006), new or replaced files come from a `BlobSource`.
//!
//! PARTIAL knowledge: alignment (512-byte blocks), children being contiguous and in a stable order, and
//! the names-table rules come from the same unverified format notes as the parser (D-011, D-035).
use crate::error::{Code, Fail};
use crate::format::*;
use crate::index::{Index, Limits};
use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom, Write};

/// One logged edit. Paths are absolute and slash-separated ("/" is the root).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Rename { path: String, new_name: String },
    Delete { path: String },
    Mkdir { parent: String, name: String },
    AddFile { parent: String, name: String, blob: u64, len: u64 },
    Replace { path: String, blob: u64, len: u64 },
    /// Replaces an RSC7 resource entry with new resource bytes (16-byte header + data). `sys`/`gfx` are the page flags from that header.
    ReplaceResource { path: String, blob: u64, len: u64, sys: u32, gfx: u32 },
}

/// Supplies the bytes of uploaded content referenced by `Op::AddFile` / `Op::Replace`.
pub trait BlobSource {
    fn open(&self, id: u64) -> io::Result<Box<dyn Read>>;
}

/// Names must be 1..=255 bytes of valid UTF-8 without control characters or `/ \ :`, and not "." or "..".
pub fn validate_name(n: &str) -> Result<(), Code> {
    if n.is_empty() || n.len() > 255 || n == "." || n == ".." {
        return Err(Code::NameInvalid);
    }
    if n.bytes().any(|c| c < 0x20 || c == 0x7f || c == b'/' || c == b'\\' || c == b':') {
        return Err(Code::NameInvalid);
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum Body {
    Dir(Vec<u32>),
    Orig(Entry),
    New { blob: u64, len: u64 },
    NewRes { blob: u64, len: u64, sys: u32, gfx: u32 },
}

#[derive(Debug, Clone)]
struct Node {
    name: String,
    parent: u32,
    body: Body,
}

/// Mutable view of the folder structure after applying operations.
pub struct Tree {
    nodes: Vec<Node>,
    live: u32,
    max_entries: u32,
    max_depth: u32,
    /// Original payload offsets per node (for copying), by node index.
    orig_span: HashMap<u32, (u64, u64)>,
}

/// (offset, length, entry id) of every payload in the archive, including large resources whose length is
/// the distance to the next payload (or to the end of the file).
pub fn payload_spans(idx: &Index) -> Vec<(u64, u64, u32)> {
    let mut v: Vec<(u64, u64, u32)> = Vec::new();
    let mut large: Vec<(u64, u32)> = Vec::new();
    for id in 0..idx.len() as u32 {
        if let Some((o, l)) = idx.stored_span(id) {
            v.push((o, l, id));
        } else if let Some(Entry { kind: Kind::Res { block, size, .. }, .. }) = idx.entry(id) {
            if *size == LARGE_RESOURCE {
                large.push((*block as u64 * BLOCK, id));
            }
        }
    }
    if !large.is_empty() {
        let mut starts: Vec<u64> = v.iter().filter(|x| x.1 > 0).map(|x| x.0).chain(large.iter().map(|x| x.0)).collect();
        starts.sort_unstable();
        starts.dedup();
        for (o, id) in large {
            let end = match starts.binary_search(&o) {
                Ok(i) => starts.get(i + 1).copied().unwrap_or(idx.file_len()),
                Err(_) => idx.file_len(),
            };
            v.push((o, end.saturating_sub(o), id));
        }
    }
    v
}

impl Tree {
    pub fn from_index(idx: &Index, lim: &Limits) -> Tree {
        let spans = payload_spans(idx);
        let mut large_len = HashMap::new();
        for (_, l, id) in &spans {
            if let Some(Entry { kind: Kind::Res { size, .. }, .. }) = idx.entry(*id) {
                if *size == LARGE_RESOURCE {
                    large_len.insert(*id, *l);
                }
            }
        }
        let mut nodes = vec![Node { name: String::new(), parent: u32::MAX, body: Body::Dir(Vec::new()) }];
        let mut stack: Vec<(u32, u32)> = vec![(0, 0)];
        let mut orig_span = HashMap::new();
        while let Some((node, eid)) = stack.pop() {
            let Some((first, count)) = idx.children(eid) else { continue };
            let mut kids = Vec::with_capacity(count as usize);
            for c in first..first + count {
                let e = *idx.entry(c).unwrap();
                let nid = nodes.len() as u32;
                let body = if matches!(e.kind, Kind::Dir { .. }) { Body::Dir(Vec::new()) } else { Body::Orig(e) };
                nodes.push(Node { name: idx.name(c).to_string(), parent: node, body });
                if matches!(e.kind, Kind::Dir { .. }) {
                    stack.push((nid, c));
                } else {
                    let l = large_len.get(&c).copied();
                    match (idx.stored_span(c), l) {
                        (Some(s), _) => {
                            orig_span.insert(nid, s);
                        }
                        (None, Some(len)) => {
                            if let Kind::Res { block, .. } = e.kind {
                                orig_span.insert(nid, (block as u64 * BLOCK, len));
                            }
                        }
                        _ => {}
                    }
                }
                kids.push(nid);
            }
            nodes[node as usize].body = Body::Dir(kids);
        }
        let live = nodes.len() as u32;
        Tree { nodes, live, max_entries: lim.max_entries, max_depth: lim.max_depth, orig_span }
    }

    /// Children of the folder at `path` as they would be after saving (used for the live preview of pending edits).
    /// Returns (name, is_folder, size in bytes, is_new) sorted by name, case-insensitively.
    pub fn list_folder(&self, path: &str) -> Result<Vec<(String, bool, u64, bool)>, Fail> {
        let n = self.resolve(path)?;
        let Body::Dir(kids) = &self.nodes[n as usize].body else {
            return Err(Fail::new(Code::EditNotAFolder, path));
        };
        let mut out: Vec<(String, bool, u64, bool)> = kids
            .iter()
            .map(|&k| {
                let node = &self.nodes[k as usize];
                match &node.body {
                    Body::Dir(_) => (node.name.clone(), true, 0, false),
                    Body::Orig(_) => (node.name.clone(), false, self.orig_span.get(&k).map(|s| s.1).unwrap_or(0), false),
                    Body::New { len, .. } | Body::NewRes { len, .. } => (node.name.clone(), false, *len, true),
                }
            })
            .collect();
        out.sort_by_key(|e| e.0.to_lowercase());
        Ok(out)
    }

    /// Number of entries the saved archive would contain.
    pub fn entry_count(&self) -> u32 {
        self.live
    }

    fn split(path: &str) -> Result<Vec<&str>, Fail> {
        if !path.starts_with('/') {
            return Err(Fail::new(Code::EditPathNotFound, path));
        }
        let parts: Vec<&str> = path.split('/').skip(1).filter(|s| !s.is_empty()).collect();
        Ok(parts)
    }

    fn resolve(&self, path: &str) -> Result<u32, Fail> {
        let mut cur = 0u32;
        for part in Self::split(path)? {
            let Body::Dir(kids) = &self.nodes[cur as usize].body else {
                return Err(Fail::new(Code::EditNotAFolder, path));
            };
            cur = *kids
                .iter()
                .find(|&&k| self.nodes[k as usize].name == part)
                .ok_or_else(|| Fail::new(Code::EditPathNotFound, path))?;
        }
        Ok(cur)
    }

    fn depth(&self, mut n: u32) -> u32 {
        let mut d = 0;
        while self.nodes[n as usize].parent != u32::MAX {
            n = self.nodes[n as usize].parent;
            d += 1;
        }
        d
    }

    fn name_taken(&self, dir: u32, name: &str, except: Option<u32>) -> bool {
        let lower = name.to_lowercase();
        if let Body::Dir(kids) = &self.nodes[dir as usize].body {
            kids.iter().any(|&k| Some(k) != except && self.nodes[k as usize].name.to_lowercase() == lower)
        } else {
            false
        }
    }

    fn subtree_size(&self, n: u32) -> u32 {
        let mut count = 0;
        let mut stack = vec![n];
        while let Some(x) = stack.pop() {
            count += 1;
            if let Body::Dir(k) = &self.nodes[x as usize].body {
                stack.extend(k.iter().copied());
            }
        }
        count
    }

    pub fn apply(&mut self, op: &Op) -> Result<(), Fail> {
        match op {
            Op::Rename { path, new_name } => {
                let n = self.resolve(path)?;
                if n == 0 {
                    return Err(Fail::new(Code::EditRootProtected, path));
                }
                validate_name(new_name).map_err(|c| Fail::new(c, new_name))?;
                if self.nodes[n as usize].name == *new_name {
                    return Err(Fail::new(Code::EditNoChange, path));
                }
                let p = self.nodes[n as usize].parent;
                if self.name_taken(p, new_name, Some(n)) {
                    return Err(Fail::new(Code::EditNameExists, new_name));
                }
                self.nodes[n as usize].name = new_name.clone();
            }
            Op::Delete { path } => {
                let n = self.resolve(path)?;
                if n == 0 {
                    return Err(Fail::new(Code::EditRootProtected, path));
                }
                let removed = self.subtree_size(n);
                let p = self.nodes[n as usize].parent;
                if let Body::Dir(kids) = &mut self.nodes[p as usize].body {
                    kids.retain(|&k| k != n);
                }
                self.live -= removed;
            }
            Op::Mkdir { parent, name } => {
                let p = self.resolve(parent)?;
                self.check_new_child(p, name, true)?;
                self.push_child(p, name, Body::Dir(Vec::new()));
            }
            Op::AddFile { parent, name, blob, len } => {
                let p = self.resolve(parent)?;
                if *len > u32::MAX as u64 {
                    return Err(Fail::new(Code::EditFileTooLarge, name));
                }
                self.check_new_child(p, name, false)?;
                self.push_child(p, name, Body::New { blob: *blob, len: *len });
            }
            Op::Replace { path, blob, len } => {
                let n = self.resolve(path)?;
                if *len > u32::MAX as u64 {
                    return Err(Fail::new(Code::EditFileTooLarge, path));
                }
                match &self.nodes[n as usize].body {
                    Body::Dir(_) => return Err(Fail::new(Code::EditTargetIsFolder, path)),
                    Body::Orig(Entry { kind: Kind::Res { .. }, .. }) | Body::NewRes { .. } => return Err(Fail::new(Code::EditResourceUnsupported, path)),
                    _ => {}
                }
                self.nodes[n as usize].body = Body::New { blob: *blob, len: *len };
            }
            Op::ReplaceResource { path, blob, len, sys, gfx } => {
                let n = self.resolve(path)?;
                if *len >= 0xFF_FFFF || *len < 16 {
                    return Err(Fail::new(Code::EditFileTooLarge, path));
                }
                match &self.nodes[n as usize].body {
                    Body::Dir(_) => return Err(Fail::new(Code::EditTargetIsFolder, path)),
                    Body::Orig(Entry { kind: Kind::Res { .. }, .. }) | Body::NewRes { .. } => {
                        self.nodes[n as usize].body = Body::NewRes { blob: *blob, len: *len, sys: *sys, gfx: *gfx };
                    }
                    // Not a resource entry: the content is plain bytes that merely start with the RSC7 magic.
                    _ => self.nodes[n as usize].body = Body::New { blob: *blob, len: *len },
                }
            }
        }
        Ok(())
    }

    fn check_new_child(&self, parent: u32, name: &str, is_dir: bool) -> Result<(), Fail> {
        if !matches!(self.nodes[parent as usize].body, Body::Dir(_)) {
            return Err(Fail::new(Code::EditNotAFolder, name));
        }
        validate_name(name).map_err(|c| Fail::new(c, name))?;
        if self.name_taken(parent, name, None) {
            return Err(Fail::new(Code::EditNameExists, name));
        }
        if self.live >= self.max_entries {
            return Err(Fail::new(Code::TooManyEntries, name));
        }
        if is_dir && self.depth(parent) + 1 > self.max_depth {
            return Err(Fail::new(Code::DepthExceeded, name));
        }
        Ok(())
    }

    fn push_child(&mut self, parent: u32, name: &str, body: Body) {
        let id = self.nodes.len() as u32;
        self.nodes.push(Node { name: name.to_string(), parent, body });
        if let Body::Dir(k) = &mut self.nodes[parent as usize].body {
            k.push(id);
        }
        self.live += 1;
    }
}

/// Applies `ops` in order; on failure returns the index of the failing operation.
pub fn replay(idx: &Index, ops: &[Op], lim: &Limits) -> Result<Tree, (usize, Fail)> {
    let mut t = Tree::from_index(idx, lim);
    for (i, op) in ops.iter().enumerate() {
        t.apply(op).map_err(|f| (i, f))?;
    }
    Ok(t)
}

#[derive(Debug, Clone, Copy)]
enum Payload {
    None,
    Orig { off: u64, len: u64 },
    Blob { id: u64, len: u64 },
}

/// A planned archive: final entry table, names table and payload sources.
pub struct Layout {
    entries: Vec<Entry>,
    names: Vec<u8>,
    payloads: Vec<(Payload, u64)>, // payload and its new byte offset
    pub file_len: u64,
    pub entry_count: u32,
    pub copied_entries: u32,
    pub new_entries: u32,
}

fn align(v: u64) -> u64 {
    (v + BLOCK - 1) / BLOCK * BLOCK
}

impl Tree {
    /// Plans the rebuilt archive and enforces all format limits. Writes nothing.
    pub fn plan(&self, lim: &Limits) -> Result<Layout, Fail> {
        // Breadth-first order: every folder's children are contiguous.
        let mut order: Vec<u32> = vec![0];
        let mut dirinfo: Vec<(u32, u32)> = Vec::new();
        let mut i = 0;
        while i < order.len() {
            let n = order[i];
            if let Body::Dir(kids) = &self.nodes[n as usize].body {
                dirinfo.push((order.len() as u32, kids.len() as u32));
                order.extend(kids.iter().copied());
            } else {
                dirinfo.push((0, 0));
            }
            i += 1;
            if order.len() as u64 > lim.max_entries as u64 {
                return Err(Fail::new(Code::TooManyEntries, format!("{} entries", order.len())));
            }
        }
        let n = order.len();
        // Names table: "" at offset 0, then file names (16-bit offsets), then folder names.
        let mut names: Vec<u8> = vec![0];
        let mut offs: HashMap<&str, u32> = HashMap::new();
        offs.insert("", 0);
        for pass in 0..2 {
            for &nid in &order[1..] {
                let node = &self.nodes[nid as usize];
                let is_dir = matches!(node.body, Body::Dir(_));
                if is_dir != (pass == 1) {
                    continue;
                }
                if !offs.contains_key(node.name.as_str()) {
                    let off = names.len() as u32;
                    if !is_dir && off > 0xFFFF {
                        return Err(Fail::new(Code::NameOffsetTooLarge, node.name.clone()));
                    }
                    names.extend_from_slice(node.name.as_bytes());
                    names.push(0);
                    offs.insert(node.name.as_str(), off);
                }
                if names.len() as u64 > lim.max_names_bytes as u64 {
                    return Err(Fail::new(Code::NamesTooLarge, format!("{} bytes", names.len())));
                }
            }
        }
        let table_end = 16 + n as u64 * 16 + names.len() as u64;
        let data_start = align(table_end);
        let mut cur = data_start;
        let mut entries = Vec::with_capacity(n);
        let mut payloads = Vec::with_capacity(n);
        let (mut copied, mut fresh) = (0u32, 0u32);
        for (k, &nid) in order.iter().enumerate() {
            let node = &self.nodes[nid as usize];
            let name = offs[node.name.as_str()];
            let place = |len: u64, cur: &mut u64| -> Result<u32, Fail> {
                if len == 0 {
                    return Ok((data_start / BLOCK) as u32);
                }
                let b = *cur / BLOCK;
                *cur += align(len);
                if b > 0x7F_FFFF || *cur / BLOCK > 0x7F_FFFF {
                    return Err(Fail::new(Code::ArchiveTooLarge, format!("{} bytes", *cur)));
                }
                Ok(b as u32)
            };
            match &node.body {
                Body::Dir(_) => {
                    let (first, count) = dirinfo[k];
                    entries.push(Entry { name, kind: Kind::Dir { first, count } });
                    payloads.push((Payload::None, 0));
                }
                Body::New { blob, len } => {
                    let block = place(*len, &mut cur)?;
                    entries.push(Entry { name, kind: Kind::Bin { csize: 0, block, usize_: *len as u32, enc: 0 } });
                    payloads.push((if *len > 0 { Payload::Blob { id: *blob, len: *len } } else { Payload::None }, block as u64 * BLOCK));
                    fresh += 1;
                }
                Body::NewRes { blob, len, sys, gfx } => {
                    let block = place(*len, &mut cur)?;
                    entries.push(Entry { name, kind: Kind::Res { size: *len as u32, block, sys: *sys, gfx: *gfx } });
                    payloads.push((Payload::Blob { id: *blob, len: *len }, block as u64 * BLOCK));
                    fresh += 1;
                }
                Body::Orig(e) => {
                    copied += 1;
                    let span = self.orig_span.get(&nid).copied();
                    let (len, keep_block) = match span {
                        Some((_, l)) if l > 0 => (l, None),
                        _ => (0, Some(match e.kind {
                            Kind::Bin { block, .. } | Kind::Res { block, .. } => block,
                            _ => 0,
                        })),
                    };
                    let block = match keep_block {
                        Some(b) => b,
                        None => place(len, &mut cur)?,
                    };
                    let kind = match e.kind {
                        Kind::Bin { csize, usize_, enc, .. } => Kind::Bin { csize, block, usize_, enc },
                        Kind::Res { size, sys, gfx, .. } => Kind::Res { size, block, sys, gfx },
                        d => d,
                    };
                    entries.push(Entry { name, kind });
                    match span {
                        Some((off, l)) if l > 0 => payloads.push((Payload::Orig { off, len: l }, block as u64 * BLOCK)),
                        _ => payloads.push((Payload::None, 0)),
                    }
                }
            }
        }
        Ok(Layout { entries, names, payloads, file_len: cur, entry_count: n as u32, copied_entries: copied, new_entries: fresh })
    }
}

impl Layout {
    /// Streams the archive. `src` must be the archive the tree was built from.
    pub fn write<R: Read + Seek, W: Write>(&self, src: &mut R, blobs: &dyn BlobSource, out: &mut W) -> Result<(), Fail> {
        let io = |e: io::Error| Fail::new(Code::Io, e.to_string());
        let mut h = [0u8; 16];
        h[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        h[4..8].copy_from_slice(&self.entry_count.to_le_bytes());
        h[8..12].copy_from_slice(&(self.names.len() as u32).to_le_bytes());
        h[12..16].copy_from_slice(&OPEN.to_le_bytes());
        out.write_all(&h).map_err(io)?;
        for e in &self.entries {
            out.write_all(&encode(e)).map_err(io)?;
        }
        out.write_all(&self.names).map_err(io)?;
        let mut pos = 16 + self.entries.len() as u64 * 16 + self.names.len() as u64;
        let zeros = [0u8; BLOCK as usize];
        let mut buf = vec![0u8; 1 << 20];
        let pad_to = |pos: &mut u64, to: u64, out: &mut W| -> Result<(), Fail> {
            while *pos < to {
                let m = ((to - *pos) as usize).min(zeros.len());
                out.write_all(&zeros[..m]).map_err(io)?;
                *pos += m as u64;
            }
            Ok(())
        };
        for (p, at) in &self.payloads {
            match *p {
                Payload::None => {}
                Payload::Orig { off, len } => {
                    pad_to(&mut pos, *at, out)?;
                    src.seek(SeekFrom::Start(off)).map_err(io)?;
                    let mut left = len;
                    while left > 0 {
                        let m = left.min(buf.len() as u64) as usize;
                        src.read_exact(&mut buf[..m]).map_err(io)?;
                        out.write_all(&buf[..m]).map_err(io)?;
                        left -= m as u64;
                    }
                    pos += len;
                }
                Payload::Blob { id, len } => {
                    pad_to(&mut pos, *at, out)?;
                    let mut r = blobs.open(id).map_err(|_| Fail::new(Code::BlobMissing, format!("content {id}")))?;
                    let mut got = 0u64;
                    loop {
                        let m = r.read(&mut buf).map_err(|_| Fail::new(Code::BlobMissing, format!("content {id}")))?;
                        if m == 0 {
                            break;
                        }
                        got += m as u64;
                        if got > len {
                            return Err(Fail::new(Code::BlobLengthMismatch, format!("content {id}")));
                        }
                        out.write_all(&buf[..m]).map_err(io)?;
                    }
                    if got != len {
                        return Err(Fail::new(Code::BlobLengthMismatch, format!("content {id}")));
                    }
                    pos += len;
                }
            }
        }
        pad_to(&mut pos, self.file_len, out)?;
        out.flush().map_err(io)?;
        if pos != self.file_len {
            return Err(Fail::new(Code::RebuildVerifyFailed, "written length differs from the plan"));
        }
        Ok(())
    }
}

/// Convenience: replay, plan and write in one call. Returns the layout summary.
pub fn rebuild<R: Read + Seek, W: Write>(
    idx: &Index,
    src: &mut R,
    ops: &[Op],
    blobs: &dyn BlobSource,
    lim: &Limits,
    out: &mut W,
) -> Result<Layout, Fail> {
    let tree = replay(idx, ops, lim).map_err(|(_, f)| f)?;
    let layout = tree.plan(lim)?;
    layout.write(src, blobs, out)?;
    Ok(layout)
}
