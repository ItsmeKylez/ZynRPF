use std::path::PathBuf;
use std::time::Duration;

/// Server limits. Defaults are the D4 proposals in `docs/M1_ARCHITECTURE.md` (UNVERIFIED against a real Railway plan).
#[derive(Debug, Clone)]
pub struct Config {
    pub workspace_root: PathBuf,
    pub max_upload_bytes: u64,
    pub max_workspace_bytes: u64,
    pub max_workspaces: usize,
    pub max_files_per_workspace: usize,
    pub idle_ttl: Duration,
    pub max_age: Duration,
    /// Adds `Secure` to the session cookie. Browsers still accept it on http://localhost.
    pub cookie_secure: bool,
    /// Directory with the built web frontend (`dist-web`). When set it is served at `/` with an SPA fallback and the
    /// developer harness moves to `/dev`. When unset, `/` is the developer harness.
    pub static_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace_root: std::env::temp_dir().join("zyn-workspaces"),
            max_upload_bytes: 512 * 1024 * 1024,
            max_workspace_bytes: 2 * 1024 * 1024 * 1024,
            max_workspaces: 50,
            max_files_per_workspace: 8,
            idle_ttl: Duration::from_secs(2 * 3600),
            max_age: Duration::from_secs(12 * 3600),
            cookie_secure: true,
            static_dir: None,
        }
    }
}

impl Config {
    /// Reads env vars through `get` (injectable for tests). A set-but-invalid value is an error, never silently ignored.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let mut c = Self::default();
        fn num<T: std::str::FromStr>(get: &impl Fn(&str) -> Option<String>, key: &str, out: &mut T) -> Result<(), String> {
            if let Some(v) = get(key) {
                *out = v.trim().parse().map_err(|_| format!("{key} must be a non-negative integer, got {v:?}"))?;
            }
            Ok(())
        }
        if let Some(v) = get("WORKSPACE_ROOT") {
            if v.trim().is_empty() {
                return Err("WORKSPACE_ROOT must not be empty".into());
            }
            c.workspace_root = PathBuf::from(v);
        }
        num(&get, "MAX_UPLOAD_BYTES", &mut c.max_upload_bytes)?;
        num(&get, "MAX_WORKSPACE_BYTES", &mut c.max_workspace_bytes)?;
        num(&get, "MAX_WORKSPACES", &mut c.max_workspaces)?;
        num(&get, "MAX_FILES_PER_WORKSPACE", &mut c.max_files_per_workspace)?;
        let (mut idle, mut age) = (c.idle_ttl.as_secs(), c.max_age.as_secs());
        num(&get, "WORKSPACE_IDLE_TTL", &mut idle)?;
        num(&get, "WORKSPACE_MAX_AGE", &mut age)?;
        c.idle_ttl = Duration::from_secs(idle);
        c.max_age = Duration::from_secs(age);
        if let Some(v) = get("STATIC_DIR") {
            let p = PathBuf::from(v.trim());
            if v.trim().is_empty() || !p.join("index.html").is_file() {
                return Err(format!("STATIC_DIR must contain an index.html, got {v:?}"));
            }
            c.static_dir = Some(p);
        }
        if let Some(v) = get("COOKIE_SECURE") {
            c.cookie_secure = match v.trim().to_ascii_lowercase().as_str() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => return Err(format!("COOKIE_SECURE must be true or false, got {v:?}")),
            };
        }
        if c.max_upload_bytes == 0 || c.max_workspaces == 0 || c.max_files_per_workspace == 0 {
            return Err("limits must be greater than zero".into());
        }
        Ok(c)
    }

    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let m: HashMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |k| m.get(k).cloned()
    }

    #[test]
    fn defaults_apply_when_unset() {
        let c = Config::from_lookup(lookup(&[])).unwrap();
        assert_eq!(c.max_upload_bytes, 512 * 1024 * 1024);
        assert_eq!(c.idle_ttl, Duration::from_secs(7200));
    }

    #[test]
    fn overrides_and_invalid_values() {
        let c = Config::from_lookup(lookup(&[("MAX_UPLOAD_BYTES", "1000"), ("WORKSPACE_ROOT", "/data/ws")])).unwrap();
        assert_eq!(c.max_upload_bytes, 1000);
        assert_eq!(c.workspace_root, PathBuf::from("/data/ws"));
        assert!(Config::from_lookup(lookup(&[("MAX_UPLOAD_BYTES", "abc")])).is_err());
        assert!(Config::from_lookup(lookup(&[("MAX_WORKSPACES", "0")])).is_err());
        assert!(Config::from_lookup(lookup(&[("WORKSPACE_ROOT", " ")])).is_err());
        assert!(Config::from_lookup(lookup(&[])).unwrap().cookie_secure);
        assert!(!Config::from_lookup(lookup(&[("COOKIE_SECURE", "false")])).unwrap().cookie_secure);
        assert!(Config::from_lookup(lookup(&[("COOKIE_SECURE", "maybe")])).is_err());
        assert!(Config::from_lookup(lookup(&[("STATIC_DIR", "/definitely/not/here")])).is_err());
        assert!(Config::from_lookup(lookup(&[])).unwrap().static_dir.is_none());
    }
}
