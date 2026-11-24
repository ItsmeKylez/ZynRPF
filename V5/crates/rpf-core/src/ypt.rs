//! Particle dictionary (.ypt, ptxFxList): keyframe values of effect rules, emitter rules and particle rules.
//! Only existing values are changed in place; structure and size never change.
//!
//! DERIVED FROM the original Zyn-RPF / tlk-rpf core (`core/src/ypt.rs`, GPL-3.0, Copyright (c) 2026 Talkdedsec),
//! translated to English and without serde. The original layout comes from CodeWalker's Particle.cs (MIT).
//! See LICENSE-GPL-NOTICE.md.
use crate::rsc::Resource;

/// ptxKeyframeProp name hashes.
const PROPERTY_NAMES: &[(u32, &str)] = &[
    (0x68f00338, "ptxAttractorDomain:m_positionKFP"), (0x8ace32c2, "ptxAttractorDomain:m_rotationKFP"),
    (0x851d3d14, "ptxAttractorDomain:m_sizeInnerKFP"), (0xc248b5c9, "ptxAttractorDomain:m_sizeOuterKFP"),
    (0x45e377e9, "ptxCreationDomain:m_positionKFP"), (0x5e692d43, "ptxCreationDomain:m_rotationKFP"),
    (0x841ab3da, "ptxCreationDomain:m_sizeInnerKFP"), (0x1104051e, "ptxCreationDomain:m_sizeOuterKFP"),
    (0xd7c1e22b, "ptxEffectRule:m_colourTintMaxKFP"), (0xe5480b3b, "ptxEffectRule:m_colourTintMinKFP"),
    (0xff864d6c, "ptxEffectRule:m_dataCapsuleKFP"), (0x34d6ded7, "ptxEffectRule:m_dataSphereKFP"),
    (0xce8e57a7, "ptxEffectRule:m_zoomScalarKFP"), (0xa83b53f0, "ptxEmitterRule:m_accnScalarKFP"),
    (0xdd18b4f2, "ptxEmitterRule:m_dampeningScalarKFP"), (0xd2df1fa0, "ptxEmitterRule:m_inheritVelocityKFP"),
    (0xe511bc23, "ptxEmitterRule:m_matrixWeightScalarKFP"), (0x9fc4652b, "ptxEmitterRule:m_particleLifeKFP"),
    (0x60855078, "ptxEmitterRule:m_playbackRateScalarKFP"), (0x4af0ffa1, "ptxEmitterRule:m_sizeScalarKFP"),
    (0xe00e5025, "ptxEmitterRule:m_spawnRateOverDistKFP"), (0x61c50318, "ptxEmitterRule:m_spawnRateOverTimeKFP"),
    (0xc9fe6abb, "ptxEmitterRule:m_speedScalarKFP"), (0x41d49131, "ptxTargetDomain:m_positionKFP"),
    (0x64c6c696, "ptxTargetDomain:m_rotationKFP"), (0xe7d61ff7, "ptxTargetDomain:m_sizeInnerKFP"),
    (0x13c0cac4, "ptxTargetDomain:m_sizeOuterKFP"), (0x72668c6f, "ptxd_Trail:m_texInfoKFP"),
    (0xd1be590a, "ptxu_Acceleration:m_strengthKFP"), (0x412a554c, "ptxu_Acceleration:m_xyzMaxKFP"),
    (0x30e327d4, "ptxu_Acceleration:m_xyzMinKFP"), (0x2946e76f, "ptxu_AnimateTexture:m_animRateKFP"),
    (0x686f965f, "ptxu_Collision:m_bounceDirVarKFP"), (0x61532d47, "ptxu_Collision:m_bouncinessKFP"),
    (0x8306b23a, "ptxu_Colour:m_emissiveIntensityKFP"), (0x60500691, "ptxu_Colour:m_rgbaMaxKFP"),
    (0x7fae9df8, "ptxu_Colour:m_rgbaMinKFP"), (0x64c7fc25, "ptxu_Dampening:m_xyzMaxKFP"),
    (0xd0ef73c5, "ptxu_Dampening:m_xyzMinKFP"), (0xa7228870, "ptxu_Decal:m_alphaKFP"),
    (0xfb8eb4e6, "ptxu_Decal:m_dimensionsKFP"), (0x3ee8e85e, "ptxu_FogVolume:m_densityRangeKFP"),
    (0x23f55175, "ptxu_FogVolume:m_rgbTintMaxKFP"), (0x3c599207, "ptxu_FogVolume:m_rgbTintMinKFP"),
    (0x570dc9cd, "ptxu_FogVolume:m_rotationMaxKFP"), (0x9ef3ceec, "ptxu_FogVolume:m_rotationMinKFP"),
    (0x5473d2fe, "ptxu_FogVolume:m_scaleMaxKFP"), (0xdafe6982, "ptxu_FogVolume:m_scaleMinKFP"),
    (0xb9410926, "ptxu_Light:m_coronaFlareKFP"), (0xe2c464a6, "ptxu_Light:m_coronaIntensityKFP"),
    (0xf8561886, "ptxu_Light:m_coronaRgbMaxKFP"), (0xe364d5b2, "ptxu_Light:m_coronaRgbMinKFP"),
    (0xc35aaf9b, "ptxu_Light:m_coronaSizeKFP"), (0xef500a62, "ptxu_Light:m_intensityKFP"),
    (0x75990186, "ptxu_Light:m_rangeKFP"), (0x12bbe65e, "ptxu_Light:m_rgbMaxKFP"),
    (0xda8c99a6, "ptxu_Light:m_rgbMinKFP"), (0xe7af1a2c, "ptxu_MatrixWeight:m_mtxWeightKFP"),
    (0xff31aaf3, "ptxu_Noise:m_posNoiseMaxKFP"), (0x2d0d70b5, "ptxu_Noise:m_posNoiseMinKFP"),
    (0x513812a5, "ptxu_Noise:m_velNoiseMaxKFP"), (0xf256e579, "ptxu_Noise:m_velNoiseMinKFP"),
    (0x687b4382, "ptxu_Rotation:m_angleMaxKFP"), (0xf0274f77, "ptxu_Rotation:m_angleMinKFP"),
    (0x351ed852, "ptxu_Rotation:m_initialAngleMaxKFP"), (0x1c256ba4, "ptxu_Rotation:m_initialAngleMinKFP"),
    (0xa67a1155, "ptxu_Size:m_tblrScalarKFP"), (0xd5c0fce5, "ptxu_Size:m_tblrVelScalarKFP"),
    (0x3dc78098, "ptxu_Size:m_whdMaxKFP"), (0x1f641348, "ptxu_Size:m_whdMinKFP"),
    (0x0aadcbef, "ptxu_Wind:m_influenceKFP"), (0xea6afaba, "ptxu_ZCull:m_fadeDistKFP"),
    (0xce9adbfd, "ptxu_ZCull:m_heightKFP"),
];

const BEHAVIOUR_TYPES: &[(u32, &str)] = &[
    (0xf5b33baa, "Age"), (0xd63d9f1b, "Acceleration"), (0x6c0719bc, "Velocity"), (0x1ee64552, "Rotation"),
    (0x38b60240, "Size"), (0x052b1293, "Dampening"), (0x64e5d702, "MatrixWeight"), (0x928a1c45, "Collision"),
    (0xeca84c1e, "AnimateTexture"), (0x164aea72, "Colour"), (0x68fa73f5, "Sprite"), (0x38b63978, "Wind"),
    (0x0544c710, "Light"), (0x6232e25a, "Model"), (0x8f3b6036, "Decal"), (0xa35c721f, "ZCull"),
    (0xb77fed19, "Noise"), (0x25ac9437, "Attractor"), (0xc57377f8, "Trail"), (0xa05da63e, "FogVolume"),
    (0xd4594bef, "River"), (0xa2d6dc3f, "DecalPool"), (0xdf229542, "Liquid"),
];

#[derive(Clone)]
pub struct Frame {
    /// x: 0..1 time or lifetime
    pub time: [f32; 4],
    pub value: [f32; 4],
    /// flat offset of the value in the decoded resource (the identity used when writing)
    pub offset: usize,
}

#[derive(Clone)]
pub struct Property {
    pub name: String,
    pub frames: Vec<Frame>,
}

#[derive(Clone)]
pub struct Group {
    /// "effect", "emitter" or "particle"
    pub kind: &'static str,
    pub name: String,
    /// behaviour type of a particle rule (Colour, Size, Light, ...)
    pub behaviour: Option<&'static str>,
    pub properties: Vec<Property>,
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> { Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?)) }
fn u32_at(b: &[u8], o: usize) -> Option<u32> { Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?)) }
fn u64_at(b: &[u8], o: usize) -> Option<u64> { Some(u64::from_le_bytes(b.get(o..o + 8)?.try_into().ok()?)) }

fn cstring(r: &Resource, p: u64) -> String {
    let Some(o) = r.offset(p) else { return String::new() };
    let end = r.data[o..].iter().take(128).position(|&b| b == 0).unwrap_or(0);
    String::from_utf8_lossy(&r.data[o..o + end]).into_owned()
}

/// Flat offsets pointed to by the pointer list at `o` (u64 pointer, u16 count).
fn pointers(r: &Resource, o: usize, limit: usize) -> Vec<usize> {
    let v = &r.data;
    let (Some(l), Some(n)) = (u64_at(v, o).and_then(|p| r.offset(p)), u16_at(v, o + 8)) else { return Vec::new() };
    (0..(n as usize).min(limit)).filter_map(|i| u64_at(v, l + i * 8).and_then(|p| r.offset(p))).collect()
}

fn read_property(r: &Resource, o: usize) -> Option<Property> {
    let v = &r.data;
    let hash = u32_at(v, o + 0x68)?;
    let name = PROPERTY_NAMES.iter().find(|x| x.0 == hash).map_or_else(|| format!("hash_{hash:08x}"), |x| x.1.to_string());
    let l = r.offset(u64_at(v, o + 0x70)?);
    let n = u16_at(v, o + 0x78)? as usize;
    if n > 256 {
        return None;
    }
    let f = |x: usize| f32::from_le_bytes(v[x..x + 4].try_into().unwrap());
    let frames = match l {
        Some(l) if l + n * 0x20 <= v.len() => (0..n)
            .map(|i| {
                let b = l + i * 0x20;
                Frame { time: std::array::from_fn(|j| f(b + j * 4)), value: std::array::from_fn(|j| f(b + 16 + j * 4)), offset: b + 16 }
            })
            .collect(),
        _ => Vec::new(),
    };
    Some(Property { name, frames })
}

/// Values of a pgDictionary (0x30 values, 0x38 count) at the pointer stored at `root`.
fn dictionary(r: &Resource, root: usize) -> Vec<usize> {
    u64_at(&r.data, root).and_then(|p| r.offset(p)).map(|s| pointers(r, s + 0x30, 8192)).unwrap_or_default()
}

pub fn read(r: &Resource) -> Result<Vec<Group>, String> {
    let name = cstring(r, u64_at(&r.data, 0x10).unwrap_or(0));
    if name.is_empty() {
        return Err("This is not a particle dictionary (.ypt).".into());
    }
    let mut groups = Vec::new();
    let props = |list: Vec<usize>| list.into_iter().filter_map(|o| read_property(r, o)).filter(|x| !x.frames.is_empty()).collect::<Vec<_>>();
    // An effect rule embeds 5 keyframe properties, an emitter rule 10.
    let embedded = |start: usize, n: usize| (0..n).map(|i| start + i * 0x90).collect::<Vec<_>>();
    for e in dictionary(r, 0x48) {
        let properties = props(embedded(e + 0xC0, 5));
        if !properties.is_empty() {
            groups.push(Group { kind: "effect", name: cstring(r, u64_at(&r.data, e + 0x20).unwrap_or(0)), behaviour: None, properties });
        }
    }
    for e in dictionary(r, 0x50) {
        let properties = props(embedded(e + 0x78, 10));
        if !properties.is_empty() {
            groups.push(Group { kind: "emitter", name: cstring(r, u64_at(&r.data, e + 0x20).unwrap_or(0)), behaviour: None, properties });
        }
    }
    for p in dictionary(r, 0x38) {
        let rule = cstring(r, u64_at(&r.data, p + 0x120).unwrap_or(0));
        for b in pointers(r, p + 0x128, 64) {
            let t = u32_at(&r.data, b + 0x08).unwrap_or(0);
            let behaviour = BEHAVIOUR_TYPES.iter().find(|x| x.0 == t).map(|x| x.1);
            let properties = props(pointers(r, b + 0x10, 64));
            if !properties.is_empty() {
                groups.push(Group { kind: "particle", name: rule.clone(), behaviour, properties });
            }
        }
    }
    Ok(groups)
}

/// Writes keyframe values; only offsets that `read` returns are accepted.
pub fn write(r: &mut Resource, changes: &[(usize, [f32; 4])]) -> Result<usize, String> {
    let valid: std::collections::HashSet<usize> = read(r)?.iter().flat_map(|g| &g.properties).flat_map(|p| &p.frames).map(|x| x.offset).collect();
    for (off, v) in changes {
        if !valid.contains(off) {
            return Err(format!("{off:#x} is not a keyframe value."));
        }
        if v.iter().any(|x| !x.is_finite()) {
            return Err("A value is not a finite number.".into());
        }
    }
    for (off, v) in changes {
        for (j, x) in v.iter().enumerate() {
            r.data[off + j * 4..off + j * 4 + 4].copy_from_slice(&x.to_le_bytes());
        }
    }
    Ok(changes.len())
}

fn num(x: f32) -> String {
    if x.is_finite() { format!("{x}") } else { "0".into() }
}

fn arr(a: &[f32; 4]) -> String {
    format!("[{},{},{},{}]", num(a[0]), num(a[1]), num(a[2]), num(a[3]))
}

/// JSON for the web UI (hand-written; this crate has no dependencies).
pub fn to_json(groups: &[Group]) -> String {
    fn js(s: &str) -> String {
        let mut o = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""),
                '\\' => o.push_str("\\\\"),
                c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
                c => o.push(c),
            }
        }
        o.push('"');
        o
    }
    let items: Vec<String> = groups
        .iter()
        .map(|g| {
            let props: Vec<String> = g
                .properties
                .iter()
                .map(|p| {
                    let fr: Vec<String> = p.frames.iter().map(|f| format!("{{\"time\":{},\"value\":{},\"offset\":{}}}", arr(&f.time), arr(&f.value), f.offset)).collect();
                    format!("{{\"name\":{},\"frames\":[{}]}}", js(&p.name), fr.join(","))
                })
                .collect();
            let b = g.behaviour.map_or("null".to_string(), js);
            format!("{{\"kind\":{},\"name\":{},\"behaviour\":{},\"properties\":[{}]}}", js(g.kind), js(&g.name), b, props.join(","))
        })
        .collect();
    format!("{{\"groups\":[{}]}}", items.join(","))
}
