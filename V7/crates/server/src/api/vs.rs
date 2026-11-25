//! visualsettings.dat routes: read the settings and colour triples, and apply value changes.
//! Uses rpf_core::vs (copied verbatim from the original Zyn-RPF core, GPL-3.0; see LICENSE-GPL-NOTICE.md).
//! The original core has Turkish names; this file maps them to the English JSON of the web API.
use super::*;

const MAX_VS_CHANGES: u64 = 4 * 1024 * 1024;
const MAX_VS_BYTES: u64 = 8 * 1024 * 1024;

impl State {
    /// Plain text of a file entry (inflated when compressed). Encrypted, large or non-UTF-8 files are rejected.
    fn vs_load(&self, base: Option<Vec<u8>>, sid: &str, vid: &str, eid: &str) -> Result<String, ApiErr> {
        let (idx, path) = self.index_for(sid, vid)?;
        let id = parse_id(&idx, eid)?;
        let info = idx.info(id).ok_or_else(not_found)?;
        if info.kind != "file" {
            return Err(err(422, "not_a_file", "This entry is not a plain file.", "Choose a visualsettings.dat file."));
        }
        if info.encryption_tag != Some(0) && info.compressed || (!info.compressed && info.encryption_tag.map_or(false, |t| t != 0)) {
            return Err(err(422, "entry_encrypted", "This file is marked as encrypted, so it cannot be read.", ""));
        }
        if let Some(b) = base {
            // Queued content is plain text already.
            if b.len() as u64 > MAX_VS_BYTES {
                return Err(err(422, "too_large_to_decode", "This file is too large to open here.", ""));
            }
            return String::from_utf8(b).map_err(|_| err(422, "not_text", "This file is not UTF-8 text.", "Use the text editor or download the bytes."));
        }
        let (off, len) = idx.stored_span(id).ok_or_else(not_found)?;
        let declared = if info.compressed { info.uncompressed_size } else { len };
        if declared > MAX_VS_BYTES || len > MAX_VS_BYTES || len > self.cfg.max_blob {
            return Err(err(422, "too_large_to_decode", "This file is too large to open here.", ""));
        }
        let mut f = File::open(&path).map_err(|_| not_found())?;
        f.seek(SeekFrom::Start(off)).map_err(|_| not_found())?;
        let mut stored = vec![0u8; len as usize];
        f.read_exact(&mut stored).map_err(|_| err(422, "entry_unreadable", "The file's data could not be read from the archive.", ""))?;
        let plain = if info.compressed {
            let p = rpf_core::deflate::inflate(&stored, declared as usize).map_err(|e| err(422, "inflate_failed", e.to_string(), "Download the stored bytes instead."))?;
            if p.len() as u64 != declared {
                return Err(err(422, "inflate_size_mismatch", "The decompressed size does not match the size in the archive.", ""));
            }
            p
        } else {
            stored
        };
        String::from_utf8(plain).map_err(|_| err(422, "not_text", "This file is not UTF-8 text.", "Use the text editor or download the bytes."))
    }

    /// GET /versions/{vid}/entries/{eid}/vs
    pub(super) fn vs_read(&self, req: &Req, sid: &str, vid: &str, eid: &str) -> Result<Resp, ApiErr> {
        let text = self.vs_load(self.base_blob(req, sid)?, sid, vid, eid)?;
        let s = rpf_core::vs::Ayarlar::coz(&text);
        let entries: Vec<String> = s.girdiler().iter().map(|g| format!("{{\"key\":{},\"value\":{},\"line\":{},\"repeats\":{}}}", q(&g.anahtar), q(&g.deger), g.satir, g.tekrar)).collect();
        let colours: Vec<String> = s.renkler().iter().map(|c| format!("{{\"base\":{},\"keys\":[{},{},{}],\"scale\":{}}}", q(&c.taban), q(&c.anahtarlar[0]), q(&c.anahtarlar[1]), q(&c.anahtarlar[2]), if c.olcek == 255.0 { 255 } else { 1 })).collect();
        Ok(Resp::json(200, format!("{{\"entries\":[{}],\"colours\":[{}]}}", entries.join(","), colours.join(","))))
    }

    /// POST /versions/{vid}/entries/{eid}/vs/edit — body: one change per line, "key<TAB>value".
    /// Returns the edited file bytes; the web UI queues them as a normal replace operation.
    pub(super) fn vs_edit(&self, req: &mut Req, sid: &str, vid: &str, eid: &str) -> Result<Resp, ApiErr> {
        let body = req.body(MAX_VS_CHANGES)?;
        let text = String::from_utf8(body).map_err(|_| bad("invalid_request", "The request body is not valid text.", "Reload the page and try again."))?;
        let mut changes: Vec<(&str, &str)> = Vec::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let (k, v) = line.split_once('\t').ok_or_else(|| bad("invalid_request", "A change line is not valid.", "Reload the page and try again."))?;
            let v = v.trim();
            if k.is_empty() || v.is_empty() || v.contains(char::is_whitespace) || k.contains(char::is_whitespace) {
                return Err(bad("invalid_value", "A value must be a single word or number without spaces.", "Correct the value and try again."));
            }
            changes.push((k, v));
        }
        if changes.is_empty() {
            return Err(bad("invalid_request", "There are no changes to apply.", "Change a value first."));
        }
        let original = self.vs_load(self.base_blob(req, sid)?, sid, vid, eid)?;
        let mut s = rpf_core::vs::Ayarlar::coz(&original);
        for (k, v) in &changes {
            if s.deger(k).is_none() {
                return Err(err(422, "unknown_setting", "A setting in the request does not exist in this file.", "Reload the settings and try again."));
            }
            s.ayarla(k, v);
        }
        let out = s.metin().into_bytes();
        if out.len() as u64 > self.cfg.max_blob {
            return Err(err(413, "limit_exceeded", "The edited file is larger than the limit for one added file.", ""));
        }
        Ok(Resp { status: 200, headers: vec![], body: Body::Bytes { data: Arc::new(out), ctype: "application/octet-stream" } })
    }
}
