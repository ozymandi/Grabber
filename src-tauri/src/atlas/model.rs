//! Atlas workspace model (design D2 of add-texture-atlas): the manifest, canvas
//! coordinates, the sector grid and pure tile/sector operations. No I/O here.
//!
//! Canvas space is pixels, x right and y down (as on screen). Sector `(u, v)` covers
//! `x in [u*S, (u+1)*S)` and `y in [-(v+1)*S, -v*S)`, so `v` grows upward like UDIM rows.

use serde::{Deserialize, Serialize};

pub const FORMAT: u32 = 1;
pub const SECTOR_SIZES: [u32; 5] = [512, 1024, 2048, 4096, 8192];
pub const DEFAULT_SECTOR: u32 = 2048;
pub const DEFAULT_PADDING: u32 = 8;

/// Axis-aligned integer rectangle in canvas pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

impl Rect {
    pub fn right(&self) -> i64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> i64 {
        self.y + self.h
    }
    /// True when the two rectangles share a positive area.
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
    pub fn contains(&self, o: &Rect) -> bool {
        o.x >= self.x && o.y >= self.y && o.right() <= self.right() && o.bottom() <= self.bottom()
    }
    pub fn inflate(&self, d: i64) -> Rect {
        Rect { x: self.x - d, y: self.y - d, w: self.w + 2 * d, h: self.h + 2 * d }
    }
    pub fn intersection(&self, o: &Rect) -> Option<Rect> {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = self.right().min(o.right());
        let y1 = self.bottom().min(o.bottom());
        (x1 > x0 && y1 > y0).then(|| Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
    }
}

/// A grid cell, in UDIM-like column `u` and row `v` (row 0 is the bottom row).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sector {
    pub u: i32,
    pub v: i32,
}

impl Sector {
    pub fn rect(&self, size: u32) -> Rect {
        let s = size as i64;
        Rect { x: self.u as i64 * s, y: -(self.v as i64 + 1) * s, w: s, h: s }
    }
    pub fn neighbour(&self, side: Side) -> Sector {
        match side {
            Side::Left => Sector { u: self.u - 1, v: self.v },
            Side::Right => Sector { u: self.u + 1, v: self.v },
            Side::Up => Sector { u: self.u, v: self.v + 1 },
            Side::Down => Sector { u: self.u, v: self.v - 1 },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left,
    Right,
    Up,
    Down,
}

/// UDIM number of a cell counted from a group's bottom-left cell.
pub fn udim(col: i32, row: i32) -> u32 {
    (1001 + col + 10 * row) as u32
}

/// Where a tile came from, for status lines and export names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Provenance {
    pub stem: String,
    pub quad: u32,
}

/// A placed texture. `w0 x h0` is the original; `x, y, w, h` the placed, already-rotated
/// rectangle on the canvas; `rot` counts quarter turns clockwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub id: u64,
    pub item: String,
    pub w0: u32,
    pub h0: u32,
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
    pub rot: u8,
    #[serde(default)]
    pub source: Provenance,
}

impl Tile {
    pub fn rect(&self) -> Rect {
        Rect { x: self.x, y: self.y, w: self.w, h: self.h }
    }
    /// Original size as seen after rotation (width, height).
    pub fn rotated_original(&self) -> (i64, i64) {
        if self.rot % 2 == 1 {
            (self.h0 as i64, self.w0 as i64)
        } else {
            (self.w0 as i64, self.h0 as i64)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub revision: u64,
    pub name: String,
    pub sector_size: u32,
    pub padding: u32,
    pub sectors: Vec<Sector>,
    pub active: Option<Sector>,
    pub tiles: Vec<Tile>,
    pub next_id: u64,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            format: FORMAT,
            revision: 0,
            name: "Untitled".into(),
            sector_size: DEFAULT_SECTOR,
            padding: DEFAULT_PADDING,
            sectors: vec![Sector { u: 0, v: 0 }],
            active: None,
            tiles: Vec::new(),
            next_id: 1,
        }
    }
}

/// Why an operation was refused; the text goes to the status row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused(pub String);

fn refuse<T>(msg: impl Into<String>) -> Result<T, Refused> {
    Err(Refused(msg.into()))
}

/// Corner of a tile, used as the fixed anchor while scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

impl Manifest {
    pub fn has_sector(&self, s: Sector) -> bool {
        self.sectors.contains(&s)
    }

    /// Sectors in UDIM order: bottom row first, then left to right.
    pub fn sectors_udim_order(&self) -> Vec<Sector> {
        let mut v = self.sectors.clone();
        v.sort_by_key(|s| (s.v, s.u));
        v
    }

    /// Bounding rectangle of all sectors.
    pub fn grid_bounds(&self) -> Rect {
        let s = self.sector_size;
        let mut it = self.sectors.iter().map(|c| c.rect(s));
        let first = it.next().unwrap_or(Sector { u: 0, v: 0 }.rect(s));
        it.fold(first, |a, r| {
            let x0 = a.x.min(r.x);
            let y0 = a.y.min(r.y);
            let x1 = a.right().max(r.right());
            let y1 = a.bottom().max(r.bottom());
            Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
        })
    }

    pub fn tile(&self, id: u64) -> Option<&Tile> {
        self.tiles.iter().find(|t| t.id == id)
    }

    fn tile_mut(&mut self, id: u64) -> Result<&mut Tile, Refused> {
        match self.tiles.iter_mut().find(|t| t.id == id) {
            Some(t) => Ok(t),
            None => refuse(format!("Tile {id} no longer exists")),
        }
    }

    /// Sides of `s` that have no neighbour, so a "+" can be shown there.
    pub fn free_sides(&self, s: Sector) -> Vec<Side> {
        [Side::Left, Side::Right, Side::Up, Side::Down]
            .into_iter()
            .filter(|&d| !self.has_sector(s.neighbour(d)))
            .collect()
    }

    pub fn add_sector(&mut self, from: Sector, side: Side) -> Result<Sector, Refused> {
        if !self.has_sector(from) {
            return refuse("That sector no longer exists");
        }
        let n = from.neighbour(side);
        if self.has_sector(n) {
            return refuse("There is already a sector on that side");
        }
        self.sectors.push(n);
        Ok(n)
    }

    /// Tiles overlapping a sector, in manifest order.
    pub fn tiles_on(&self, s: Sector) -> impl Iterator<Item = &Tile> {
        let r = s.rect(self.sector_size);
        self.tiles.iter().filter(move |t| t.rect().intersects(&r))
    }

    pub fn remove_sector(&mut self, s: Sector) -> Result<(), Refused> {
        if !self.has_sector(s) {
            return refuse("That sector no longer exists");
        }
        if self.sectors.len() == 1 {
            return refuse("A workspace keeps at least one sector");
        }
        if self.tiles_on(s).next().is_some() {
            return refuse("Move or delete the tiles on this sector before removing it");
        }
        self.sectors.retain(|&c| c != s);
        if self.active == Some(s) {
            self.active = None;
        }
        Ok(())
    }

    pub fn set_active(&mut self, s: Sector) -> Result<(), Refused> {
        if !self.has_sector(s) {
            return refuse("That sector no longer exists");
        }
        self.active = Some(s);
        Ok(())
    }

    pub fn set_sector_size(&mut self, size: u32) -> Result<(), Refused> {
        if !SECTOR_SIZES.contains(&size) {
            return refuse(format!("Sector size must be one of {SECTOR_SIZES:?}"));
        }
        if !self.tiles.is_empty() {
            return refuse("The sector size can only change while the workspace has no tiles");
        }
        self.sector_size = size;
        Ok(())
    }

    /// Changes the padding for future automatic placement; tiles do not move.
    pub fn set_padding(&mut self, padding: u32) -> Result<(), Refused> {
        if padding > self.sector_size / 4 {
            return refuse(format!("Padding can be at most {} px", self.sector_size / 4));
        }
        self.padding = padding;
        Ok(())
    }

    /// Moves each tile to a new top-left position.
    pub fn move_tiles(&mut self, moves: &[(u64, i64, i64)]) -> Result<(), Refused> {
        for &(id, x, y) in moves {
            let t = self.tile_mut(id)?;
            t.x = x;
            t.y = y;
        }
        Ok(())
    }

    /// Scales a tile uniformly to `new_w` pixels wide, keeping `anchor` fixed. The height
    /// follows the original's aspect ratio; both sides are at least 1 pixel.
    pub fn scale_tile(&mut self, id: u64, new_w: i64, anchor: Corner) -> Result<(), Refused> {
        let t = self.tile_mut(id)?;
        let (ow, oh) = t.rotated_original();
        let w = new_w.max(1);
        let h = ((w as f64) * (oh as f64) / (ow as f64)).round().max(1.0) as i64;
        let (ax, ay) = match anchor {
            Corner::TopLeft => (t.x, t.y),
            Corner::TopRight => (t.x + t.w, t.y),
            Corner::BottomRight => (t.x + t.w, t.y + t.h),
            Corner::BottomLeft => (t.x, t.y + t.h),
        };
        let (x, y) = match anchor {
            Corner::TopLeft => (ax, ay),
            Corner::TopRight => (ax - w, ay),
            Corner::BottomRight => (ax - w, ay - h),
            Corner::BottomLeft => (ax, ay - h),
        };
        *t = Tile { x, y, w, h, ..t.clone() };
        Ok(())
    }

    /// Rotates tiles a quarter turn about their own centres. Integer division truncates
    /// toward zero, so four turns return exactly to the starting rectangle.
    pub fn rotate_tiles(&mut self, ids: &[u64], clockwise: bool) -> Result<(), Refused> {
        for &id in ids {
            let t = self.tile_mut(id)?;
            let (w, h) = (t.w, t.h);
            t.x += (w - h) / 2;
            t.y += (h - w) / 2;
            t.w = h;
            t.h = w;
            t.rot = if clockwise { (t.rot + 1) % 4 } else { (t.rot + 3) % 4 };
        }
        Ok(())
    }

    /// Removes tiles; returns how many were removed. Items stay on disk until the next save.
    pub fn delete_tiles(&mut self, ids: &[u64]) -> usize {
        let before = self.tiles.len();
        self.tiles.retain(|t| !ids.contains(&t.id));
        before - self.tiles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(u: i32, v: i32) -> Sector {
        Sector { u, v }
    }

    #[test]
    fn sector_rects_follow_udim_rows_upward() {
        let size = 1024;
        assert_eq!(s(0, 0).rect(size), Rect { x: 0, y: -1024, w: 1024, h: 1024 });
        assert_eq!(s(1, 0).rect(size), Rect { x: 1024, y: -1024, w: 1024, h: 1024 });
        // Row 1 sits above row 0: smaller y on screen.
        assert_eq!(s(0, 1).rect(size), Rect { x: 0, y: -2048, w: 1024, h: 1024 });
        assert_eq!(s(-1, 0).rect(size), Rect { x: -1024, y: -1024, w: 1024, h: 1024 });
        // Neighbours share edges exactly, no gaps.
        assert_eq!(s(0, 0).rect(size).right(), s(1, 0).rect(size).x);
        assert_eq!(s(0, 1).rect(size).bottom(), s(0, 0).rect(size).y);
    }

    #[test]
    fn udim_numbers() {
        assert_eq!(udim(0, 0), 1001);
        assert_eq!(udim(1, 0), 1002);
        assert_eq!(udim(0, 1), 1011);
        assert_eq!(udim(9, 2), 1030);
    }

    #[test]
    fn default_workspace() {
        let m = Manifest::default();
        assert_eq!(m.sectors, vec![s(0, 0)]);
        assert_eq!(m.sector_size, 2048);
        assert_eq!(m.padding, 8);
        assert_eq!(m.name, "Untitled");
    }

    fn tile(id: u64, x: i64, y: i64, w: i64, h: i64) -> Tile {
        Tile { id, item: format!("{id}.png"), w0: w as u32, h0: h as u32, x, y, w, h, rot: 0, source: Provenance::default() }
    }

    #[test]
    fn add_sector_to_the_right_and_free_sides() {
        let mut m = Manifest::default();
        assert_eq!(m.add_sector(s(0, 0), Side::Right), Ok(s(1, 0)));
        assert!(!m.free_sides(s(0, 0)).contains(&Side::Right));
        assert!(m.free_sides(s(1, 0)).contains(&Side::Right));
        assert!(m.add_sector(s(0, 0), Side::Right).is_err());
    }

    #[test]
    fn remove_sector_rules() {
        let mut m = Manifest::default();
        m.sector_size = 1024;
        m.add_sector(s(0, 0), Side::Right).unwrap();
        // A tile across the border blocks both sectors.
        m.tiles.push(tile(1, 1000, -500, 100, 100));
        assert!(m.remove_sector(s(1, 0)).is_err());
        m.tiles.clear();
        m.set_active(s(1, 0)).unwrap();
        assert_eq!(m.remove_sector(s(1, 0)), Ok(()));
        assert_eq!(m.active, None);
        assert!(m.free_sides(s(0, 0)).contains(&Side::Right));
        assert!(m.remove_sector(s(0, 0)).is_err(), "last sector stays");
    }

    #[test]
    fn size_locked_after_first_tile() {
        let mut m = Manifest::default();
        assert_eq!(m.set_sector_size(1024), Ok(()));
        assert!(m.set_sector_size(1000).is_err());
        m.tiles.push(tile(1, 0, -100, 10, 10));
        assert!(m.set_sector_size(512).is_err());
        assert_eq!(m.sector_size, 1024);
    }

    #[test]
    fn padding_change_keeps_tiles() {
        let mut m = Manifest::default();
        m.tiles.push(tile(1, 8, -500, 10, 10));
        m.set_padding(16).unwrap();
        assert_eq!(m.tiles[0].rect(), Rect { x: 8, y: -500, w: 10, h: 10 });
    }

    #[test]
    fn scale_with_corner_handle_keeps_opposite_corner() {
        let mut m = Manifest::default();
        m.tiles.push(tile(1, 100, -900, 512, 256));
        // Dragging the bottom-right handle keeps the top-left corner.
        m.scale_tile(1, 1024, Corner::TopLeft).unwrap();
        assert_eq!(m.tiles[0].rect(), Rect { x: 100, y: -900, w: 1024, h: 512 });
        m.scale_tile(1, 0, Corner::BottomRight).unwrap();
        let r = m.tiles[0].rect();
        assert_eq!((r.w, r.h), (1, 1));
        assert_eq!((r.right(), r.bottom()), (100 + 1024, -900 + 512));
    }

    #[test]
    fn rotate_swaps_sides_about_centre_and_four_turns_restore() {
        let mut m = Manifest::default();
        m.tiles.push(tile(1, 0, -300, 400, 200));
        let start = m.tiles[0].clone();
        m.rotate_tiles(&[1], true).unwrap();
        let t = &m.tiles[0];
        assert_eq!((t.w, t.h, t.rot), (200, 400, 1));
        assert_eq!((t.x + t.w / 2, t.y + t.h / 2), (200, -200), "centre kept");
        for _ in 0..3 {
            m.rotate_tiles(&[1], true).unwrap();
        }
        assert_eq!(m.tiles[0], start);
        // Odd side difference, counter-clockwise too.
        let mut m = Manifest::default();
        m.tiles.push(tile(2, 7, -33, 5, 2));
        let start = m.tiles[0].clone();
        for _ in 0..4 {
            m.rotate_tiles(&[2], false).unwrap();
        }
        assert_eq!(m.tiles[0], start);
    }

    #[test]
    fn scale_after_rotation_uses_rotated_aspect() {
        let mut m = Manifest::default();
        m.tiles.push(tile(1, 0, -500, 400, 200));
        m.rotate_tiles(&[1], true).unwrap();
        m.scale_tile(1, 100, Corner::TopLeft).unwrap();
        assert_eq!((m.tiles[0].w, m.tiles[0].h), (100, 200));
    }

    #[test]
    fn delete_counts() {
        let mut m = Manifest::default();
        for i in 1..=3 {
            m.tiles.push(tile(i, 0, -100, 10, 10));
        }
        assert_eq!(m.delete_tiles(&[1, 3, 99]), 2);
        assert_eq!(m.tiles.len(), 1);
    }
}
