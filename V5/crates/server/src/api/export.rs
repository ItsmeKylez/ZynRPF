//! "Export all": the contents of one archive version as a ZIP file (stored method, no compression).
//! Built in memory and bounded by MAX_BLOB_BYTES. Compressed plain files are inflated; resources,
//! encrypted files and anything that cannot be inflated are written as their stored bytes, and the
//! ZIP then carries an `_export_notes.txt` that says which entries were treated that way.
use super::*;
use std::collections::HashSet;

const MAX_FILES: usize = 20_000;

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

/// Makes one path component safe for a ZIP: no separators, no control characters, no "." or "..".
fn clean(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_control() || c == '/' || c == '\\' || c == ':' { '_' } else { c }).collect();
    let t = t.trim().to_string();
    if t.is_empty() || t == "." || t == ".." { "_".to_string() } else { t }
}

struct Zip {
    out: Vec<u8>,
    central: Vec<u8>,
    count: u32,
}
impl Zip {
    fn new() -> Zip { Zip { out: Vec::new(), central: Vec::new(), count: 0 } }
    fn add(&mut self, name: &str, data: &[u8]) {
        let crc = crc32(data);
        let off = self.out.len() as u32;
        let nb = name.as_bytes();
        // local header
        self.out.extend_from_slice(&0x0403_4B50u32.to_le_bytes());
        self.out.extend_from_slice(&[20, 0, 0x00, 0x08, 0, 0, 0, 0, 0x21, 0]); // version, flags (UTF-8), method 0, time 0, date 1980-01-01
        self.out.extend_from_slice(&crc.to_le_bytes());
        self.out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.out.extend_from_slice(&(nb.len() as u16).to_le_bytes());
        self.out.extend_from_slice(&0u16.to_le_bytes());
        self.out.extend_from_slice(nb);
        self.out.extend_from_slice(data);
        // central directory record
        let c = &mut self.central;
        c.extend_from_slice(&0x0201_4B50u32.to_le_bytes());
        c.extend_from_slice(&[20, 0, 20, 0, 0x00, 0x08, 0, 0, 0, 0, 0x21, 0]);
        c.extend_from_slice(&crc.to_le_bytes());
        c.extend_from_slice(&(data.len() as u32).to_le_bytes());
        c.extend_from_slice(&(data.len() as u32).to_le_bytes());
        c.extend_from_slice(&(nb.len() as u16).to_le_bytes());
        c.extend_from_slice(&0u16.to_le_bytes()); // extra
        c.extend_from_slice(&0u16.to_le_bytes()); // comment
        c.extend_from_slice(&0u16.to_le_bytes()); // disk
        c.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        c.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        c.extend_from_slice(&off.to_le_bytes());
        c.extend_from_slice(nb);
        self.count += 1;
    }
    fn finish(mut self) -> Vec<u8> {
        let start = self.out.len() as u32;
        let csize = self.central.len() as u32;
        self.out.extend_from_slice(&self.central);
        self.out.extend_from_slice(&0x0605_4B50u32.to_le_bytes());
        self.out.extend_from_slice(&[0, 0, 0, 0]);
        self.out.extend_from_slice(&(self.count as u16).to_le_bytes());
        self.out.extend_from_slice(&(self.count as u16).to_le_bytes());
        self.out.extend_from_slice(&csize.to_le_bytes());
        self.out.extend_from_slice(&start.to_le_bytes());
        self.out.extend_from_slice(&0u16.to_le_bytes());
        self.out
    }
}

impl State {
    pub(super) fn export_zip(&self, sid: &str, vid: &str) -> Result<Resp, ApiErr> {
        let (idx, path) = self.index_for(sid, vid)?;
        let cap = self.cfg.max_blob;
        let too_big = || err(413, "export_too_large", format!("The export would be larger than {} bytes or have more than {} files.", cap, MAX_FILES), "Download single files instead, or the whole archive.");
        let mut f = File::open(&path).map_err(|_| not_found())?;
        let mut zip = Zip::new();
        let mut notes: Vec<String> = Vec::new();
        let mut used: HashSet<String> = HashSet::new();
        let mut files = 0usize;
        // depth-first walk from the root (id 0); stack holds (folder id, zip prefix)
        let mut stack: Vec<(u32, String, u32)> = vec![(0, String::new(), 0)];
        while let Some((dir, prefix, depth)) = stack.pop() {
            let Some((first, count)) = idx.children(dir) else { continue };
            if depth > 64 {
                notes.push(format!("{}: folder nesting deeper than 64 levels was not exported.", prefix));
                continue;
            }
            for id in first..first.saturating_add(count) {
                let Some(info) = idx.info(id) else { continue };
                let mut name = clean(&info.name);
                let mut full = format!("{}{}", prefix, name);
                let mut n = 1;
                while !used.insert(full.to_lowercase()) {
                    n += 1;
                    name = format!("{} ({})", clean(&info.name), n);
                    full = format!("{}{}", prefix, name);
                }
                if info.kind == "directory" {
                    stack.push((id, format!("{}/", full), depth + 1));
                    continue;
                }
                files += 1;
                if files > MAX_FILES { return Err(too_big()); }
                if info.large_resource {
                    notes.push(format!("{}: skipped (large-size resource marker is not supported yet).", full));
                    continue;
                }
                let (off, len) = idx.stored_span(id).ok_or_else(not_found)?;
                if zip.out.len() as u64 + len > cap { return Err(too_big()); }
                f.seek(SeekFrom::Start(off)).map_err(|_| not_found())?;
                let mut stored = vec![0u8; len as usize];
                f.read_exact(&mut stored).map_err(|_| err(422, "entry_unreadable", format!("The data of {} could not be read from the archive.", full), "Validate the archive."))?;
                if info.kind == "file" && info.compressed {
                    if info.encryption_tag != Some(0) {
                        notes.push(format!("{}: written as stored bytes (marked encrypted, cannot be decompressed).", full));
                    } else if info.uncompressed_size > cap {
                        notes.push(format!("{}: written as stored bytes (too large to decompress).", full));
                    } else {
                        match rpf_core::deflate::inflate(&stored, info.uncompressed_size as usize) {
                            Ok(p) if p.len() as u64 == info.uncompressed_size => { stored = p; }
                            _ => notes.push(format!("{}: written as stored bytes (decompression failed).", full)),
                        }
                    }
                    if zip.out.len() + stored.len() > cap as usize { return Err(too_big()); }
                } else if info.kind == "resource" {
                    notes.push(format!("{}: resource written as its stored bytes (compressed resource payload, as in the archive).", full));
                }
                zip.add(&full, &stored);
            }
        }
        if !notes.is_empty() {
            let text = format!("Export notes\n\n{}\n", notes.join("\n"));
            zip.add("_export_notes.txt", text.as_bytes());
        }
        let fname = {
            let g = self.inner.lock().unwrap();
            g.vers.get(vid).map(|v| safe_filename(&v.filename)).unwrap_or("archive.rpf".into())
        };
        let data = zip.finish();
        Ok(Resp {
            status: 200,
            headers: vec![("Content-Disposition".into(), format!("attachment; filename=\"{}.zip\"", fname))],
            body: Body::Bytes { data: Arc::new(data), ctype: "application/zip" },
        })
    }
}
