//! Standalone validation (D-021): stable check codes (D-031), severities, and a report that can be run on any
//! archive. Any `Severity::Error` finding blocks publishing a saved version (D-007).
use crate::error::{Code, Severity};
use crate::format::*;
use crate::index::{Index, Limits};
use crate::rebuild::payload_spans;
use std::collections::HashSet;
use std::io::{Read, Seek};

const MAX_PER_CODE: usize = 50;

#[derive(Debug, Clone)]
pub struct Finding {
    pub code: Code,
    pub severity: Severity,
    pub detail: String,
}

fn add(out: &mut Vec<Finding>, counts: &mut Vec<(Code, usize)>, code: Code, detail: String) {
    let c = match counts.iter_mut().find(|x| x.0 == code) {
        Some(x) => x,
        None => {
            counts.push((code, 0));
            counts.last_mut().unwrap()
        }
    };
    c.1 += 1;
    if c.1 <= MAX_PER_CODE {
        out.push(Finding { code, severity: code.severity(), detail });
    }
}

/// Structural checks on a parsed index. `file_len` and `table_end` come from the index itself.
pub fn check_index(idx: &Index) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut counts = Vec::new();
    // Duplicate names inside one folder (case-insensitive).
    for id in 0..idx.len() as u32 {
        if let Some((first, count)) = idx.children(id) {
            if count < 2 || first as u64 + count as u64 > idx.len() as u64 {
                continue;
            }
            let mut seen: HashSet<String> = HashSet::with_capacity(count as usize);
            for c in first..first + count {
                if !seen.insert(idx.name(c).to_lowercase()) {
                    add(&mut out, &mut counts, Code::DuplicateName, format!("{} in {}", idx.name(c), idx.path(id)));
                }
            }
        }
    }
    if idx.orphans() > 0 {
        add(&mut out, &mut counts, Code::OrphanEntries, format!("{} entries", idx.orphans()));
    }
    // Payload bounds (including large resources, which parse() cannot size) and overlaps.
    let mut spans = payload_spans(idx);
    spans.retain(|s| s.1 > 0 || matches!(idx.entry(s.2), Some(Entry { kind: Kind::Res { .. }, .. })));
    for &(off, len, id) in &spans {
        let large = matches!(idx.entry(id), Some(Entry { kind: Kind::Res { size, .. }, .. }) if *size == LARGE_RESOURCE);
        if large {
            if off < idx.table_end() {
                add(&mut out, &mut counts, Code::DataInTable, idx.path(id));
            } else if off > idx.file_len() {
                add(&mut out, &mut counts, Code::DataOob, idx.path(id));
            }
        }
        let _ = len;
    }
    spans.retain(|s| s.1 > 0);
    spans.sort_unstable();
    let mut end = 0u64;
    let mut prev = 0u32;
    for &(off, len, id) in &spans {
        if off < end {
            add(&mut out, &mut counts, Code::DataOverlap, format!("{} and {}", idx.path(prev), idx.path(id)));
        }
        if off + len > end {
            end = off + len;
            prev = id;
        }
    }
    for (code, n) in counts {
        if n > MAX_PER_CODE {
            out.push(Finding { code, severity: code.severity(), detail: format!("{} more of the same kind not listed", n - MAX_PER_CODE) });
        }
    }
    out
}

/// Parses and checks an archive. A parse failure is reported as a single Error finding.
pub fn validate<R: Read + Seek>(r: &mut R, lim: &Limits) -> (Option<Index>, Vec<Finding>) {
    match Index::parse(r, lim) {
        Err(code) => (None, vec![Finding { code, severity: code.severity(), detail: String::new() }]),
        Ok(idx) => {
            let f = check_index(&idx);
            (Some(idx), f)
        }
    }
}

pub fn has_errors(f: &[Finding]) -> bool {
    f.iter().any(|x| x.severity == Severity::Error)
}
