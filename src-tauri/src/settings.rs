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
    /// Shape for new quads: "rect", "circle", "ngon" or "path" (add-quad-shapes D9).
    pub shape: String,
    /// N for new N-gon quads, 3 to 12.
    pub ngon: u32,
    /// Last normal (not maximized) window inner size in logical pixels.
    pub window: Option<[u32; 2]>,
    /// The window was maximized when it closed; it opens maximized again.
    pub maximized: bool,
    /// Atlas output on: Send and Save All also add tiles to the current workspace.
    pub atlas_on: bool,
    /// Working-folder id of the current atlas workspace; empty until one is created.
    pub current_workspace: String,
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
            shape: "rect".into(),
            ngon: 8,
            window: None,
            maximized: false,
            atlas_on: false,
            current_workspace: String::new(),
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

/// Smallest window inner size, as in tauri.conf.json.
pub const MIN_WINDOW: [f64; 2] = [900.0, 600.0];

/// Inner size for a window opened on a work area of `work` (logical pixels) whose frame adds
/// `frame` (outer minus inner). The saved size is used when it is at least the minimum,
/// otherwise `default`; either is shrunk so the whole window fits the work area, so its title
/// bar never leaves the screen and its bottom never hides under the taskbar.
pub fn fit_window(saved: Option<[u32; 2]>, default: [f64; 2], work: [f64; 2], frame: [f64; 2]) -> [f64; 2] {
    let want = match saved {
        Some([w, h]) if w as f64 >= MIN_WINDOW[0] && h as f64 >= MIN_WINDOW[1] => [w as f64, h as f64],
        _ => default,
    };
    [
        want[0].min(work[0] - frame[0]).max(1.0).floor(),
        want[1].min(work[1] - frame[1]).max(1.0).floor(),
    ]
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
            shape: "ngon".into(),
            ngon: 6,
            window: Some([1440, 900]),
            maximized: true,
            atlas_on: true,
            current_workspace: "w18f0-1a-0".into(),
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
    fn config_from_1_0_gets_atlas_defaults() {
        let p = temp("v1");
        let old = "local_dir = ''
local_on = false
drive_dir = ''
drive_on = false
aspect_mode = 'edges'
aspect_custom = [1.0, 1.0]
pot = false
window = [2194, 1211]
";
        std::fs::write(&p, old).unwrap();
        let (s, msg) = load(&p);
        assert!(msg.is_none());
        assert!(!s.atlas_on);
        assert_eq!(s.current_workspace, "");
        assert_eq!(s.window, Some([2194, 1211]));
        assert!(!s.maximized, "configs before 2.1.1 open as a normal window");
        assert_eq!((s.shape.as_str(), s.ngon), ("rect", 8), "configs before 2.3.0 get the shape defaults");
    }

    #[test]
    fn window_fits_the_work_area() {
        // 3840 x 2076 work area at 175 %: 2194.3 x 1186.3 logical; Windows 11 frame 14 x 46.
        let work = [3840.0 / 1.75, 2076.0 / 1.75];
        let frame = [14.0, 46.0];
        let [w, h] = fit_window(Some([2194, 1211]), [1440.0, 900.0], work, frame);
        assert!(w + frame[0] <= work[0] && h + frame[1] <= work[1], "{w} x {h}");
        assert_eq!(fit_window(Some([1600, 1000]), [1440.0, 900.0], work, frame), [1600.0, 1000.0], "a size that fits is kept");
        assert_eq!(fit_window(None, [1440.0, 900.0], work, frame), [1440.0, 900.0]);
        assert_eq!(fit_window(Some([300, 200]), [1440.0, 900.0], work, frame), [1440.0, 900.0], "tiny sizes are ignored");
        // A small screen shrinks even the default.
        let [w, h] = fit_window(None, [1440.0, 900.0], [1280.0, 680.0], frame);
        assert_eq!([w, h], [1266.0, 634.0]);
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
