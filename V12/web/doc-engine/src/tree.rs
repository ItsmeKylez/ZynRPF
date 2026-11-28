//! Archive tree DTO. Adapted from `agac_kur` / `dizin_bilgisi` / `tahmini_boyut` (src-tauri/src/lib.rs).
use serde::Serialize;
use tlk_rpf_core::rpf::{Dizin, Dugum};

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Directory,
    Archive,
    File,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub name: String,
    pub path: String,
    pub kind: NodeKind,
    pub size: u64,
    pub stored_size: u64,
    pub modified: bool,
    pub resource: bool,
    pub children: Vec<Node>,
}

pub fn build_tree(d: &Dizin, prefix: &str) -> Vec<Node> {
    let mut v: Vec<Node> = d
        .cocuklar
        .iter()
        .map(|c| {
            let path = if prefix.is_empty() { c.ad().to_string() } else { format!("{prefix}/{}", c.ad()) };
            match c {
                Dugum::Dizin(sub) => container(c.ad(), path.clone(), NodeKind::Directory, build_tree(sub, &path)),
                Dugum::Arsiv(a) => container(c.ad(), path.clone(), NodeKind::Archive, build_tree(&a.arsiv.kok, &path)),
                Dugum::Dosya(f) => Node {
                    name: f.ad.clone(),
                    path,
                    kind: NodeKind::File,
                    size: f.boyut() as u64,
                    stored_size: f.kayitli_boyut() as u64,
                    modified: f.degisti(),
                    resource: f.kaynak_mi(),
                    children: Vec::new(),
                },
            }
        })
        .collect();
    // Same order as the desktop UI: containers first, then by name.
    v.sort_by(|a, b| (a.kind == NodeKind::File).cmp(&(b.kind == NodeKind::File)).then_with(|| a.name.cmp(&b.name)));
    v
}

fn container(name: &str, path: String, kind: NodeKind, children: Vec<Node>) -> Node {
    Node {
        name: name.to_string(),
        path,
        kind,
        size: children.iter().map(|c| c.size).sum(),
        stored_size: children.iter().map(|c| c.stored_size).sum(),
        modified: children.iter().any(|c| c.modified),
        resource: false,
        children,
    }
}

/// Estimated on-disk size after a rewrite (512-byte alignment), as in `tahmini_boyut`.
pub fn estimated_size(d: &Dizin) -> u64 {
    let align = |n: u64| n.div_ceil(512) * 512;
    let mut total = 0;
    let mut count = 1u64;
    for c in &d.cocuklar {
        count += 1;
        total += match c {
            Dugum::Dizin(sub) => estimated_size(sub),
            Dugum::Arsiv(a) => align(estimated_size(&a.arsiv.kok)),
            Dugum::Dosya(f) => align(f.kayitli_boyut() as u64),
        };
    }
    total + align(count * 40)
}
