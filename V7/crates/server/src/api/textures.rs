//! Texture routes for .ytd dictionaries and the dictionaries embedded in .ydr/.ydd/.yft/.ypt resources:
//! list, PNG preview of a mip level, DDS export, in-place replace with an uploaded image, and in-place recolour.
//! Uses rpf_core::ytd and rpf_core::doku (copied verbatim from the original Zyn-RPF core, GPL-3.0; see LICENSE-GPL-NOTICE.md).
//! The original core has Turkish names and messages; this file maps them to English JSON and English errors.
use super::*;
use rpf_core::doku::{RenkAyari, Sigdirma};
use rpf_core::ytd::Ytd;

const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_IMAGE_SIDE: u32 = 8192;
const MAX_IMAGE_PIXELS: u64 = 32 * 1024 * 1024;
const MAX_PREVIEW_PIXELS: usize = 16 * 1024 * 1024;

fn bad_tex(e: rpf_core::Hata) -> ApiErr {
    // Never pass the original (Turkish) text to the UI.
    let _ = e;
    err(422, "texture_failed", "The texture could not be processed. It may use an unsupported format or damaged data.", "Choose another texture or download the entry bytes.")
}

impl State {
    /// Decoded texture dictionary of a resource entry (the .ytd itself, or the embedded dictionaries of a model or particle file).
    fn tex_load(&self, base: Option<Vec<u8>>, sid: &str, vid: &str, eid: &str) -> Result<(Ytd, String), ApiErr> {
        let (idx, path) = self.index_for(sid, vid)?;
        let id = parse_id(&idx, eid)?;
        let info = idx.info(id).ok_or_else(not_found)?;
        if info.kind != "resource" {
            return Err(err(422, "not_a_resource", "This entry is not a game resource, so it has no textures.", "Choose a .ytd file."));
        }
        if info.large_resource {
            return Err(err(422, "large_resource_unsupported", "This resource uses the large-size marker, which is not supported yet.", ""));
        }
        let ext = info.name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        if !matches!(ext.as_str(), "ytd" | "ydr" | "ydd" | "yft" | "ypt") {
            return Err(err(422, "no_textures", "This resource type has no texture dictionary.", "Choose a .ytd, .ydr, .ydd, .yft or .ypt file."));
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
        let y = if ext == "ytd" { Ytd::coz(&stored) } else { Ytd::modelden(&stored, &ext) };
        let y = y.map_err(|_| err(422, "resource_invalid", "The resource could not be decoded. It may be damaged, encrypted or too large.", "Download the entry bytes instead."))?;
        Ok((y, ext))
    }

    fn tex_index(y: &Ytd, s: &str) -> Result<usize, ApiErr> {
        let i: usize = s.parse().map_err(|_| not_found())?;
        if i < y.dokular.len() { Ok(i) } else { Err(not_found()) }
    }

    /// GET /versions/{vid}/entries/{eid}/textures
    pub(super) fn tex_list(&self, req: &Req, sid: &str, vid: &str, eid: &str) -> Result<Resp, ApiErr> {
        let (y, ext) = self.tex_load(self.base_blob(req, sid)?, sid, vid, eid)?;
        let items: Vec<String> = y
            .dokular
            .iter()
            .enumerate()
            .map(|(i, d)| format!("{{\"index\":{},\"name\":{},\"width\":{},\"height\":{},\"format\":{},\"levels\":{},\"supported\":{}}}", i, q(&d.ad), d.genislik, d.yukseklik, q(&d.bicim.ad()), d.seviye, d.bicim.destekli()))
            .collect();
        Ok(Resp::json(200, format!("{{\"embedded\":{},\"textures\":[{}]}}", ext != "ytd", items.join(","))))
    }

    /// GET /versions/{vid}/entries/{eid}/textures/{i}/png?mip=N
    pub(super) fn tex_png(&self, req: &Req, sid: &str, vid: &str, eid: &str, i: &str) -> Result<Resp, ApiErr> {
        let (y, _) = self.tex_load(self.base_blob(req, sid)?, sid, vid, eid)?;
        let i = Self::tex_index(&y, i)?;
        let mip: usize = req.query.get("mip").and_then(|s| s.parse().ok()).unwrap_or(0);
        let d = &y.dokular[i];
        let (w, h) = ((d.genislik >> mip.min(d.seviye - 1)).max(1), (d.yukseklik >> mip.min(d.seviye - 1)).max(1));
        if w * h > MAX_PREVIEW_PIXELS {
            return Err(err(422, "too_large_to_preview", "This texture level is too large to preview.", "Preview a smaller mip level."));
        }
        let (rgba, w, h) = y.rgba(i, mip).map_err(bad_tex)?;
        let png = rpf_core::doku::png(&rgba, w, h).map_err(bad_tex)?;
        Ok(Resp { status: 200, headers: vec![], body: Body::Bytes { data: Arc::new(png), ctype: "image/png" } })
    }

    /// GET /versions/{vid}/entries/{eid}/textures/{i}/dds
    pub(super) fn tex_dds(&self, req: &Req, sid: &str, vid: &str, eid: &str, i: &str) -> Result<Resp, ApiErr> {
        let (y, _) = self.tex_load(self.base_blob(req, sid)?, sid, vid, eid)?;
        let i = Self::tex_index(&y, i)?;
        let dds = y.dds(i).map_err(bad_tex)?;
        Ok(Resp { status: 200, headers: vec![], body: Body::Bytes { data: Arc::new(dds), ctype: "application/octet-stream" } })
    }

    fn colour_from(req: &Req) -> Result<RenkAyari, ApiErr> {
        let f = |k: &str, d: f32, lo: f32, hi: f32| -> Result<f32, ApiErr> {
            match req.query.get(k) {
                None => Ok(d),
                Some(s) => s.parse::<f32>().ok().filter(|v| v.is_finite() && *v >= lo && *v <= hi).ok_or_else(|| bad("invalid_value", "A color setting is out of range.", "Correct the value and try again.")),
            }
        };
        let mut c = RenkAyari::notr();
        c.ton = f("hue", 0.0, -180.0, 180.0)?;
        c.doygunluk = f("saturation", 1.0, 0.0, 4.0)?;
        c.parlaklik = f("brightness", 0.0, -1.0, 1.0)?;
        c.kontrast = f("contrast", 1.0, 0.0, 4.0)?;
        c.boya_gucu = f("tint_strength", 0.0, 0.0, 1.0)?;
        if let Some(t) = req.query.get("tint") {
            let ok = t.len() == 6 && t.bytes().all(|b| b.is_ascii_hexdigit());
            if !ok {
                return Err(bad("invalid_value", "The tint color must be six hexadecimal digits.", "Correct the value and try again."));
            }
            let p = |a: usize| u8::from_str_radix(&t[a..a + 2], 16).unwrap_or(0);
            c.boya = Some([p(0), p(2), p(4)]);
        }
        Ok(c)
    }

    fn tex_output(&self, y: &Ytd) -> Result<Resp, ApiErr> {
        let out = y.dosya();
        if out.len() as u64 > self.cfg.max_blob {
            return Err(err(413, "limit_exceeded", "The edited resource is larger than the limit for one added file.", ""));
        }
        Ok(Resp { status: 200, headers: vec![], body: Body::Bytes { data: Arc::new(out), ctype: "application/octet-stream" } })
    }

    /// POST /versions/{vid}/entries/{eid}/textures/{i}/replace?fit=stretch|fit|fill&hue=..&tint=rrggbb.. — body: PNG, JPEG, WebP, BMP, TGA or DDS image.
    /// The texture keeps its size, format and mip levels. Returns the edited resource bytes; the web UI queues them as a Replace operation.
    pub(super) fn tex_replace(&self, req: &mut Req, sid: &str, vid: &str, eid: &str, i: &str) -> Result<Resp, ApiErr> {
        let colour = Self::colour_from(req)?;
        let fit = match req.query.get("fit").map(|s| s.as_str()).unwrap_or("stretch") {
            "stretch" => Sigdirma::Esnet,
            "fit" => Sigdirma::Sigdir,
            "fill" => Sigdirma::Doldur,
            _ => return Err(bad("invalid_value", "The fit mode must be stretch, fit or fill.", "Correct the value and try again.")),
        };
        let body = req.body(MAX_IMAGE_BYTES)?;
        if body.is_empty() {
            return Err(bad("invalid_request", "No image was sent.", "Choose an image file."));
        }
        // Check the declared size before decoding so a small file cannot expand into a huge image.
        let dims = if body.starts_with(b"DDS ") && body.len() >= 20 {
            Some((u32::from_le_bytes([body[16], body[17], body[18], body[19]]), u32::from_le_bytes([body[12], body[13], body[14], body[15]])))
        } else {
            image::ImageReader::new(std::io::Cursor::new(&body)).with_guessed_format().ok().and_then(|r| r.into_dimensions().ok())
        };
        let (iw, ih) = dims.ok_or_else(|| err(422, "image_unreadable", "The image could not be read. Use PNG, JPEG, WebP, BMP, TGA or DDS.", "Choose another image."))?;
        if iw == 0 || ih == 0 || iw > MAX_IMAGE_SIDE || ih > MAX_IMAGE_SIDE || (iw as u64) * (ih as u64) > MAX_IMAGE_PIXELS {
            return Err(err(422, "image_too_large", "The image is too large (at most 8192 pixels on a side).", "Resize the image and try again."));
        }
        let (mut y, _) = self.tex_load(self.base_blob(req, sid)?, sid, vid, eid)?;
        let i = Self::tex_index(&y, i)?;
        let (rgba, w, h) = rpf_core::doku::gorsel_ac(&body).map_err(|_| err(422, "image_unreadable", "The image could not be read. Use PNG, JPEG, WebP, BMP, TGA or DDS.", "Choose another image."))?;
        y.degistir(i, rgba, w, h, fit, &colour).map_err(bad_tex)?;
        self.tex_output(&y)
    }

    /// POST /versions/{vid}/entries/{eid}/textures/recolor?indices=1,2&hue=..&saturation=..&brightness=..&contrast=..&tint=rrggbb&tint_strength=..
    pub(super) fn tex_recolor(&self, req: &Req, sid: &str, vid: &str, eid: &str) -> Result<Resp, ApiErr> {
        let colour = Self::colour_from(req)?;
        if colour.bos_mu() {
            return Err(bad("invalid_request", "There is no color change to apply.", "Change at least one color setting."));
        }
        let (mut y, _) = self.tex_load(self.base_blob(req, sid)?, sid, vid, eid)?;
        let list = req.query.get("indices").ok_or_else(|| bad("invalid_request", "No textures were chosen.", "Choose at least one texture."))?;
        let mut ids: Vec<usize> = Vec::new();
        for p in list.split(',').filter(|s| !s.is_empty()) {
            ids.push(Self::tex_index(&y, p)?);
            if ids.len() > 4096 {
                return Err(bad("invalid_request", "Too many textures in one request.", "Choose fewer textures."));
            }
        }
        if ids.is_empty() {
            return Err(bad("invalid_request", "No textures were chosen.", "Choose at least one texture."));
        }
        for i in ids {
            y.renklendir(i, &colour).map_err(bad_tex)?;
        }
        self.tex_output(&y)
    }
}
