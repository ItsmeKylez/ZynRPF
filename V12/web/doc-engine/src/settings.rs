//! `visualsettings.dat` read/update through the core's `vs` module (format logic is NOT re-implemented here).
//! Port of the desktop `vs_oku` / `vs_ayarla`, with two deliberate web differences that avoid silent failures:
//! values are validated (a value containing whitespace or a line break would corrupt or inject lines), and setting a key
//! that does not exist is an error instead of a no-op reported as success. All keys are validated before anything changes.
use crate::document::Document;
use crate::error::EngineError;
use crate::paths::validate_virtual_path;
use serde::Serialize;
use tlk_rpf_core::vs::Ayarlar;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
    /// 0-based line of the last occurrence, as the core reports it.
    pub line: usize,
    /// How many times the key occurs in the file (duplicates are all updated together).
    pub repeat: usize,
}

/// A colour stored as three scalar keys (`x.red/.green/.blue`, `x.r/.g/.b`, `xR/xG/xB`, `xRed/xGreen/xBlue`).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingColor {
    pub base: String,
    pub keys: [String; 3],
    /// 255 or 1: the value range the file uses.
    pub scale: f32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub entries: Vec<SettingEntry>,
    pub colors: Vec<SettingColor>,
}

pub const MAX_SETTING_UPDATES: usize = 5_000;

fn check_pair(key: &str, value: &str) -> Result<(), EngineError> {
    let bad = |what: &str| EngineError::new("INVALID_REQUEST", format!("Invalid setting {what}."));
    if key.is_empty() || key.len() > 256 || key.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(bad("name"));
    }
    let v = value.trim();
    if v.is_empty() || v.len() > 256 || v.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(bad(&format!("value for \"{key}\" (it must be one token without spaces or line breaks)")));
    }
    Ok(())
}

impl Document {
    fn read_settings(&self, path: &str) -> Result<(String, Ayarlar), EngineError> {
        let path = validate_virtual_path(path, false)?;
        let bytes = self.archive.dosya(&path)?.veri()?.into_owned();
        Ok((path, Ayarlar::coz(&String::from_utf8_lossy(&bytes))))
    }

    pub fn settings_view(&self, path: &str) -> Result<SettingsView, EngineError> {
        let (_, a) = self.read_settings(path)?;
        Ok(SettingsView {
            entries: a.girdiler().into_iter().map(|g| SettingEntry { key: g.anahtar, value: g.deger, line: g.satir, repeat: g.tekrar }).collect(),
            colors: a.renkler().into_iter().map(|c| SettingColor { base: c.taban, keys: c.anahtarlar, scale: c.olcek }).collect(),
        })
    }

    /// Applies `(key, value)` pairs in order as ONE undo step. Comments, line order, spacing and duplicate keys are kept by the
    /// core; a trailing `f` on float values is preserved.
    pub fn settings_apply(&mut self, path: &str, values: Vec<(String, String)>) -> Result<(), EngineError> {
        if values.is_empty() || values.len() > MAX_SETTING_UPDATES {
            return Err(EngineError::new("INVALID_REQUEST", format!("Send between 1 and {MAX_SETTING_UPDATES} settings.")));
        }
        let (path, probe) = self.read_settings(path)?;
        for (k, v) in &values {
            check_pair(k, v)?;
            if probe.deger(k).is_none() {
                return Err(EngineError::new("NOT_FOUND", format!("Setting \"{k}\" does not exist in this file.")));
            }
        }
        self.edit(|arch| {
            let f = arch.dosya_mut(&path)?;
            let mut a = Ayarlar::coz(&String::from_utf8_lossy(&f.veri()?));
            for (k, v) in &values {
                a.ayarla(k, v);
            }
            f.degistir(a.metin().into_bytes());
            Ok(())
        })
    }
}
