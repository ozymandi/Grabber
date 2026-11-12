//! Persistent settings (design D10): `%APPDATA%\ripper\config.toml`, written atomically.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub local_dir: String,
    pub local_on: bool,
    pub drive_dir: String,
    pub drive_on: bool,
    /// Aspect for new quads: "edges", "1:1", "4:3", "2:1" or "custom".
    pub aspect_mode: String,
    /// Custom ratio terms `[w, h]`.
    pub aspect_custom: [f64; 2],
    pub pot: bool,
    /// Last window inner size in logical pixels.
    pub window: Option<[u32; 2]>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            local_dir: String::new(),
            local_on: false,
            drive_dir: String::new(),
            drive_on: false,
            aspect_mode: "edges".into(),
            aspect_custom: [1.0, 1.0],
            pot: false,
            window: None,
        }
    }
}

/// An output folder slot that is switched on and has a path.
#[derive(Debug, Clone, PartialEq)]
pub struct Folder {
    pub label: &'static str,
    pub dir: PathBuf,
}

impl Settings {
    /// Enabled output folders, Local first. A slot with no path counts as off.
    pub fn folders(&self) -> Vec<Folder> {
        let mut v = Vec::new();
        if self.local_on && !self.local_dir.trim().is_empty() {
            v.push(Folder { label: "Local", dir: PathBuf::from(self.local_dir.trim()) });
        }
        if self.drive_on && !self.drive_dir.trim().is_empty() {
            v.push(Folder { label: "Google Drive", dir: PathBuf::from(self.drive_dir.trim()) });
        }
        v
    }
}

/// `%APPDATA%\ripper\config.toml`.
pub fn default_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("ripper").join("config.toml")
}

/// Loads settings. A missing file gives defaults. A corrupt file is renamed to
/// `config.bad.toml` and defaults are used; the returned message says so.
pub fn load(path: &Path) -> (Settings, Option<String>) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (Settings::default(), None),
        Err(e) => return (Settings::default(), Some(format!("Cannot read settings: {e}"))),
    };
    match toml::from_str::<Settings>(&text) {
        Ok(s) => (s, None),
        Err(_) => {
            let bad = path.with_file_name("config.bad.toml");
            let _ = std::fs::remove_file(&bad);
            let moved = std::fs::rename(path, &bad).is_ok();
            let msg = if moved {
                format!("Settings were unreadable and were reset (old file kept as {})", bad.display())
            } else {
                "Settings were unreadable and were reset".to_string()
            };
            (Settings::default(), Some(msg))
        }
    }
}

/// Saves settings atomically: write a temp file next to it, then rename over the target.
pub fn save(path: &Path, s: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = toml::to_string_pretty(s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ripper-settings-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("config.toml")
    }

    #[test]
    fn round_trip() {
        let p = temp("rt");
        let s = Settings {
            local_dir: r"E:\Textures\ripped".into(),
            local_on: true,
            drive_dir: r"G:\My Drive\Textures".into(),
            drive_on: true,
            aspect_mode: "custom".into(),
            aspect_custom: [3.0, 1.0],
            pot: true,
            window: Some([1440, 900]),
        };
        save(&p, &s).unwrap();
        assert_eq!(load(&p), (s, None));
        assert!(!p.with_extension("toml.tmp").exists());
    }

    #[test]
    fn missing_file_gives_defaults() {
        let p = temp("missing");
        assert_eq!(load(&p), (Settings::default(), None));
    }

    #[test]
    fn corrupt_file_is_set_aside() {
        let p = temp("corrupt");
        std::fs::write(&p, "local_on = [this is not toml").unwrap();
        let (s, msg) = load(&p);
        assert_eq!(s, Settings::default());
        assert!(msg.unwrap().contains("config.bad.toml"));
        assert!(p.with_file_name("config.bad.toml").exists());
        assert!(!p.exists());
    }

    #[test]
    fn partial_file_fills_defaults() {
        let p = temp("partial");
        // TOML literal string: backslashes are taken as-is.
        std::fs::write(&p, "drive_dir = 'G:\\Tex'\ndrive_on = true\n").unwrap();
        let (s, msg) = load(&p);
        assert!(msg.is_none());
        assert_eq!(s.drive_dir, r"G:\Tex");
        assert!(s.drive_on);
        assert_eq!(s.aspect_mode, "edges");
    }

    #[test]
    fn folders_skip_off_and_empty_slots() {
        let mut s = Settings { local_dir: "C:\\a".into(), local_on: true, drive_on: true, ..Default::default() };
        assert_eq!(s.folders().len(), 1, "drive has no path");
        s.drive_dir = "G:\\b".into();
        assert_eq!(s.folders().iter().map(|f| f.label).collect::<Vec<_>>(), ["Local", "Google Drive"]);
        s.local_on = false;
        assert_eq!(s.folders()[0].label, "Google Drive");
    }
}
