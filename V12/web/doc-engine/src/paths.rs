use crate::error::EngineError;

const MAX_PATH_LEN: usize = 1024;
const MAX_SEGMENT_LEN: usize = 255;

/// Validates a client-supplied archive-internal virtual path (`/`-separated, no leading/trailing slash).
/// The empty string (archive root) is accepted only when `allow_root` is true.
/// Core's own path splitter also treats `\` as a separator, so `\` is rejected here to keep one canonical form.
pub fn validate_virtual_path(path: &str, allow_root: bool) -> Result<String, EngineError> {
    let bad = |m: &str| EngineError::new("INVALID_PATH", m.to_string());
    if path.is_empty() {
        return if allow_root { Ok(String::new()) } else { Err(bad("The path must not be empty.")) };
    }
    if path.len() > MAX_PATH_LEN {
        return Err(bad("The path is too long."));
    }
    for seg in path.split('/') {
        if seg.is_empty() {
            return Err(bad("The path contains an empty segment."));
        }
        if seg == "." || seg == ".." {
            return Err(bad("The path must not contain '.' or '..'."));
        }
        if seg.len() > MAX_SEGMENT_LEN {
            return Err(bad("A path segment is too long."));
        }
        if seg.chars().any(|c| c == '\\' || c.is_control()) {
            return Err(bad("The path contains a forbidden character."));
        }
    }
    Ok(path.to_string())
}

/// Validates a single entry name (rename target, new file or folder name): no separators, no control characters.
pub fn validate_entry_name(name: &str) -> Result<String, EngineError> {
    let bad = |m: &str| EngineError::new("INVALID_NAME", m.to_string());
    if name.is_empty() || name.trim().is_empty() {
        return Err(bad("The name must not be empty."));
    }
    if name == "." || name == ".." {
        return Err(bad("The name must not be '.' or '..'."));
    }
    if name.len() > MAX_SEGMENT_LEN {
        return Err(bad("The name is too long."));
    }
    if name.chars().any(|c| c == '/' || c == '\\' || c.is_control()) {
        return Err(bad("The name contains a forbidden character."));
    }
    Ok(name.to_string())
}

/// Removes duplicates (case-insensitive, like RPF paths) and paths that lie inside another selected path, so that
/// selecting a folder and one of its files does not make the second operation fail.
pub fn prune_nested(paths: Vec<String>) -> Vec<String> {
    let lower: Vec<String> = paths.iter().map(|p| p.to_lowercase()).collect();
    let mut out = Vec::new();
    for (i, p) in paths.iter().enumerate() {
        let dup = lower[..i].contains(&lower[i]);
        let inside = lower.iter().enumerate().any(|(j, q)| j != i && *q != lower[i] && lower[i].starts_with(&format!("{q}/")));
        if !dup && !inside {
            out.push(p.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_paths() {
        assert_eq!(validate_virtual_path("content/a.rpf/b.ytd", false).unwrap(), "content/a.rpf/b.ytd");
        assert_eq!(validate_virtual_path("", true).unwrap(), "");
    }

    #[test]
    fn rejects_traversal_and_oddities() {
        for p in ["../x", "a/../b", "./a", "a//b", "/a", "a/", "a\\b", "a\0b", "a\nb", ""] {
            assert!(validate_virtual_path(p, false).is_err(), "should reject {p:?}");
        }
        assert!(validate_virtual_path(&"a/".repeat(600), false).is_err());
        assert!(validate_virtual_path(&"x".repeat(300), false).is_err());
    }

    #[test]
    fn entry_names() {
        assert!(validate_entry_name("readme.txt").is_ok());
        for n in ["", "  ", ".", "..", "a/b", "a\\b", "a\0b", &"x".repeat(300)] {
            assert_eq!(validate_entry_name(n).unwrap_err().code, "INVALID_NAME", "{n:?}");
        }
    }

    #[test]
    fn prune_removes_duplicates_and_children() {
        let v = |x: &[&str]| x.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(prune_nested(v(&["a", "a/b", "A", "c", "a/b/c", "ab"])), v(&["a", "c", "ab"]));
        assert_eq!(prune_nested(v(&["x/y", "x/z"])), v(&["x/y", "x/z"]));
    }
}
