//! Particle (.ypt) routes: read the effects of a stored resource entry, and write changed keyframe values.
//! Uses rpf_core::rsc and rpf_core::ypt (copied verbatim from the original Zyn-RPF core, GPL-3.0; see LICENSE-GPL-NOTICE.md).
//! The original core has Turkish names and messages; this file maps them to the English JSON of the web API and to English errors.
use super::*;

/// Change lines are about 70 bytes each and up to 50,000 are accepted, so the 16 KiB JSON cap is far too small here.
const MAX_YPT_CHANGES: u64 = 8 * 1024 * 1024;

impl State {
    /// Stored bytes of a resource entry, decoded. Large-size resources and non-resources are rejected.
    fn ypt_load(&self, base: Option<Vec<u8>>, sid: &str, vid: &str, eid: &str) -> Result<rpf_core::rsc::Kaynak, ApiErr> {
        let (idx, path) = self.index_for(sid, vid)?;
        let id = parse_id(&idx, eid)?;
        let info = idx.info(id).ok_or_else(not_found)?;
        if info.kind != "resource" {
            return Err(err(422, "not_a_resource", "This entry is not a game resource, so it has no particle data.", "Choose a .ypt file."));
        }
        if info.large_resource {
            return Err(err(422, "large_resource_unsupported", "This resource uses the large-size marker, which is not supported yet.", ""));
        }
        let stored = match base {
            Some(b) => b,
            None => {
                let (off, len) = idx.stored_span(id).ok_or_else(not_found)?;
                if len > self.cfg.max_blob {
                    return Err(err(422, "too_large_to_decode", "This resource is too large to open here.", ""));
                }
                let mut f = File::open(&path).map_err(|_| not_found())?;
                f.seek(SeekFrom::Start(off)).map_err(|_| not_found())?;
                let mut stored = vec![0u8; len as usize];
                f.read_exact(&mut stored).map_err(|_| err(422, "entry_unreadable", "The resource data could not be read from the archive.", ""))?;
                stored
            }
        };
        rpf_core::rsc::Kaynak::coz(&stored).map_err(|_| err(422, "resource_invalid", "The resource could not be decoded. It may be damaged, encrypted or too large.", "Download the entry bytes instead."))
    }

    /// GET /versions/{vid}/entries/{eid}/ypt
    pub(super) fn ypt_read(&self, req: &Req, sid: &str, vid: &str, eid: &str) -> Result<Resp, ApiErr> {
        let r = self.ypt_load(self.base_blob(req, sid)?, sid, vid, eid)?;
        let groups = rpf_core::ypt::oku(&r).map_err(|_| err(422, "not_a_ypt", "This is not a particle dictionary (.ypt).", "Choose a particle dictionary (.ypt)."))?;
        Ok(Resp::json(200, groups_json(&groups)))
    }

    /// POST /versions/{vid}/entries/{eid}/ypt/edit — body: one change per line, "offset v0 v1 v2 v3" (decimal).
    /// Returns the modified resource entry bytes; the web UI queues them as a normal replace operation.
    pub(super) fn ypt_edit(&self, req: &mut Req, sid: &str, vid: &str, eid: &str) -> Result<Resp, ApiErr> {
        let body = req.body(MAX_YPT_CHANGES)?;
        let text = String::from_utf8(body).map_err(|_| bad("invalid_request", "The request body is not valid text.", "Reload the page and try again."))?;
        let mut changes: Vec<(usize, [f32; 4])> = Vec::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let p: Vec<&str> = line.split_whitespace().collect();
            let parsed = (p.len() == 5).then(|| {
                let off = p[0].parse::<usize>().ok()?;
                let v = [p[1].parse::<f32>().ok()?, p[2].parse::<f32>().ok()?, p[3].parse::<f32>().ok()?, p[4].parse::<f32>().ok()?];
                Some((off, v))
            }).flatten();
            changes.push(parsed.ok_or_else(|| bad("invalid_request", "A change line is not valid.", "Reload the page and try again."))?);
            if changes.len() > 50_000 {
                return Err(bad("invalid_request", "Too many changes in one request.", "Apply fewer changes at once."));
            }
        }
        if changes.is_empty() {
            return Err(bad("invalid_request", "There are no changes to apply.", "Change a value first."));
        }
        let mut r = self.ypt_load(self.base_blob(req, sid)?, sid, vid, eid)?;
        rpf_core::ypt::yaz(&mut r, &changes).map_err(|_| err(422, "invalid_change", "A change does not point at a keyframe value or is not a finite number.", "Reload the effects and try again."))?;
        let out = r.paketle();
        if out.len() as u64 > self.cfg.max_blob {
            return Err(err(413, "limit_exceeded", "The edited resource is larger than the limit for one added file.", ""));
        }
        Ok(Resp { status: 200, headers: vec![], body: Body::Bytes { data: Arc::new(out), ctype: "application/octet-stream" } })
    }
}

fn num(x: f32) -> String {
    if x.is_finite() { format!("{x}") } else { "0".into() }
}

fn arr(a: &[f32; 4]) -> String {
    format!("[{},{},{},{}]", num(a[0]), num(a[1]), num(a[2]), num(a[3]))
}

/// English JSON for the web UI from the original core's groups (kind is effect, emitter or particle).
fn groups_json(groups: &[rpf_core::ypt::Grup]) -> String {
    let items: Vec<String> = groups
        .iter()
        .map(|g| {
            let props: Vec<String> = g
                .ozellikler
                .iter()
                .map(|p| {
                    let fr: Vec<String> = p.kareler.iter().map(|f| format!("{{\"time\":{},\"value\":{},\"offset\":{}}}", arr(&f.zaman), arr(&f.deger), f.ofs)).collect();
                    format!("{{\"name\":{},\"frames\":[{}]}}", q(&p.ad), fr.join(","))
                })
                .collect();
            let kind = match g.tur {
                "efekt" => "effect",
                "yayici" => "emitter",
                _ => "particle",
            };
            let b = g.davranis.map_or("null".to_string(), q);
            format!("{{\"kind\":{},\"name\":{},\"behaviour\":{},\"properties\":[{}]}}", q(kind), q(&g.ad), b, props.join(","))
        })
        .collect();
    format!("{{\"groups\":[{}]}}", items.join(","))
}
