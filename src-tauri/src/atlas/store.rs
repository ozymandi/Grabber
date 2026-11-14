//! Workspace storage (design D1, D3 of add-texture-atlas).
//!
//! Each workspace lives in a working folder `<root>/<id>/` with `manifest.json` and
//! `items/*.png`. Every mutation runs lock -> re-read -> apply -> atomic write, so a ShareX
//! process and an open atlas window never overwrite each other. A `.ripper` file is a zip
//! of the same content, written on SAVE.

use super::model::{Manifest, Refused};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MANIFEST: &str = "manifest.json";
const LOCK: &str = "manifest.lock";
const ITEMS: &str = "items";
const INDEX: &str = "index.json";
/// How long a writer waits for the lock before giving up.
pub const LOCK_WAIT: Duration = Duration::from_secs(2);
/// A lock older than this belongs to a writer that died; operations hold it for milliseconds.
const LOCK_STALE: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    /// Another ripper window held the workspace for longer than [`LOCK_WAIT`].
    Busy,
    Refused(String),
    Corrupt(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Busy => write!(f, "the workspace is busy in another ripper window"),
            Error::Refused(m) => write!(f, "{m}"),
            Error::Corrupt(m) => write!(f, "the workspace file is damaged: {m}"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<Refused> for Error {
    fn from(r: Refused) -> Self {
        Error::Refused(r.0)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// `%APPDATA%\ripper\workspaces`.
pub fn default_root() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("ripper").join("workspaces")
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A short unique name from the clock, the process and a counter.
fn unique(prefix: &str) -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}{nanos:x}-{:x}-{n:x}", std::process::id())
}

/// Writes `bytes` to `path` through a temporary file and a rename.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension(format!("tmp-{}", unique("")));
    {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Only plain file names may come out of an archive or a manifest.
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() < 128
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && !name.starts_with('.')
}

/// Where a working folder's document lives and which revision it was saved at.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub file: Option<String>,
    pub saved_revision: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Index {
    entries: BTreeMap<String, IndexEntry>,
}

/// All working folders under one root.
#[derive(Debug, Clone)]
pub struct Store {
    pub root: PathBuf,
}

/// One working folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub dir: PathBuf,
}

/// What the UI needs besides the manifest.
#[derive(Debug, Clone, Serialize)]
pub struct Doc {
    pub id: String,
    pub file: Option<String>,
    pub unsaved: bool,
    pub manifest: Manifest,
}

impl Store {
    pub fn new(root: PathBuf) -> Store {
        Store { root }
    }

    fn index_path(&self) -> PathBuf {
        self.root.join(INDEX)
    }

    fn read_index(&self) -> Index {
        fs::read_to_string(self.index_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn update_index(&self, f: impl FnOnce(&mut Index)) -> io::Result<()> {
        fs::create_dir_all(&self.root)?;
        let mut idx = self.read_index();
        f(&mut idx);
        write_atomic(&self.index_path(), serde_json::to_string_pretty(&idx).unwrap().as_bytes())
    }

    pub fn entry(&self, id: &str) -> IndexEntry {
        self.read_index().entries.get(id).cloned().unwrap_or_default()
    }

    /// Creates an empty untitled workspace.
    pub fn create(&self) -> Result<Workspace> {
        let ws = self.workspace(&unique("w"));
        fs::create_dir_all(ws.dir.join(ITEMS))?;
        write_atomic(&ws.dir.join(MANIFEST), serde_json::to_string_pretty(&Manifest::default()).unwrap().as_bytes())?;
        self.update_index(|i| {
            i.entries.insert(ws.id.clone(), IndexEntry::default());
        })?;
        Ok(ws)
    }

    pub fn workspace(&self, id: &str) -> Workspace {
        Workspace { id: id.to_string(), dir: self.root.join(id) }
    }

    /// The working folder for `id` if it still exists.
    pub fn existing(&self, id: &str) -> Option<Workspace> {
        if id.is_empty() || !safe_name(id) {
            return None;
        }
        let ws = self.workspace(id);
        ws.dir.join(MANIFEST).is_file().then_some(ws)
    }

    pub fn doc(&self, ws: &Workspace) -> Result<Doc> {
        let manifest = ws.read()?;
        let e = self.entry(&ws.id);
        let unsaved = e.saved_revision != manifest.revision;
        Ok(Doc { id: ws.id.clone(), file: e.file, unsaved, manifest })
    }

    /// Writes the workspace to `path` as a `.ripper` zip and records it as saved. Items no
    /// tile references are left out of the archive and removed from the working folder.
    pub fn save_as(&self, ws: &Workspace, path: &Path) -> Result<Doc> {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").to_string();
        let (manifest, ()) = ws.mutate(|m| {
            m.name = stem.clone();
            Ok(())
        })?;
        let _guard = ws.lock()?;
        let manifest = ws.read().unwrap_or(manifest);
        let used: std::collections::BTreeSet<&str> = manifest.tiles.iter().map(|t| t.item.as_str()).collect();
        for e in fs::read_dir(ws.dir.join(ITEMS))? {
            let e = e?;
            let name = e.file_name().to_string_lossy().to_string();
            if !used.contains(name.as_str()) {
                let _ = fs::remove_file(e.path());
            }
        }
        let tmp = path.with_extension(format!("ripper.tmp-{}", unique("")));
        let result = (|| -> Result<()> {
            let mut zw = zip::ZipWriter::new(File::create(&tmp)?);
            let deflated = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            let zerr = |e: zip::result::ZipError| Error::Io(io::Error::other(e));
            zw.start_file(MANIFEST, deflated).map_err(zerr)?;
            zw.write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes())?;
            for item in &used {
                zw.start_file(format!("{ITEMS}/{item}"), stored).map_err(zerr)?;
                zw.write_all(&fs::read(ws.item_path(item))?)?;
            }
            let f = zw.finish().map_err(zerr)?;
            f.sync_all()?;
            Ok(())
        })();
        if let Err(e) = result {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
        fs::rename(&tmp, path).inspect_err(|_| {
            let _ = fs::remove_file(&tmp);
        })?;
        let file = path.to_string_lossy().to_string();
        let rev = manifest.revision;
        self.update_index(|i| {
            i.entries.insert(ws.id.clone(), IndexEntry { file: Some(file), saved_revision: rev });
        })?;
        drop(_guard);
        self.doc(ws)
    }

    /// Opens a `.ripper` file. A working folder for the same file with unsaved changes is
    /// newer than the file and wins (recovery after a closed window); otherwise the file is
    /// unpacked into a fresh working folder.
    pub fn open(&self, path: &Path) -> Result<Workspace> {
        let key = path.to_string_lossy().to_lowercase();
        let idx = self.read_index();
        for (id, e) in &idx.entries {
            if e.file.as_deref().map(|f| f.to_lowercase()) == Some(key.clone()) {
                if let Some(ws) = self.existing(id) {
                    if ws.read().map(|m| m.revision != e.saved_revision).unwrap_or(false) {
                        return Ok(ws);
                    }
                }
            }
        }
        let ws = self.workspace(&unique("w"));
        fs::create_dir_all(ws.dir.join(ITEMS))?;
        let unpacked = (|| -> Result<Manifest> {
            let mut za = zip::ZipArchive::new(File::open(path)?).map_err(|e| Error::Corrupt(e.to_string()))?;
            let mut text = String::new();
            za.by_name(MANIFEST).map_err(|e| Error::Corrupt(e.to_string()))?.read_to_string(&mut text)?;
            let m: Manifest = serde_json::from_str(&text).map_err(|e| Error::Corrupt(e.to_string()))?;
            for t in &m.tiles {
                if !safe_name(&t.item) {
                    return Err(Error::Corrupt(format!("bad item name {:?}", t.item)));
                }
                let mut bytes = Vec::new();
                za.by_name(&format!("{ITEMS}/{}", t.item))
                    .map_err(|e| Error::Corrupt(format!("{}: {e}", t.item)))?
                    .read_to_end(&mut bytes)?;
                write_atomic(&ws.item_path(&t.item), &bytes)?;
            }
            write_atomic(&ws.dir.join(MANIFEST), text.as_bytes())?;
            Ok(m)
        })();
        let m = match unpacked {
            Ok(m) => m,
            Err(e) => {
                let _ = fs::remove_dir_all(&ws.dir);
                return Err(e);
            }
        };
        // Older working folders of this file had no unsaved changes; drop them.
        let stale: Vec<String> = idx
            .entries
            .iter()
            .filter(|(_, e)| e.file.as_deref().map(|f| f.to_lowercase()) == Some(key.clone()))
            .map(|(id, _)| id.clone())
            .collect();
        let file = path.to_string_lossy().to_string();
        self.update_index(|i| {
            for id in &stale {
                i.entries.remove(id);
            }
            i.entries.insert(ws.id.clone(), IndexEntry { file: Some(file), saved_revision: m.revision });
        })?;
        for id in stale {
            if safe_name(&id) {
                let _ = fs::remove_dir_all(self.root.join(id));
            }
        }
        Ok(ws)
    }
}

/// Holds `manifest.lock` until dropped.
pub struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl Workspace {
    pub fn item_path(&self, item: &str) -> PathBuf {
        self.dir.join(ITEMS).join(item)
    }

    pub fn read(&self) -> Result<Manifest> {
        let text = fs::read_to_string(self.dir.join(MANIFEST))?;
        serde_json::from_str(&text).map_err(|e| Error::Corrupt(e.to_string()))
    }

    /// The manifest's revision, read cheaply for change polling.
    pub fn revision(&self) -> Result<u64> {
        Ok(self.read()?.revision)
    }

    /// Takes the workspace lock, waiting up to [`LOCK_WAIT`]; breaks a stale lock.
    pub fn lock(&self) -> Result<LockGuard> {
        self.lock_for(LOCK_WAIT)
    }

    pub fn lock_for(&self, wait: Duration) -> Result<LockGuard> {
        let path = self.dir.join(LOCK);
        let start = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut f) => {
                    let _ = write!(f, "{} {}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0));
                    return Ok(LockGuard { path });
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists || e.kind() == io::ErrorKind::PermissionDenied => {
                    let stale = fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .map(|t| t.elapsed().unwrap_or_default() > LOCK_STALE)
                        .unwrap_or(false);
                    if stale {
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    if start.elapsed() >= wait {
                        return Err(Error::Busy);
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Runs `f` on the latest manifest under the lock and writes the result with the next
    /// revision. Nothing is written when `f` refuses.
    pub fn mutate<R>(&self, f: impl FnOnce(&mut Manifest) -> std::result::Result<R, Refused>) -> Result<(Manifest, R)> {
        let _guard = self.lock()?;
        let mut m = self.read()?;
        let r = f(&mut m)?;
        m.revision += 1;
        write_atomic(&self.dir.join(MANIFEST), serde_json::to_string_pretty(&m).unwrap().as_bytes())?;
        Ok((m, r))
    }

    /// Adds a captured texture: stores the original, then places it (design D4) under the
    /// lock so concurrent writers each see the other's tiles.
    pub fn add_tile(&self, png: &[u8], w0: u32, h0: u32, source: super::model::Provenance) -> Result<(Manifest, super::place::Placement)> {
        let item = self.add_item(png)?;
        let r = self.mutate(|m| {
            let p = super::place::place_new(m, w0, h0);
            let id = m.next_id;
            m.next_id += 1;
            m.tiles.push(super::model::Tile {
                id,
                item: item.clone(),
                w0,
                h0,
                x: p.rect.x,
                y: p.rect.y,
                w: p.rect.w,
                h: p.rect.h,
                rot: 0,
                source,
            });
            Ok(p)
        });
        if r.is_err() {
            let _ = fs::remove_file(self.item_path(&item));
        }
        r
    }

    /// Stores a new original and returns its item name. Called before the manifest that
    /// references it is written, so a manifest never points at a missing file.
    pub fn add_item(&self, png: &[u8]) -> Result<String> {
        fs::create_dir_all(self.dir.join(ITEMS))?;
        let name = format!("{}.png", unique("i"));
        write_atomic(&self.item_path(&name), png)?;
        Ok(name)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::atlas::model::{Provenance, Tile};

    pub fn temp_store(tag: &str) -> Store {
        let root = std::env::temp_dir().join(format!("ripper-atlas-test-{tag}-{}", unique("")));
        let _ = fs::remove_dir_all(&root);
        Store::new(root)
    }

    pub fn push_tile(m: &mut Manifest, item: &str, x: i64, y: i64, w: i64, h: i64) -> u64 {
        let id = m.next_id;
        m.next_id += 1;
        m.tiles.push(Tile { id, item: item.into(), w0: w as u32, h0: h as u32, x, y, w, h, rot: 0, source: Provenance::default() });
        id
    }

    #[test]
    fn fresh_workspace_and_unsaved_flag() {
        let store = temp_store("fresh");
        let ws = store.create().unwrap();
        let doc = store.doc(&ws).unwrap();
        assert_eq!(doc.manifest.sectors.len(), 1);
        assert_eq!(doc.manifest.sector_size, 2048);
        assert_eq!(doc.manifest.padding, 8);
        assert!(!doc.unsaved);
        let (m, ()) = ws.mutate(|m| m.set_padding(4)).unwrap();
        assert_eq!(m.revision, 1);
        assert!(store.doc(&ws).unwrap().unsaved);
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn refused_mutation_writes_nothing() {
        let store = temp_store("refused");
        let ws = store.create().unwrap();
        assert!(matches!(ws.mutate(|m| m.set_sector_size(1000)), Err(Error::Refused(_))));
        assert_eq!(ws.read().unwrap().revision, 0);
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn two_writers_lose_nothing() {
        let store = temp_store("concurrent");
        let ws = store.create().unwrap();
        let handles: Vec<_> = (0..2)
            .map(|k| {
                let ws = ws.clone();
                std::thread::spawn(move || {
                    for i in 0..50 {
                        let item = ws.add_item(format!("png {k} {i}").as_bytes()).unwrap();
                        ws.mutate(|m| {
                            push_tile(m, &item, i * 10, -100, 8, 8);
                            Ok(())
                        })
                        .unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let m = ws.read().unwrap();
        assert_eq!(m.tiles.len(), 100);
        assert_eq!(m.revision, 100);
        let ids: std::collections::BTreeSet<u64> = m.tiles.iter().map(|t| t.id).collect();
        assert_eq!(ids.len(), 100, "ids unique");
        for t in &m.tiles {
            assert!(ws.item_path(&t.item).is_file(), "item {} present", t.item);
        }
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn held_lock_fails_with_busy_not_hang() {
        let store = temp_store("busy");
        let ws = store.create().unwrap();
        let _held = ws.lock().unwrap();
        let t = Instant::now();
        let r = ws.mutate(|_| Ok(()));
        assert!(matches!(r, Err(Error::Busy)));
        let e = t.elapsed();
        assert!(e >= Duration::from_millis(1900) && e < Duration::from_secs(4), "waited {e:?}");
        drop(_held);
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn save_new_open_roundtrip_and_purge() {
        let store = temp_store("roundtrip");
        let ws = store.create().unwrap();
        let mut keep = Vec::new();
        for i in 0..7 {
            let item = ws.add_item(format!("image bytes {i}").as_bytes()).unwrap();
            keep.push(item.clone());
            ws.mutate(|m| {
                let id = push_tile(m, &item, i * 100, -300, 90, 40);
                if i % 2 == 1 {
                    m.rotate_tiles(&[id], true)?;
                }
                Ok(())
            })
            .unwrap();
        }
        // Delete the last tile: its item must be purged on save.
        let purged = keep.pop().unwrap();
        ws.mutate(|m| {
            let id = m.tiles.last().unwrap().id;
            m.delete_tiles(&[id]);
            Ok(())
        })
        .unwrap();
        let file = store.root.join("cafe.ripper");
        let saved = store.save_as(&ws, &file).unwrap();
        assert!(!saved.unsaved);
        assert_eq!(saved.manifest.name, "cafe");
        assert!(!ws.item_path(&purged).exists(), "unreferenced item purged");

        let a = ws.read().unwrap();
        let bytes: Vec<Vec<u8>> = a.tiles.iter().map(|t| fs::read(ws.item_path(&t.item)).unwrap()).collect();
        let fresh = store.create().unwrap();
        assert_ne!(fresh.id, ws.id);
        // The saved working folder has no unsaved changes, so OPEN unpacks the file afresh
        // and drops the old folder.
        let opened = store.open(&file).unwrap();
        assert_ne!(opened.id, ws.id);
        assert!(!ws.dir.exists());
        let b = opened.read().unwrap();
        assert_eq!(b.tiles.len(), 6);
        assert_eq!(a.tiles, b.tiles, "same rects, rotations and items");
        for (t, want) in b.tiles.iter().zip(&bytes) {
            assert_eq!(&fs::read(opened.item_path(&t.item)).unwrap(), want);
        }
        assert!(!store.doc(&opened).unwrap().unsaved);
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn open_prefers_unsaved_working_copy() {
        let store = temp_store("recovery");
        let ws = store.create().unwrap();
        let file = store.root.join("shop.ripper");
        store.save_as(&ws, &file).unwrap();
        ws.mutate(|m| m.set_padding(12)).unwrap();
        let opened = store.open(&file).unwrap();
        assert_eq!(opened, ws, "unsaved working folder wins");
        assert_eq!(opened.read().unwrap().padding, 12);
        assert!(store.doc(&opened).unwrap().unsaved);
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn add_tile_keeps_original_and_fits_down() {
        let store = temp_store("addtile");
        let ws = store.create().unwrap();
        ws.mutate(|m| m.set_sector_size(1024)).unwrap();
        let (m, p) = ws.add_tile(b"png", 1500, 900, Provenance { stem: "shot".into(), quad: 2 }).unwrap();
        assert!(p.fitted);
        let t = &m.tiles[0];
        assert_eq!((t.w0, t.h0), (1500, 900));
        assert_eq!((t.w, t.h), (1008, 604));
        assert_eq!(t.source.quad, 2);
        assert_eq!(fs::read(ws.item_path(&t.item)).unwrap(), b"png");
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn damaged_archive_is_reported() {
        let store = temp_store("corrupt");
        fs::create_dir_all(&store.root).unwrap();
        let bad = store.root.join("bad.ripper");
        fs::write(&bad, b"not a zip").unwrap();
        assert!(matches!(store.open(&bad), Err(Error::Corrupt(_))));
        let _ = fs::remove_dir_all(&store.root);
    }
}
