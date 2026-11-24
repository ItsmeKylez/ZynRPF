//! Particle (.ypt) routes: read the effects of a stored resource entry, and write changed keyframe values.
//! Uses rpf_core::rsc and rpf_core::ypt (derived from the original Zyn-RPF core, GPL-3.0; see LICENSE-GPL-NOTICE.md).
use super::*;

impl State {
    /// Stored bytes of a resource entry, decoded. Large-size resources and non-resources are rejected.
    fn ypt_load(&self, sid: &str, vid: &str, eid: &str) -> Result<rpf_core::rsc::Resource, ApiErr> {
        let (idx, path) = self.index_for(sid, vid)?;
        let id = parse_id(&idx, eid)?;
        let info = idx.info(id).ok_or_else(not_found)?;
        if info.kind != "resource" {
            return Err(err(422, "not_a_resource", "This entry is not a game resource, so it has no particle data.", "Choose a .ypt file."));
        }
        if info.large_resource {
            return Err(err(422, "large_resource_unsupported", "This resource uses the large-size marker, which is not supported yet.", ""));
        }
        let (off, len) = idx.stored_span(id).ok_or_else(not_found)?;
        if len > self.cfg.max_blob {
            return Err(err(422, "too_large_to_decode", "This resource is too large to open here.", ""));
        }
        let mut f = File::open(&path).map_err(|_| not_found())?;
        f.seek(SeekFrom::Start(off)).map_err(|_| not_found())?;
        let mut stored = vec![0u8; len as usize];
        f.read_exact(&mut stored).map_err(|_| err(422, "entry_unreadable", "The resource data could not be read from the archive.", ""))?;
        rpf_core::rsc::Resource::decode(&stored).map_err(|m| err(422, "resource_invalid", m, "Download the entry bytes instead."))
    }

    /// GET /versions/{vid}/entries/{eid}/ypt
    pub(super) fn ypt_read(&self, sid: &str, vid: &str, eid: &str) -> Result<Resp, ApiErr> {
        let r = self.ypt_load(sid, vid, eid)?;
        let groups = rpf_core::ypt::read(&r).map_err(|m| err(422, "not_a_ypt", m, "Choose a particle dictionary (.ypt)."))?;
        Ok(Resp::json(200, rpf_core::ypt::to_json(&groups)))
    }

    /// POST /versions/{vid}/entries/{eid}/ypt/edit — body: one change per line, "offset v0 v1 v2 v3" (decimal).
    /// Returns the modified resource entry bytes; the web UI queues them as a normal replace operation.
    pub(super) fn ypt_edit(&self, req: &mut Req, sid: &str, vid: &str, eid: &str) -> Result<Resp, ApiErr> {
        let body = req.body(MAX_JSON)?;
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
        let mut r = self.ypt_load(sid, vid, eid)?;
        rpf_core::ypt::write(&mut r, &changes).map_err(|m| err(422, "invalid_change", m, "Reload the effects and try again."))?;
        let out = r.pack();
        if out.len() as u64 > self.cfg.max_blob {
            return Err(err(413, "limit_exceeded", "The edited resource is larger than the limit for one added file.", ""));
        }
        Ok(Resp { status: 200, headers: vec![], body: Body::Bytes { data: Arc::new(out), ctype: "application/octet-stream" } })
    }
}
