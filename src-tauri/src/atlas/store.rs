//! Workspace storage (design D1, D3 of add-texture-atlas).
//!
//! Each workspace lives in a working folder `<root>/<id>/` with `manifest.json` and
//! `items/*.png`. Every mutation runs lock -> re-read -> apply -> atomic write, so a ShareX
//! process and an open atlas window never overwrite each other. A `.ripper` file is a zip
//! of the same content, written on SAVE.

use super::model::{parse_manifest, Manifest, ParseError, Refused};
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
    /// The file was written by a newer ripper format.
    NewerFormat(u32),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Busy => write!(f, "the workspace is busy in another ripper window"),
            Error::Refused(m) => write!(f, "{m}"),
            Error::Corrupt(m) => write!(f, "the workspace file is damaged: {m}"),
            Error::NewerFormat(n) => write!(f, "this file needs a newer ripper (workspace format {n})"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        match e {
            ParseError::Corrupt(m) => Error::Corrupt(m),
            ParseError::Newer(n) => Error::NewerFormat(n),
        }
    }
}

impl From<Refused> for Error {
    fn from(r: Refused) -> Self {
        Error::Refused(r.0)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Result of an undo or redo: the manifest, the inverse of what was applied, the skipped
/// reasons and the items that left the manifest.
pub type Steps = (Manifest, Vec<super::edit::Edit>, Vec<String>, Vec<String>);

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
    /// tile or guide references are left out of the archive and removed from the working folder.
    pub fn save_as(&self, ws: &Workspace, path: &Path) -> Result<Doc> {
        self.save_as_keeping(ws, path, &std::collections::HashSet::new())
    }

    /// [`Store::save_as`] that leaves the items in `keep` in the working folder (they are still
    /// referenced by this window's undo history) without packing them.
    pub fn save_as_keeping(&self, ws: &Workspace, path: &Path, keep: &std::collections::HashSet<String>) -> Result<Doc> {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").to_string();
        let (manifest, ()) = ws.mutate(|m| {
            m.name = stem.clone();
            Ok(())
        })?;
        let _guard = ws.lock()?;
        let manifest = ws.read().unwrap_or(manifest);
        let used = manifest.items();
        for e in fs::read_dir(ws.dir.join(ITEMS))? {
            let e = e?;
            let name = e.file_name().to_string_lossy().to_string();
            if !used.contains(name.as_str()) && !keep.contains(&name) {
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
            let m = parse_manifest(&text)?;
            for item in m.items() {
                if !safe_name(item) {
                    return Err(Error::Corrupt(format!("bad item name {item:?}")));
                }
                let mut bytes = Vec::new();
                za.by_name(&format!("{ITEMS}/{item}"))
                    .map_err(|e| Error::Corrupt(format!("{item}: {e}")))?
                    .read_to_end(&mut bytes)?;
                write_atomic(&ws.item_path(item), &bytes)?;
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
        Ok(parse_manifest(&text)?)
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
            let mut t = super::model::Tile {
                id,
                item: item.clone(),
                w0,
                h0,
                frame: super::model::Frame::from_rect(p.rect),
                source,
                history: Vec::new(),
                current: 0,
            };
            t.ensure_history("RIP");
            t.history[0].at = super::model::now();
            m.tiles.push(t);
            Ok(p)
        });
        if r.is_err() {
            let _ = fs::remove_file(self.item_path(&item));
        }
        r
    }

    /// Applies one forward edit under the lock; a refusal writes nothing.
    pub fn edit(&self, e: super::edit::Edit) -> Result<(Manifest, super::edit::Applied)> {
        self.mutate(|m| super::edit::apply(m, e))
    }

    /// Undo or redo (design D6): applies `edits` best effort under one lock. Edits that would
    /// bring back an original no longer on disk are skipped up front. Returns the manifest, the
    /// inverse of what was applied, the skipped reasons and the items that left the manifest.
    pub fn apply_steps(&self, edits: Vec<super::edit::Edit>) -> Result<Steps> {
        use super::edit::Edit;
        let mut skipped = Vec::new();
        let edits: Vec<Edit> = edits
            .into_iter()
            .filter(|e| {
                let (item, what) = match e {
                    Edit::RestoreTile { tile, .. } => (&tile.item, format!("the original of tile {} is gone", tile.id)),
                    Edit::SetGuide { guide: Some(g), .. } => (&g.item, "the guide image is gone".to_string()),
                    _ => return true,
                };
                let ok = self.item_path(item).is_file();
                if !ok {
                    skipped.push(what);
                }
                ok
            })
            .collect();
        let (m, (inverse, more, released)) = self.mutate(|m| Ok(super::edit::apply_steps(m, edits)))?;
        skipped.extend(more);
        Ok((m, inverse, skipped, released))
    }

    /// Loads guide images (design D4): `files` are (file name, PNG bytes). Each image is
    /// stored as an item first, then all are placed under the lock. Items of a refused load
    /// are removed again.
    pub fn load_guides(&self, files: &[(String, Vec<u8>)], target: Option<super::model::Sector>) -> Result<(Manifest, super::model::GuideReport)> {
        use super::model::{udim_in_name, GuideTarget};
        let mut loads = Vec::new();
        let mut written = Vec::new();
        let result = (|| -> Result<(Manifest, super::model::GuideReport)> {
            for (name, bytes) in files {
                let (w, h) = image::ImageReader::new(io::Cursor::new(bytes))
                    .with_guessed_format()?
                    .into_dimensions()
                    .map_err(|e| Error::Refused(format!("{name} is not a readable image: {e}")))?;
                let t = match udim_in_name(name) {
                    Some(n) => GuideTarget::Udim(n),
                    None => GuideTarget::Single(target),
                };
                let item = self.add_item(bytes)?;
                written.push(item.clone());
                loads.push((t, item, w, h));
            }
            self.mutate(|m| m.place_guides(&loads))
        })();
        if result.is_err() {
            for item in written {
                let _ = fs::remove_file(self.item_path(&item));
            }
        }
        result
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
    use crate::atlas::model::{Frame, Provenance, Rect, Tile};

    pub fn temp_store(tag: &str) -> Store {
        let root = std::env::temp_dir().join(format!("ripper-atlas-test-{tag}-{}", unique("")));
        let _ = fs::remove_dir_all(&root);
        Store::new(root)
    }

    pub fn push_tile(m: &mut Manifest, item: &str, x: i64, y: i64, w: i64, h: i64) -> u64 {
        let id = m.next_id;
        m.next_id += 1;
        m.tiles.push(Tile { id, item: item.into(), w0: w as u32, h0: h as u32, frame: Frame::from_rect(Rect { x, y, w, h }), source: Provenance::default(), history: Vec::new(), current: 0 });
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
        assert_eq!((t.frame.w, t.frame.h), (1008, 604));
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

    fn zip_with(path: &Path, manifest: &str, items: &[(&str, &[u8])]) {
        let mut zw = zip::ZipWriter::new(File::create(path).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        zw.start_file(MANIFEST, o).unwrap();
        zw.write_all(manifest.as_bytes()).unwrap();
        for (name, bytes) in items {
            zw.start_file(format!("{ITEMS}/{name}"), o).unwrap();
            zw.write_all(bytes).unwrap();
        }
        zw.finish().unwrap();
    }

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::new();
        image::RgbaImage::from_pixel(w, h, image::Rgba([255, 255, 255, 128]))
            .write_to(&mut io::Cursor::new(&mut v), image::ImageFormat::Png)
            .unwrap();
        v
    }

    #[test]
    fn guides_travel_with_the_file() {
        use super::super::model::{Fill, Sector};
        let store = temp_store("guides");
        let ws = store.create().unwrap();
        let (m, rep) = ws
            .load_guides(&[("body.1001.png".into(), png(16, 16)), ("body.1002.png".into(), png(32, 16))], None)
            .unwrap();
        assert_eq!((rep.loaded, rep.created.len(), rep.not_square), (2, 1, 1));
        assert_eq!(m.guides.len(), 2);
        ws.mutate(|m| {
            m.set_guide_view(Some(false), Some(30))?;
            m.set_fill(Fill::Extend, 32)
        })
        .unwrap();
        let f = store.root.join("cafe.ripper");
        store.save_as(&ws, &f).unwrap();
        let fresh = store.create().unwrap();
        assert!(fresh.read().unwrap().guides.is_empty());
        // Opening the file from a different store mimics another machine.
        let other = temp_store("guides-other");
        let opened = other.open(&f).unwrap();
        let m = opened.read().unwrap();
        assert_eq!(m.guides.len(), 2);
        assert_eq!(m.guide(Sector { u: 1, v: 0 }).unwrap().w, 32);
        for g in &m.guides {
            assert!(opened.item_path(&g.item).is_file(), "guide image unpacked");
        }
        assert_eq!((m.guide_show, m.guide_opacity, m.fill, m.edge), (false, 30, Fill::Extend, 32));
        let _ = fs::remove_dir_all(&store.root);
        let _ = fs::remove_dir_all(&other.root);
    }

    #[test]
    fn refused_guide_load_leaves_no_items() {
        let store = temp_store("guides-refused");
        let ws = store.create().unwrap();
        let r = ws.load_guides(&[("a.png".into(), png(4, 4)), ("b.png".into(), png(4, 4))], None);
        assert!(matches!(r, Err(Error::Refused(_))));
        assert_eq!(fs::read_dir(ws.dir.join(ITEMS)).map(|d| d.count()).unwrap_or(0), 0);
        let r = ws.load_guides(&[("c.png".into(), b"not an image".to_vec())], None);
        assert!(r.is_err());
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn held_items_survive_save_but_are_not_packed() {
        let store = temp_store("held");
        let ws = store.create().unwrap();
        let item = ws.add_item(b"gone").unwrap();
        let f = store.root.join("held.ripper");
        let keep = std::collections::HashSet::from([item.clone()]);
        store.save_as_keeping(&ws, &f, &keep).unwrap();
        assert!(ws.item_path(&item).is_file(), "held item stays on disk");
        let za = zip::ZipArchive::new(File::open(&f).unwrap()).unwrap();
        assert!(!za.file_names().any(|n| n.map(|n| n.contains(&item)).unwrap_or(false)), "held item is not packed");
        store.save_as(&ws, &f).unwrap();
        assert!(!ws.item_path(&item).is_file(), "released item is purged");
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn newer_format_file_is_refused_and_leaves_nothing_behind() {
        let store = temp_store("newer");
        fs::create_dir_all(&store.root).unwrap();
        let before: Vec<_> = fs::read_dir(&store.root).unwrap().collect();
        let f = store.root.join("future.ripper");
        zip_with(&f, r#"{"format":4,"revision":1,"name":"future","sector_size":1024,"padding":8,"sectors":[],"active":null,"tiles":[],"next_id":1}"#, &[]);
        assert!(matches!(store.open(&f), Err(Error::NewerFormat(4))));
        let after: Vec<_> = fs::read_dir(&store.root).unwrap().filter_map(|e| e.ok()).filter(|e| e.path() != f).collect();
        assert_eq!(after.len(), before.len(), "no working folder left behind");
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn format_one_file_opens_with_the_same_placement() {
        let store = temp_store("v1");
        fs::create_dir_all(&store.root).unwrap();
        let f = store.root.join("cafe.ripper");
        let v1 = r#"{"format":1,"revision":2,"name":"cafe","sector_size":1024,"padding":8,"sectors":[{"u":0,"v":0}],"active":null,"next_id":3,
            "tiles":[{"id":1,"item":"a.png","w0":40,"h0":20,"x":10,"y":-500,"w":40,"h":20,"rot":0},
                     {"id":2,"item":"b.png","w0":5,"h0":2,"x":7,"y":-33,"w":2,"h":5,"rot":3}]}"#;
        zip_with(&f, v1, &[("a.png", b"a"), ("b.png", b"b")]);
        let ws = store.open(&f).unwrap();
        let m = ws.read().unwrap();
        assert_eq!(m.format, super::super::model::FORMAT);
        assert_eq!(m.tiles[0].rect(), Rect { x: 10, y: -500, w: 40, h: 20 });
        assert_eq!(m.tiles[1].rect(), Rect { x: 7, y: -33, w: 2, h: 5 });
        assert_eq!(m.tiles[1].frame.rot, 3);
        let _ = fs::remove_dir_all(&store.root);
    }
}
