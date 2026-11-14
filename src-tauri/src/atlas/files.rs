//! File-level atlas work that needs no Tauri: writing exports to a folder and serving
//! downscaled display copies of originals (design D5 of add-texture-atlas).

use super::model::{Refused, Sector, Tile};
use super::render::{self, Image, Scope};
use super::store::{Error, Result, Workspace};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Decodes a stored original to straight-alpha RGBA8.
pub fn load_original(ws: &Workspace, t: &Tile) -> std::result::Result<Image, String> {
    let bytes = fs::read(ws.item_path(&t.item)).map_err(|e| format!("{}: {e}", t.item))?;
    let img = image::load_from_memory(&bytes).map_err(|e| format!("{}: {e}", t.item))?.to_rgba8();
    Ok(Image { w: img.width(), h: img.height(), rgba: img.into_raw() })
}

pub fn encode_png(img: &Image) -> Vec<u8> {
    let mut out = Vec::new();
    let enc = image::codecs::png::PngEncoder::new(&mut out);
    image::ImageEncoder::write_image(enc, &img.rgba, img.w, img.h, image::ExtendedColorType::Rgba8)
        .expect("PNG encoding to memory cannot fail for valid RGBA8");
    out
}

/// Exports `selected` sectors (the active sector when none) into `dir` under `base`. Never
/// overwrites: the whole export gets `_2`, `_3`... when any of its names exists. Returns the
/// written paths in order.
pub fn export(ws: &Workspace, selected: &[Sector], scope: Scope, dir: &Path, base: &str) -> Result<Vec<PathBuf>> {
    let m = ws.read()?;
    let mut sel = selected.to_vec();
    if sel.is_empty() {
        sel.extend(m.active.or_else(|| m.sectors_udim_order().first().copied()));
    }
    let outputs = render::plan(&m, &sel, scope)?;
    if !dir.is_dir() {
        return Err(Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "folder not found")));
    }
    let base = sanitize(base);
    if base.is_empty() {
        return Err(Refused("Enter a file name for the export".into()).into());
    }
    let stem = render::free_stem(&base, &outputs, &|n| dir.join(n).exists());
    let mut written = Vec::new();
    for o in &outputs {
        let mut load = |t: &Tile| load_original(ws, t);
        let img = render::render_area(&m, (o.area.x, o.area.y), o.area.w as u32, o.area.h as u32, &o.cells, &mut load)
            .map_err(|e| Error::Corrupt(e))?;
        let path = dir.join(format!("{stem}{}", o.tail));
        let mut f = OpenOptions::new().write(true).create_new(true).open(&path)?;
        f.write_all(&encode_png(&img))?;
        f.sync_all()?;
        written.push(path);
    }
    Ok(written)
}

/// Keeps a base name usable as a Windows file name.
pub fn sanitize(name: &str) -> String {
    name.trim()
        .chars()
        .map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { '_' } else { c })
        .collect::<String>()
        .trim_end_matches(['.', ' '])
        .to_string()
}

/// A display copy of an original, at most `max` px on its long side, cached as
/// `thumbs/<item>-<max>.png`. Returns PNG bytes.
pub fn thumbnail(ws: &Workspace, item: &str, max: u32) -> Result<Vec<u8>> {
    let cache = ws.dir.join("thumbs").join(format!("{item}-{max}.png"));
    if let Ok(b) = fs::read(&cache) {
        return Ok(b);
    }
    let bytes = fs::read(ws.item_path(item))?;
    let img = image::load_from_memory(&bytes).map_err(|e| Error::Corrupt(e.to_string()))?;
    let (w, h) = (img.width(), img.height());
    let out = if w.max(h) <= max {
        bytes
    } else {
        let s = max as f64 / w.max(h) as f64;
        let small = img.resize_exact(((w as f64 * s).round() as u32).max(1), ((h as f64 * s).round() as u32).max(1), image::imageops::FilterType::Triangle);
        let mut v = Vec::new();
        small.to_rgba8().write_to(&mut std::io::Cursor::new(&mut v), image::ImageFormat::Png).map_err(|e| Error::Corrupt(e.to_string()))?;
        v
    };
    if let Some(d) = cache.parent() {
        let _ = fs::create_dir_all(d);
        let _ = fs::write(&cache, &out);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::model::{Provenance, Side};
    use crate::atlas::store::tests::temp_store;

    fn png(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        encode_png(&Image { w, h, rgba: rgba.iter().copied().cycle().take((w * h * 4) as usize).collect() })
    }

    #[test]
    fn export_set_writes_numbered_files_without_overwrite() {
        let store = temp_store("export");
        let ws = store.create().unwrap();
        ws.mutate(|m| {
            m.set_sector_size(512)?;
            m.add_sector(Sector { u: 0, v: 0 }, Side::Right)?;
            Ok(())
        })
        .unwrap();
        ws.add_tile(&png(100, 50, [10, 20, 30, 255]), 100, 50, Provenance::default()).unwrap();
        let dir = store.root.join("out");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("cafe.1001.png"), b"existing").unwrap();
        let sel = [Sector { u: 0, v: 0 }, Sector { u: 1, v: 0 }];
        let written = export(&ws, &sel, Scope::GroupSet, &dir, "cafe").unwrap();
        let names: Vec<String> = written.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(names, ["cafe_2.1001.png", "cafe_2.1002.png"]);
        assert_eq!(fs::read(dir.join("cafe.1001.png")).unwrap(), b"existing");
        let first = image::open(&written[0]).unwrap().to_rgba8();
        assert_eq!(first.dimensions(), (512, 512));
        assert_eq!(first.get_pixel(8, 8).0, [10, 20, 30, 255], "tile placed at the padding offset");
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn export_without_selection_uses_active_sector() {
        let store = temp_store("export-active");
        let ws = store.create().unwrap();
        ws.mutate(|m| {
            m.set_sector_size(512)?;
            let n = m.add_sector(Sector { u: 0, v: 0 }, Side::Up)?;
            m.set_active(n)
        })
        .unwrap();
        let dir = store.root.join("out");
        fs::create_dir_all(&dir).unwrap();
        let written = export(&ws, &[], Scope::Sector, &dir, "one").unwrap();
        assert_eq!(written.len(), 1);
        assert!(written[0].ends_with("one.png"));
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn thumbnail_is_capped_and_cached() {
        let store = temp_store("thumb");
        let ws = store.create().unwrap();
        let item = ws.add_item(&png(300, 100, [1, 2, 3, 255])).unwrap();
        let t = thumbnail(&ws, &item, 64).unwrap();
        let img = image::load_from_memory(&t).unwrap();
        assert_eq!((img.width(), img.height()), (64, 21));
        assert!(ws.dir.join("thumbs").join(format!("{item}-64.png")).is_file());
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn sanitize_names() {
        assert_eq!(sanitize(" cafe:shop?. "), "cafe_shop_");
        assert_eq!(sanitize("..."), "");
    }
}
