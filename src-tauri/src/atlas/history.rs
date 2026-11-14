//! Tile picture histories (add-mcp-editing D4, D6; spec atlas-history): writing new pictures as
//! states, keeping the original's alpha, going to a state, and the sector log. Manifest work
//! is pure; [`Workspace`] methods add the item files and run it under the workspace lock.

use super::model::{now, Fill, Filter, Manifest, PictureState, Rect, Refused, Sector, SectorChange, SectorEntry};
use super::render::{self, Image};
use super::store::{Result, Workspace};
use serde::Serialize;
use std::collections::BTreeMap;

/// A new picture for one tile, already decoded.
pub struct PictureWrite {
    pub id: u64,
    pub image: Image,
    pub filter: Filter,
}

/// What a write did, per tile.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct WriteReport {
    /// `(tile id, new state index)`.
    pub written: Vec<(u64, usize)>,
    /// `(tile id, reason)`.
    pub skipped: Vec<(u64, String)>,
    /// Indices into `sector_log` of the entries this write added.
    pub entries: Vec<usize>,
}

/// Multiplies `img`'s alpha by `original`'s alpha resampled (bilinear) to `img`'s size, so a
/// quad's shape mask survives an edit that dropped or changed the transparency.
pub fn keep_alpha(img: &mut Image, original: &Image) {
    let (w, h) = (img.w as usize, img.h as usize);
    let (ow, oh) = (original.w as usize, original.h as usize);
    if w == 0 || h == 0 || ow == 0 || oh == 0 {
        return;
    }
    let a = |x: usize, y: usize| original.rgba[(y * ow + x) * 4 + 3] as f64;
    for y in 0..h {
        let sy = ((y as f64 + 0.5) * oh as f64 / h as f64 - 0.5).clamp(0.0, (oh - 1) as f64);
        let (y0, fy) = (sy.floor() as usize, sy.fract());
        let y1 = (y0 + 1).min(oh - 1);
        for x in 0..w {
            let sx = ((x as f64 + 0.5) * ow as f64 / w as f64 - 0.5).clamp(0.0, (ow - 1) as f64);
            let (x0, fx) = (sx.floor() as usize, sx.fract());
            let x1 = (x0 + 1).min(ow - 1);
            let top = a(x0, y0) * (1.0 - fx) + a(x1, y0) * fx;
            let bottom = a(x0, y1) * (1.0 - fx) + a(x1, y1) * fx;
            let oa = top * (1.0 - fy) + bottom * fy;
            let p = &mut img.rgba[(y * w + x) * 4 + 3];
            *p = (*p as f64 * oa / 255.0).round() as u8;
        }
    }
}

/// One picture to paste as a new tile (spec atlas-history "Copy a state as a new object"): the
/// state, the unrotated frame size of its source, and for a sector entry its top-left relative to
/// the source sector, so the group keeps its layout.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct PasteTile {
    pub state: PictureState,
    pub w: i64,
    pub h: i64,
    #[serde(default)]
    pub offset: Option<[i64; 2]>,
    #[serde(default)]
    pub source: super::model::Provenance,
}

impl Manifest {
    /// Pastes pictures as new unrotated tiles into the active sector and returns their ids.
    /// Tiles with offsets (a sector entry) keep their layout: at the same offsets when that space
    /// is free, otherwise together at the first free spot; a single tile goes to the first free
    /// spot. Each new tile's history starts with the pasted picture.
    pub fn paste_states(&mut self, tiles: Vec<PasteTile>) -> std::result::Result<Vec<u64>, Refused> {
        if tiles.is_empty() {
            return Err(Refused("Nothing to paste".into()));
        }
        let size = self.sector_size;
        let pad = self.padding as i64;
        let sector = self.active.filter(|a| self.has_sector(*a)).or_else(|| self.sectors.first().copied()).ok_or_else(|| Refused("The workspace has no sector".into()))?;
        let cell = sector.rect(size);
        // Layout relative to the group's own top-left.
        let offs: Vec<[i64; 2]> = tiles.iter().scan(0i64, |x, t| {
            let o = t.offset.unwrap_or([*x, 0]);
            *x += t.w + pad;
            Some(o)
        }).collect();
        let (gx, gy) = (offs.iter().map(|o| o[0]).min().unwrap(), offs.iter().map(|o| o[1]).min().unwrap());
        let gw = tiles.iter().zip(&offs).map(|(t, o)| o[0] + t.w).max().unwrap() - gx;
        let gh = tiles.iter().zip(&offs).map(|(t, o)| o[1] + t.h).max().unwrap() - gy;
        let occupied: Vec<Rect> = self.tiles.iter().map(|t| t.rect()).filter(|r| r.intersects(&cell)).map(|r| r.inflate(pad)).collect();
        let rects_at = |x0: i64, y0: i64| -> Vec<Rect> { tiles.iter().zip(&offs).map(|(t, o)| Rect { x: x0 + o[0] - gx, y: y0 + o[1] - gy, w: t.w, h: t.h }).collect() };
        let keep = tiles.iter().all(|t| t.offset.is_some()) && {
            let rs = rects_at(cell.x + gx, cell.y + gy);
            rs.iter().all(|r| cell.contains(r) && !occupied.iter().any(|o| o.intersects(r)))
        };
        let rects = if keep {
            rects_at(cell.x + gx, cell.y + gy)
        } else {
            let (x, y) = super::place::find_spot(cell.inflate(-pad), &occupied, gw, gh)
                .ok_or_else(|| Refused(format!("There is no free space for {} x {} px in sector {}", gw, gh, sector_label(self, sector))))?;
            rects_at(x, y)
        };
        let mut ids = Vec::new();
        for (t, r) in tiles.into_iter().zip(rects) {
            let id = self.next_id;
            self.next_id += 1;
            let s = t.state;
            self.tiles.push(super::model::Tile {
                id,
                item: s.item.clone(),
                w0: s.w0,
                h0: s.h0,
                frame: super::model::Frame::from_rect(r),
                source: t.source,
                history: vec![s],
                current: 0,
            });
            ids.push(id);
        }
        Ok(ids)
    }

    /// The sector holding the centre of tile `id`, if it is in the grid.
    pub fn sector_of(&self, id: u64) -> Option<Sector> {
        let t = self.tile(id)?;
        let r = t.rect();
        let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
        self.sectors.iter().copied().find(|s| {
            let c = s.rect(self.sector_size);
            cx >= c.x && cx < c.right() && cy >= c.y && cy < c.bottom()
        })
    }

    /// Pushes stored pictures (`(tile id, item, w0, h0, filter)`) as new current states named
    /// `name` by `by`. With `sector_entries`, adds one sector-log entry per sector of the written
    /// tiles. Tiles that no longer exist are skipped.
    pub fn push_pictures(&mut self, items: &[(u64, String, u32, u32, Filter)], name: &str, by: &str, sector_entries: bool) -> WriteReport {
        let at = now();
        let mut report = WriteReport::default();
        let mut by_sector: BTreeMap<(i32, i32), Vec<SectorChange>> = BTreeMap::new();
        for (id, item, w0, h0, filter) in items {
            let sector = self.sector_of(*id);
            let Some(t) = self.tiles.iter_mut().find(|t| t.id == *id) else {
                report.skipped.push((*id, format!("tile {id} no longer exists")));
                continue;
            };
            t.ensure_history("RIP");
            let before = t.current;
            let after = t.push_state(PictureState { item: item.clone(), w0: *w0, h0: *h0, name: name.into(), by: by.into(), at: at.clone(), filter: *filter });
            report.written.push((*id, after));
            if let (true, Some(s)) = (sector_entries, sector) {
                by_sector.entry((s.u, s.v)).or_default().push(SectorChange { id: *id, before, after });
            }
        }
        for ((u, v), tiles) in by_sector {
            self.sector_log.push(SectorEntry { sector: Sector { u, v }, name: name.into(), by: by.into(), at: at.clone(), tiles });
            report.entries.push(self.sector_log.len() - 1);
        }
        report
    }

    /// Makes state `i` of tile `id` current.
    pub fn go_to_state(&mut self, id: u64, i: usize) -> std::result::Result<(), Refused> {
        let t = self.tiles.iter_mut().find(|t| t.id == id).ok_or_else(|| Refused(format!("Tile {id} no longer exists")))?;
        t.go_to(i)
    }

    /// Goes to a sector-log entry: each of its tiles that still exists takes the state the entry
    /// wrote, or with `before` the state it had before the entry. Returns the reasons for tiles
    /// that were skipped.
    pub fn go_to_entry(&mut self, entry: usize, before: bool) -> std::result::Result<Vec<String>, Refused> {
        let e = self.sector_log.get(entry).cloned().ok_or_else(|| Refused(format!("There is no history entry {}", entry + 1)))?;
        let mut skipped = Vec::new();
        for c in &e.tiles {
            let i = if before { c.before } else { c.after };
            match self.tiles.iter_mut().find(|t| t.id == c.id) {
                None => skipped.push(format!("tile {} no longer exists", c.id)),
                Some(t) => {
                    if let Err(Refused(r)) = t.go_to(i) {
                        skipped.push(r);
                    }
                }
            }
        }
        Ok(skipped)
    }
}

/// A tile in a sector render: where it is in the image, and whether a whole-sector write can
/// cut it back (upright or quarter-turned, wholly inside the sector).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TileInRender {
    pub id: u64,
    /// Bounding box in the render's pixels.
    pub rect: Rect,
    pub writable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Which tiles of `sector` a whole-sector render shows, and which of them can be written back.
pub fn sector_tiles(m: &Manifest, sector: Sector) -> Vec<TileInRender> {
    let cell = sector.rect(m.sector_size);
    m.tiles
        .iter()
        .filter(|t| t.rect().intersects(&cell))
        .map(|t| {
            let r = t.rect();
            let reason = if t.frame.tilt != 0.0 {
                Some(format!("tile {} is turned by {:.1} degrees", t.id, t.frame.tilt))
            } else if !cell.contains(&r) {
                Some(format!("tile {} crosses the sector edge", t.id))
            } else {
                None
            };
            TileInRender { id: t.id, rect: Rect { x: r.x - cell.x, y: r.y - cell.y, w: r.w, h: r.h }, writable: reason.is_none(), reason }
        })
        .collect()
}

/// Cuts a sector render back into pictures for its writable tiles: each tile's box, turned back
/// by its quarter turns, at frame size. Returns the pictures and the refused tiles.
/// Pictures cut from a sector render, and the refused tiles with their reasons.
pub type Cut = (Vec<(u64, Image)>, Vec<(u64, String)>);

pub fn cut_sector(m: &Manifest, sector: Sector, img: &Image) -> std::result::Result<Cut, Refused> {
    let size = m.sector_size;
    if (img.w, img.h) != (size, size) {
        return Err(Refused(format!("The picture is {} x {}; a sector render is {size} x {size}", img.w, img.h)));
    }
    let mut out = Vec::new();
    let mut refused = Vec::new();
    for t in sector_tiles(m, sector) {
        if let Some(r) = t.reason {
            refused.push((t.id, r));
            continue;
        }
        let rot = m.tile(t.id).map(|x| x.frame.rot % 4).unwrap_or(0);
        let r = t.rect;
        let mut crop = Image::transparent(r.w as u32, r.h as u32);
        for y in 0..r.h as usize {
            let s = ((r.y as usize + y) * size as usize + r.x as usize) * 4;
            crop.rgba[y * r.w as usize * 4..(y + 1) * r.w as usize * 4].copy_from_slice(&img.rgba[s..s + r.w as usize * 4]);
        }
        out.push((t.id, render::rotate_exact(&crop, (4 - rot) % 4)));
    }
    Ok((out, refused))
}

impl Workspace {
    /// Renders `sector` at full size with a transparent fill (an editing copy, not an export).
    pub fn render_sector(&self, sector: Sector) -> Result<(Image, Vec<TileInRender>)> {
        let mut m = self.read()?;
        if !m.has_sector(sector) {
            return Err(Refused(format!("There is no sector {}", sector_label(&m, sector))).into());
        }
        m.fill = Fill::Transparent;
        let cell = sector.rect(m.sector_size);
        let img = render::render_area(&m, (cell.x, cell.y), m.sector_size, m.sector_size, &[cell], &mut |t| super::files::load_original(self, t))
            .map_err(super::store::Error::Refused)?;
        Ok((img, sector_tiles(&m, sector)))
    }

    /// Writes a whole-sector picture back (spec atlas-mcp "Sector edits per tile or whole").
    pub fn write_sector(&self, sector: Sector, img: &Image, name: &str, by: &str, keep_alpha_: bool, filter: Filter) -> Result<(Manifest, WriteReport)> {
        let m = self.read()?;
        let (pictures, refused) = cut_sector(&m, sector, img)?;
        let writes = pictures.into_iter().map(|(id, image)| PictureWrite { id, image, filter }).collect();
        let (m, mut report) = self.write_pictures(writes, name, by, keep_alpha_, true)?;
        report.skipped.extend(refused);
        Ok((m, report))
    }

    /// Writes new pictures for tiles (spec atlas-mcp "Write a tile picture"): with
    /// `keep_alpha`, each picture's alpha is multiplied by its tile's original alpha; the PNGs
    /// are stored as items, then pushed as states under one lock. Items of skipped tiles are
    /// removed again.
    pub fn write_pictures(&self, writes: Vec<PictureWrite>, name: &str, by: &str, keep_alpha_: bool, sector_entries: bool) -> Result<(Manifest, WriteReport)> {
        let m = self.read()?;
        let mut stored = Vec::new();
        let mut early: Vec<(u64, String)> = Vec::new();
        for mut w in writes {
            let Some(t) = m.tile(w.id) else {
                early.push((w.id, format!("tile {} no longer exists", w.id)));
                continue;
            };
            if keep_alpha_ {
                match super::files::load_item(self, t.original_item()) {
                    Ok(o) => keep_alpha(&mut w.image, &o),
                    Err(e) => {
                        early.push((w.id, format!("cannot read the original of tile {}: {e}", w.id)));
                        continue;
                    }
                }
            }
            let item = self.add_item(&super::files::encode_png(&w.image))?;
            stored.push((w.id, item, w.image.w, w.image.h, w.filter));
        }
        let r = self.mutate(|m| Ok(m.push_pictures(&stored, name, by, sector_entries)));
        match r {
            Ok((m, mut report)) => {
                for (id, _) in &report.skipped {
                    if let Some((_, item, ..)) = stored.iter().find(|s| s.0 == *id) {
                        let _ = std::fs::remove_file(self.item_path(item));
                    }
                }
                report.skipped.extend(early);
                Ok((m, report))
            }
            Err(e) => {
                for (_, item, ..) in &stored {
                    let _ = std::fs::remove_file(self.item_path(item));
                }
                Err(e)
            }
        }
    }
}

/// UDIM-style label of a sector, counted from the grid's bottom-left cell.
pub fn sector_label(m: &Manifest, s: Sector) -> u32 {
    let u0 = m.sectors.iter().map(|c| c.u).min().unwrap_or(0);
    let v0 = m.sectors.iter().map(|c| c.v).min().unwrap_or(0);
    super::model::udim(s.u - u0, s.v - v0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::store::tests::temp_store;

    fn img(w: u32, h: u32, rgba: [u8; 4]) -> Image {
        Image { w, h, rgba: rgba.iter().copied().cycle().take((w * h * 4) as usize).collect() }
    }

    /// A 512 x 512 picture with a transparent outside of an inscribed circle.
    fn circle(n: u32) -> Image {
        let mut o = img(n, n, [200, 40, 40, 255]);
        let r = n as f64 / 2.0;
        for y in 0..n {
            for x in 0..n {
                if ((x as f64 + 0.5 - r).powi(2) + (y as f64 + 0.5 - r).powi(2)).sqrt() > r {
                    o.rgba[((y * n + x) * 4 + 3) as usize] = 0;
                }
            }
        }
        o
    }

    fn png(i: &Image) -> Vec<u8> {
        crate::atlas::files::encode_png(i)
    }

    #[test]
    fn pixelized_circle_keeps_the_circle() {
        let store = temp_store("hist-alpha");
        let ws = store.create().unwrap();
        let (_, p) = ws.add_tile(&png(&circle(512)), 512, 512, Default::default()).unwrap();
        let id = ws.read().unwrap().tiles[0].id;
        let _ = p;
        let (m, rep) = ws
            .write_pictures(vec![PictureWrite { id, image: img(64, 64, [10, 200, 10, 255]), filter: Filter::Nearest }], "Pixelize 64", "Claude Code", true, false)
            .unwrap();
        assert_eq!(rep.written, vec![(id, 1)]);
        let t = m.tile(id).unwrap();
        assert_eq!((t.w0, t.h0, t.current, t.history.len()), (64, 64, 1, 2));
        assert_eq!(t.history[1].name, "Pixelize 64");
        assert_eq!(t.filter(), Filter::Nearest);
        let cur = crate::atlas::files::load_item(&ws, &t.item).unwrap();
        let alpha = |x: u32, y: u32| cur.rgba[((y * 64 + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0, "corner outside the circle");
        assert_eq!(alpha(32, 32), 255, "centre");
        assert!(alpha(32, 0) > 0 && alpha(32, 0) < 255 || alpha(32, 0) == 255, "top edge is inside or partial");
        // The frame did not change.
        assert_eq!(t.frame.rect().w, 512);
    }

    #[test]
    fn without_keep_alpha_the_picture_keeps_its_own() {
        let store = temp_store("hist-own");
        let ws = store.create().unwrap();
        ws.add_tile(&png(&circle(128)), 128, 128, Default::default()).unwrap();
        let id = ws.read().unwrap().tiles[0].id;
        let (m, _) = ws.write_pictures(vec![PictureWrite { id, image: img(32, 32, [1, 2, 3, 255]), filter: Filter::Smooth }], "Opaque", "Codex", false, false).unwrap();
        let cur = crate::atlas::files::load_item(&ws, &m.tile(id).unwrap().item).unwrap();
        assert!(cur.rgba.chunks(4).all(|p| p[3] == 255));
    }

    #[test]
    fn going_down_keeps_the_states_above() {
        let store = temp_store("hist-goto");
        let ws = store.create().unwrap();
        ws.add_tile(&png(&img(16, 16, [9, 9, 9, 255])), 16, 16, Default::default()).unwrap();
        let id = ws.read().unwrap().tiles[0].id;
        for name in ["Restyle", "Pixelize"] {
            ws.write_pictures(vec![PictureWrite { id, image: img(8, 8, [1, 1, 1, 255]), filter: Filter::Smooth }], name, "Gemini", false, false).unwrap();
        }
        let (m, ()) = ws.mutate(|m| m.go_to_state(id, 0)).unwrap();
        let t = m.tile(id).unwrap();
        assert_eq!((t.current, t.history.len(), t.w0), (0, 3, 16));
        assert_eq!(t.item, t.history[0].item);
        // A later write goes on top.
        let (m, rep) = ws.write_pictures(vec![PictureWrite { id, image: img(4, 4, [1, 1, 1, 255]), filter: Filter::Smooth }], "Again", "Gemini", false, false).unwrap();
        assert_eq!(rep.written, vec![(id, 3)]);
        assert_eq!(m.tile(id).unwrap().history.len(), 4);
    }

    #[test]
    fn sector_entry_and_deleted_tile() {
        let store = temp_store("hist-sector");
        let ws = store.create().unwrap();
        for _ in 0..3 {
            ws.add_tile(&png(&img(64, 64, [9, 9, 9, 255])), 64, 64, Default::default()).unwrap();
        }
        let ids: Vec<u64> = ws.read().unwrap().tiles.iter().map(|t| t.id).collect();
        let writes = ids.iter().map(|&id| PictureWrite { id, image: img(8, 8, [1, 1, 1, 255]), filter: Filter::Nearest }).collect();
        let (m, rep) = ws.write_pictures(writes, "Pixelize 1001", "Claude Code", false, true).unwrap();
        assert_eq!(rep.entries, vec![0]);
        assert_eq!(m.sector_log[0].tiles.len(), 3);
        assert_eq!(m.sector_log[0].sector, Sector { u: 0, v: 0 });
        // Delete one tile, then go to "before" the entry: two tiles go back, one is named.
        ws.edit(crate::atlas::edit::Edit::Delete { ids: vec![ids[1]] }).unwrap();
        let (m, skipped) = ws.mutate(|m| m.go_to_entry(0, true)).unwrap();
        assert_eq!(skipped, vec![format!("tile {} no longer exists", ids[1])]);
        assert!(m.tiles.iter().all(|t| t.current == 0 && t.w0 == 64));
    }

    #[test]
    fn older_formats_start_with_the_original() {
        let v2 = r#"{"format":2,"revision":4,"name":"x","sector_size":1024,"padding":8,"sectors":[{"u":0,"v":0}],"active":null,
            "tiles":[{"id":1,"item":"a.png","w0":40,"h0":20,"c2":[40,-1984],"w":40,"h":20,"rot":0}],"next_id":2}"#;
        let v1 = r#"{"format":1,"revision":4,"name":"x","sector_size":1024,"padding":8,"sectors":[{"u":0,"v":0}],"active":null,
            "tiles":[{"id":1,"item":"a.png","w0":40,"h0":20,"x":0,"y":-1000,"w":40,"h":20,"rot":0}],"next_id":2}"#;
        for text in [v1, v2] {
            let m = crate::atlas::model::parse_manifest(text).unwrap();
            let t = &m.tiles[0];
            assert_eq!(m.format, crate::atlas::model::FORMAT);
            assert_eq!((t.history.len(), t.current, t.history[0].name.as_str(), t.history[0].item.as_str()), (1, 0, "Original", "a.png"));
        }
    }

    #[test]
    fn save_as_keeps_every_state_and_drops_the_rest() {
        let store = temp_store("hist-save");
        let ws = store.create().unwrap();
        ws.add_tile(&png(&img(16, 16, [9, 9, 9, 255])), 16, 16, Default::default()).unwrap();
        let id = ws.read().unwrap().tiles[0].id;
        ws.write_pictures(vec![PictureWrite { id, image: img(8, 8, [1, 1, 1, 255]), filter: Filter::Smooth }], "A", "Codex", false, false).unwrap();
        ws.mutate(|m| m.go_to_state(id, 0)).unwrap();
        let orphan = ws.add_item(&png(&img(2, 2, [0, 0, 0, 255]))).unwrap();
        let file = store.root.join("keep.ripper");
        store.save_as(&ws, &file).unwrap();
        let m = ws.read().unwrap();
        let t = m.tile(id).unwrap();
        assert_eq!(t.current, 0);
        for s in &t.history {
            assert!(ws.item_path(&s.item).is_file(), "state item kept");
        }
        assert!(!ws.item_path(&orphan).is_file(), "orphan dropped");
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&file).unwrap()).unwrap();
        for s in &t.history {
            assert!(zip.by_name(&format!("items/{}", s.item)).is_ok(), "state item packed");
        }
        // Reopening the file brings the same history.
        let back = store.open(&file).unwrap().read().unwrap();
        assert_eq!(back.tile(id).unwrap().history, t.history);
    }

    #[test]
    fn undo_of_a_delete_restores_the_history() {
        let store = temp_store("hist-undo");
        let ws = store.create().unwrap();
        ws.add_tile(&png(&img(16, 16, [9, 9, 9, 255])), 16, 16, Default::default()).unwrap();
        let id = ws.read().unwrap().tiles[0].id;
        ws.write_pictures(vec![PictureWrite { id, image: img(8, 8, [1, 1, 1, 255]), filter: Filter::Smooth }], "A", "Codex", false, false).unwrap();
        let before = ws.read().unwrap().tile(id).unwrap().clone();
        let (_, applied) = ws.edit(crate::atlas::edit::Edit::Delete { ids: vec![id] }).unwrap();
        assert_eq!(applied.released.len(), 2, "both states' items are released together");
        ws.apply_steps(applied.inverse).unwrap();
        assert_eq!(ws.read().unwrap().tile(id).unwrap(), &before);
    }

    /// Noise so a wrong crop or turn cannot pass by accident.
    fn noise(w: u32, h: u32, seed: u32) -> Image {
        let mut x = seed.wrapping_mul(2654435761).max(1);
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            rgba.extend_from_slice(&[x as u8, (x >> 8) as u8, (x >> 16) as u8, 255]);
        }
        Image { w, h, rgba }
    }

    #[test]
    fn whole_sector_round_trip() {
        use crate::atlas::edit::Edit;
        use crate::atlas::model::{Frame, Rect};
        let store = temp_store("hist-sector-whole");
        let ws = store.create().unwrap();
        ws.edit(Edit::SetSize { size: 512 }).unwrap();
        let originals = [noise(100, 60, 1), noise(80, 80, 2), noise(50, 120, 3), noise(40, 40, 4), noise(60, 60, 5)];
        for o in &originals {
            ws.add_tile(&png(o), o.w, o.h, Default::default()).unwrap();
        }
        let ids: Vec<u64> = ws.read().unwrap().tiles.iter().map(|t| t.id).collect();
        // Sector (0,0) spans x 0..512, y -512..0. Three upright tiles (one quarter-turned),
        // one tilted, one across the right edge.
        let place = |x: i64, y: i64, w: i64, h: i64, rot: u8, tilt: f64| {
            let mut f = Frame::from_rect(Rect { x, y, w, h });
            f.rot = rot;
            f.tilt = tilt;
            f
        };
        let frames = [
            place(10, -500, 100, 60, 0, 0.0),
            place(200, -500, 80, 80, 0, 0.0),
            place(300, -300, 50, 120, 1, 0.0),
            place(60, -200, 40, 40, 0, 30.0),
            place(480, -100, 60, 60, 0, 0.0),
        ];
        for (id, f) in ids.iter().zip(frames) {
            ws.edit(Edit::SetFrame { id: *id, frame: f }).unwrap();
        }
        let sector = Sector { u: 0, v: 0 };
        let (img, tiles) = ws.render_sector(sector).unwrap();
        assert_eq!((img.w, img.h), (512, 512));
        assert_eq!(tiles.iter().filter(|t| t.writable).count(), 3);
        let before = ws.read().unwrap();
        let (m, rep) = ws.write_sector(sector, &img, "Same", "Claude Code", true, Filter::Smooth).unwrap();
        let mut written: Vec<u64> = rep.written.iter().map(|w| w.0).collect();
        written.sort();
        assert_eq!(written, ids[..3].to_vec());
        let skipped: Vec<u64> = rep.skipped.iter().map(|s| s.0).collect();
        assert_eq!(skipped, vec![ids[3], ids[4]]);
        assert!(rep.skipped[0].1.contains("turned") && rep.skipped[1].1.contains("edge"));
        // Each written tile's new picture is its frame-size render of the original.
        for (k, id) in ids[..3].iter().enumerate() {
            let t = m.tile(*id).unwrap();
            let pic = crate::atlas::files::load_item(&ws, &t.item).unwrap();
            let f = &t.frame;
            assert_eq!((pic.w as i64, pic.h as i64), (f.w, f.h));
            let want = render::render_tile(&originals[k], 0, f.w as u32, f.h as u32);
            assert_eq!(pic, want, "tile {id}");
        }
        for id in &ids[3..] {
            assert_eq!(m.tile(*id).unwrap(), before.tile(*id).unwrap(), "untouched");
        }
        assert_eq!(m.sector_log.len(), 1);
        assert_eq!(m.sector_log[0].tiles.len(), 3);
        // A wrong-size picture is refused.
        assert!(ws.write_sector(sector, &noise(10, 10, 9), "Bad", "x", true, Filter::Smooth).is_err());
    }

    #[test]
    fn paste_a_state_and_a_sector_entry() {
        use crate::atlas::edit::Edit;
        use crate::atlas::model::Side;
        let store = temp_store("hist-paste");
        let ws = store.create().unwrap();
        ws.edit(Edit::SetSize { size: 512 }).unwrap();
        ws.add_tile(&png(&img(100, 50, [9, 9, 9, 255])), 100, 50, Default::default()).unwrap();
        ws.add_tile(&png(&img(60, 60, [9, 9, 9, 255])), 60, 60, Default::default()).unwrap();
        let ids: Vec<u64> = ws.read().unwrap().tiles.iter().map(|t| t.id).collect();
        let writes = ids.iter().map(|&id| PictureWrite { id, image: img(10, 5, [1, 1, 1, 255]), filter: Filter::Nearest }).collect();
        ws.write_pictures(writes, "Pixelize 1001", "Claude Code", false, true).unwrap();
        let m = ws.read().unwrap();

        // A single state: unrotated, at the source frame size, history of one.
        let t0 = m.tile(ids[0]).unwrap();
        let one = PasteTile { state: t0.history[0].clone(), w: t0.frame.w, h: t0.frame.h, offset: None, source: t0.source.clone() };
        let (m, a) = ws.edit(Edit::PasteStates { tiles: vec![one] }).unwrap();
        let new = m.tiles.last().unwrap();
        assert_eq!((new.history.len(), new.current, new.item.as_str(), new.frame.w, new.frame.rot), (1, 0, t0.history[0].item.as_str(), 100, 0));
        assert!(m.sector_of(new.id).is_some(), "placed in a sector");
        assert!(!m.tiles[..m.tiles.len() - 1].iter().any(|t| t.rect().intersects(&new.rect())), "in free space");
        // Undo is a delete of the pasted tile.
        ws.apply_steps(a.inverse).unwrap();
        assert_eq!(ws.read().unwrap().tiles.len(), 2);

        // A sector entry into an empty sector keeps the layout at the same offsets.
        ws.edit(Edit::AddSector { sector: Sector { u: 0, v: 0 }, side: Side::Right }).unwrap();
        ws.edit(Edit::SetActive { sector: Sector { u: 1, v: 0 } }).unwrap();
        let m = ws.read().unwrap();
        let cell = Sector { u: 0, v: 0 }.rect(512);
        let group: Vec<PasteTile> = m.sector_log[0]
            .tiles
            .iter()
            .map(|c| {
                let t = m.tile(c.id).unwrap();
                let r = t.rect();
                PasteTile { state: t.history[c.after].clone(), w: t.frame.w, h: t.frame.h, offset: Some([r.x - cell.x, r.y - cell.y]), source: t.source.clone() }
            })
            .collect();
        let (m2, _) = ws.edit(Edit::PasteStates { tiles: group.clone() }).unwrap();
        let pasted = &m2.tiles[2..];
        let target = Sector { u: 1, v: 0 }.rect(512);
        for (p, g) in pasted.iter().zip(&group) {
            let r = p.rect();
            assert_eq!([r.x - target.x, r.y - target.y], g.offset.unwrap(), "same offsets");
            assert_eq!(p.history[0].name, "Pixelize 1001");
        }
        // Pasting again: the space is taken, so the group moves together, keeping its layout.
        let (m3, _) = ws.edit(Edit::PasteStates { tiles: group.clone() }).unwrap();
        let again = &m3.tiles[4..];
        let (a0, a1) = (again[0].rect(), again[1].rect());
        let (g0, g1) = (group[0].offset.unwrap(), group[1].offset.unwrap());
        assert_eq!([a1.x - a0.x, a1.y - a0.y], [g1[0] - g0[0], g1[1] - g0[1]]);
        assert!(!m3.tiles[..4].iter().any(|t| t.rect().intersects(&a0) || t.rect().intersects(&a1)));
    }

    #[test]
    fn capture_during_a_write_loses_nothing() {
        let store = temp_store("hist-concurrent");
        let ws = store.create().unwrap();
        ws.add_tile(&png(&img(32, 32, [9, 9, 9, 255])), 32, 32, Default::default()).unwrap();
        let id = ws.read().unwrap().tiles[0].id;
        let ws2 = store.workspace(&ws.id);
        let pic = png(&img(32, 32, [5, 5, 5, 255]));
        let t = std::thread::spawn(move || {
            for _ in 0..5 {
                ws2.add_tile(&pic, 32, 32, Default::default()).unwrap();
            }
        });
        for _ in 0..5 {
            ws.write_pictures(vec![PictureWrite { id, image: img(16, 16, [1, 1, 1, 255]), filter: Filter::Smooth }], "Edit", "Codex", false, false).unwrap();
        }
        t.join().unwrap();
        let m = ws.read().unwrap();
        assert_eq!(m.tiles.len(), 6);
        assert_eq!(m.tile(id).unwrap().history.len(), 6);
    }
}
