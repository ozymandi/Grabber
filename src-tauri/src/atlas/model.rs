//! Atlas workspace model (design D2 of add-texture-atlas): the manifest, canvas
//! coordinates, the sector grid and pure tile/sector operations. No I/O here.
//!
//! Canvas space is pixels, x right and y down (as on screen). Sector `(u, v)` covers
//! `x in [u*S, (u+1)*S)` and `y in [-(v+1)*S, -v*S)`, so `v` grows upward like UDIM rows.

use serde::{Deserialize, Serialize};

pub const FORMAT: u32 = 2;
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

/// Where a tile sits (design D1 of add-atlas-fine-work). The centre is kept in half pixels
/// (`c2` = 2 x centre) so odd sizes and quarter turns about any whole- or half-pixel pivot stay
/// exact. `w x h` is the placed size before rotation (the scaled original); the angle is
/// `rot * 90 + tilt` degrees clockwise. `pivot` is in whole pixels from the unrotated frame's
/// top-left; `None` is the centre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub c2: [i64; 2],
    pub w: i64,
    pub h: i64,
    pub rot: u8,
    #[serde(default)]
    pub tilt: f64,
    #[serde(default)]
    pub pivot: Option<[i64; 2]>,
}

/// Quarter turns clockwise on screen (y down) of an integer vector.
pub fn quarter(v: [i64; 2], turns: u8) -> [i64; 2] {
    match turns % 4 {
        0 => v,
        1 => [-v[1], v[0]],
        2 => [-v[0], -v[1]],
        _ => [v[1], -v[0]],
    }
}

/// Rotates a vector clockwise on screen (y down) by `deg` degrees.
pub fn turn(v: [f64; 2], deg: f64) -> [f64; 2] {
    let (s, c) = deg.to_radians().sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c]
}

fn floor_half(v: i64) -> i64 {
    v.div_euclid(2)
}

impl Frame {
    /// A frame for an unrotated rectangle.
    pub fn from_rect(r: Rect) -> Frame {
        Frame { c2: [2 * r.x + r.w, 2 * r.y + r.h], w: r.w, h: r.h, rot: 0, tilt: 0.0, pivot: None }
    }

    pub fn angle(&self) -> f64 {
        self.rot as f64 * 90.0 + self.tilt
    }

    /// Size after the quarter turns (width, height); the tilt is not included.
    pub fn rotated_size(&self) -> (i64, i64) {
        if self.rot % 2 == 1 {
            (self.h, self.w)
        } else {
            (self.w, self.h)
        }
    }

    pub fn centre(&self) -> [f64; 2] {
        [self.c2[0] as f64 / 2.0, self.c2[1] as f64 / 2.0]
    }

    /// Pivot offset from the centre in the unrotated frame, in half pixels.
    pub fn pivot_local2(&self) -> [i64; 2] {
        match self.pivot {
            None => [0, 0],
            Some(p) => [2 * p[0] - self.w, 2 * p[1] - self.h],
        }
    }

    /// Canvas position of a point given as an offset from the centre in the unrotated frame.
    pub fn to_canvas(&self, local: [f64; 2]) -> [f64; 2] {
        let v = turn(local, self.angle());
        let c = self.centre();
        [c[0] + v[0], c[1] + v[1]]
    }

    pub fn pivot_canvas(&self) -> [f64; 2] {
        let l = self.pivot_local2();
        self.to_canvas([l[0] as f64 / 2.0, l[1] as f64 / 2.0])
    }

    /// Canvas corners of the frame: top-left, top-right, bottom-right, bottom-left of the
    /// unrotated tile.
    pub fn corners(&self) -> [[f64; 2]; 4] {
        let (hw, hh) = (self.w as f64 / 2.0, self.h as f64 / 2.0);
        [[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]].map(|l| self.to_canvas(l))
    }

    /// Bounding box in whole pixels. Without tilt it is the exact rotated rectangle, its
    /// top-left floored to whole pixels; with tilt it covers the four rotated corners.
    pub fn rect(&self) -> Rect {
        if self.tilt == 0.0 {
            let (rw, rh) = self.rotated_size();
            return Rect { x: floor_half(self.c2[0] - rw), y: floor_half(self.c2[1] - rh), w: rw, h: rh };
        }
        let cs = self.corners();
        let x0 = cs.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min).floor() as i64;
        let y0 = cs.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min).floor() as i64;
        let x1 = cs.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max).ceil() as i64;
        let y1 = cs.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max).ceil() as i64;
        Rect { x: x0, y: y0, w: (x1 - x0).max(1), h: (y1 - y0).max(1) }
    }

    /// Moves the frame so that its centre is at `c` (canvas pixels), rounded to half pixels.
    fn set_centre(&mut self, c: [f64; 2]) {
        self.c2 = [(2.0 * c[0]).round() as i64, (2.0 * c[1]).round() as i64];
    }

    /// Sets the total angle, split into quarter turns and a tilt in [-45, 45].
    fn set_angle(&mut self, deg: f64) {
        let t = deg.rem_euclid(360.0);
        let q = (t / 90.0).round();
        let mut tilt = t - q * 90.0;
        if tilt.abs() < 1e-9 {
            tilt = 0.0;
        }
        self.rot = (q as i64).rem_euclid(4) as u8;
        self.tilt = tilt;
    }

    /// A quarter turn about the pivot. Exact in half pixels when there is no tilt, so four
    /// turns are an identity.
    pub fn quarter_turn(&mut self, clockwise: bool) {
        let turns = if clockwise { 1 } else { 3 };
        if self.tilt == 0.0 {
            let l = quarter(self.pivot_local2(), self.rot);
            let p2 = [self.c2[0] + l[0], self.c2[1] + l[1]];
            let d = quarter([self.c2[0] - p2[0], self.c2[1] - p2[1]], turns);
            self.c2 = [p2[0] + d[0], p2[1] + d[1]];
            self.rot = (self.rot + turns) % 4;
        } else {
            let delta = if clockwise { 90.0 } else { -90.0 };
            self.turn_to(self.angle() + delta);
        }
    }

    /// Turns the frame about its pivot to the total angle `deg`.
    pub fn turn_to(&mut self, deg: f64) {
        let p = self.pivot_canvas();
        let c = self.centre();
        let d = turn([c[0] - p[0], c[1] - p[1]], deg - self.angle());
        self.set_angle(deg);
        self.set_centre([p[0] + d[0], p[1] + d[1]]);
    }

    /// Scales to `new_w x new_h` (before rotation) keeping `anchor` fixed on the canvas.
    pub fn scale_to(&mut self, new_w: i64, new_h: i64, anchor: Corner) {
        let (hw, hh) = (self.w as f64 / 2.0, self.h as f64 / 2.0);
        let local = |c: Corner, hw: f64, hh: f64, l2: [i64; 2]| -> [f64; 2] {
            match c {
                Corner::TopLeft => [-hw, -hh],
                Corner::TopRight => [hw, -hh],
                Corner::BottomRight => [hw, hh],
                Corner::BottomLeft => [-hw, hh],
                Corner::Pivot => [l2[0] as f64 / 2.0, l2[1] as f64 / 2.0],
            }
        };
        let fixed = self.to_canvas(local(anchor, hw, hh, self.pivot_local2()));
        let (ow, oh) = (self.w, self.h);
        self.pivot = self.pivot.map(|p| {
            [
                ((p[0] as f64) * new_w as f64 / ow as f64).round() as i64,
                ((p[1] as f64) * new_h as f64 / oh as f64).round() as i64,
            ]
        });
        self.w = new_w;
        self.h = new_h;
        let off = turn(local(anchor, new_w as f64 / 2.0, new_h as f64 / 2.0, self.pivot_local2()), self.angle());
        self.set_centre([fixed[0] - off[0], fixed[1] - off[1]]);
    }
}

/// A placed texture: the original `w0 x h0`, where it sits ([`Frame`]) and where it came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub id: u64,
    pub item: String,
    pub w0: u32,
    pub h0: u32,
    #[serde(flatten)]
    pub frame: Frame,
    #[serde(default)]
    pub source: Provenance,
}

impl Tile {
    /// Bounding box on the canvas.
    pub fn rect(&self) -> Rect {
        self.frame.rect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// UV-layout guides, at most one per sector (design D4 of add-atlas-fine-work).
    #[serde(default)]
    pub guides: Vec<Guide>,
    #[serde(default = "default_true")]
    pub guide_show: bool,
    /// Percent, 0..=100.
    #[serde(default = "default_opacity")]
    pub guide_opacity: u8,
    #[serde(default)]
    pub fill: Fill,
    /// Edge fill distance in pixels, 1..=sector size.
    #[serde(default = "default_edge")]
    pub edge: u32,
}

pub const DEFAULT_OPACITY: u8 = 50;
/// Longest workspace name, in characters.
pub const MAX_NAME: usize = 100;
pub const DEFAULT_EDGE: u32 = 16;

fn default_true() -> bool {
    true
}
fn default_opacity() -> u8 {
    DEFAULT_OPACITY
}
fn default_edge() -> u32 {
    DEFAULT_EDGE
}

/// A UV-layout image drawn on a sector; never exported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guide {
    pub sector: Sector,
    pub item: String,
    pub w: u32,
    pub h: u32,
}

/// How empty space is filled on export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fill {
    #[default]
    Transparent,
    Extend,
}

/// The UDIM number in a guide file name `<name>.<udim>.png` (1001..=1999), if any.
pub fn udim_in_name(name: &str) -> Option<u32> {
    let lower = name.to_ascii_lowercase();
    let stem = lower.strip_suffix(".png")?;
    let (_, num) = stem.rsplit_once('.')?;
    if num.len() != 4 || !num.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: u32 = num.parse().ok()?;
    (1001..=1999).contains(&n).then_some(n)
}

/// Where a loaded guide goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideTarget {
    Udim(u32),
    /// The given sector, else the active one, else the first in UDIM order.
    Single(Option<Sector>),
}

/// What a guide LOAD did, for the status row and for undo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuideReport {
    pub loaded: usize,
    pub created: Vec<Sector>,
    /// Each sector that received a guide, with the guide it had before.
    pub replaced: Vec<(Sector, Option<Guide>)>,
    pub not_square: usize,
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
            guides: Vec::new(),
            guide_show: true,
            guide_opacity: DEFAULT_OPACITY,
            fill: Fill::Transparent,
            edge: DEFAULT_EDGE,
        }
    }
}

/// Why a manifest could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Corrupt(String),
    /// Written by a newer ripper than this one.
    Newer(u32),
}

/// Reads a manifest of any known format; format 1 (ripper 2.0.0) is upgraded on the way in.
pub fn parse_manifest(text: &str) -> Result<Manifest, ParseError> {
    let mut v: serde_json::Value = serde_json::from_str(text).map_err(|e| ParseError::Corrupt(e.to_string()))?;
    let format = v.get("format").and_then(|f| f.as_u64()).unwrap_or(1) as u32;
    if format > FORMAT {
        return Err(ParseError::Newer(format));
    }
    if format < 2 {
        if let Some(tiles) = v.get_mut("tiles").and_then(|t| t.as_array_mut()) {
            for t in tiles {
                let o = t.as_object_mut().ok_or_else(|| ParseError::Corrupt("tile is not an object".into()))?;
                let num = |k: &str| o.get(k).and_then(|x| x.as_i64()).ok_or_else(|| ParseError::Corrupt(format!("tile field {k}")));
                let (x, y, w, h) = (num("x")?, num("y")?, num("w")?, num("h")?);
                let rot = num("rot")?.rem_euclid(4);
                let (uw, uh) = if rot % 2 == 1 { (h, w) } else { (w, h) };
                o.remove("x");
                o.remove("y");
                o.insert("c2".into(), serde_json::json!([2 * x + w, 2 * y + h]));
                o.insert("w".into(), uw.into());
                o.insert("h".into(), uh.into());
                o.insert("rot".into(), rot.into());
            }
        }
        v["format"] = FORMAT.into();
    }
    serde_json::from_value(v).map_err(|e| ParseError::Corrupt(e.to_string()))
}

/// Why an operation was refused; the text goes to the status row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused(pub String);

fn refuse<T>(msg: impl Into<String>) -> Result<T, Refused> {
    Err(Refused(msg.into()))
}

/// The point kept fixed while scaling: a corner of the unrotated tile, or its pivot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    Pivot,
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
        self.guides.retain(|g| g.sector != s);
        if self.active == Some(s) {
            self.active = None;
        }
        Ok(())
    }

    /// Every item the workspace references: tile originals and guide images.
    pub fn items(&self) -> std::collections::BTreeSet<&str> {
        self.tiles.iter().map(|t| t.item.as_str()).chain(self.guides.iter().map(|g| g.item.as_str())).collect()
    }

    pub fn guide(&self, s: Sector) -> Option<&Guide> {
        self.guides.iter().find(|g| g.sector == s)
    }

    /// Puts `guide` on sector `s` (or clears it with `None`); returns the guide it had.
    pub fn set_guide(&mut self, s: Sector, guide: Option<Guide>) -> Result<Option<Guide>, Refused> {
        if guide.is_some() && !self.has_sector(s) {
            return refuse("That sector no longer exists");
        }
        let old = self.guides.iter().position(|g| g.sector == s).map(|i| self.guides.remove(i));
        if let Some(mut g) = guide {
            g.sector = s;
            self.guides.push(g);
        }
        Ok(old)
    }

    /// Bottom-left cell of the grid's bounding box: UDIM 1001 for guide sets.
    pub fn grid_origin(&self) -> Sector {
        Sector {
            u: self.sectors.iter().map(|s| s.u).min().unwrap_or(0),
            v: self.sectors.iter().map(|s| s.v).min().unwrap_or(0),
        }
    }

    /// Places loaded guide images (design D4): UDIM numbers count from the grid's bottom-left
    /// cell and create missing sectors; a single guide goes to the target, the active sector
    /// or the first sector in UDIM order. More than one single guide is refused.
    pub fn place_guides(&mut self, loads: &[(GuideTarget, String, u32, u32)]) -> Result<GuideReport, Refused> {
        let singles = loads.iter().filter(|l| matches!(l.0, GuideTarget::Single(_))).count();
        if singles > 1 {
            return refuse("Several guides without a UDIM number (name.1001.png); load them one at a time");
        }
        let origin = self.grid_origin();
        let mut rep = GuideReport::default();
        for (target, item, w, h) in loads {
            let s = match *target {
                GuideTarget::Udim(n) => {
                    let k = (n - 1001) as i32;
                    Sector { u: origin.u + k % 10, v: origin.v + k / 10 }
                }
                GuideTarget::Single(t) => t
                    .filter(|t| self.has_sector(*t))
                    .or(self.active.filter(|a| self.has_sector(*a)))
                    .or_else(|| self.sectors_udim_order().first().copied())
                    .ok_or_else(|| Refused("The workspace has no sector for the guide".into()))?,
            };
            if !self.has_sector(s) {
                self.sectors.push(s);
                rep.created.push(s);
            }
            let old = self.set_guide(s, Some(Guide { sector: s, item: item.clone(), w: *w, h: *h }))?;
            if !rep.replaced.iter().any(|(c, _)| *c == s) {
                rep.replaced.push((s, old));
            }
            rep.loaded += 1;
            if w != h {
                rep.not_square += 1;
            }
        }
        Ok(rep)
    }

    /// Names the workspace (2.1.2). The name becomes the default file name on SAVE and the
    /// default export name, so it must be usable in a Windows file name.
    pub fn rename(&mut self, name: &str) -> Result<(), Refused> {
        let name = name.trim();
        if name.is_empty() {
            return refuse("Enter a name for the workspace");
        }
        if name.chars().count() > MAX_NAME {
            return refuse(format!("A workspace name can be at most {MAX_NAME} characters"));
        }
        if name.chars().any(|c| c.is_control() || r#"<>:"/\|?*"#.contains(c)) || name.ends_with('.') {
            return refuse(r#"A workspace name cannot contain < > : " / \ | ? * or end with a dot"#);
        }
        self.name = name.to_string();
        Ok(())
    }

    /// Guide visibility for the whole workspace; opacity in percent.
    pub fn set_guide_view(&mut self, show: Option<bool>, opacity: Option<u8>) -> Result<(), Refused> {
        if let Some(o) = opacity {
            if o > 100 {
                return refuse("Opacity is a percentage from 0 to 100");
            }
            self.guide_opacity = o;
        }
        if let Some(s) = show {
            self.guide_show = s;
        }
        Ok(())
    }

    /// Export fill for the whole workspace; the edge is 1..=sector size pixels.
    pub fn set_fill(&mut self, fill: Fill, edge: u32) -> Result<(), Refused> {
        if edge < 1 || edge > self.sector_size {
            return refuse(format!("Edge must be from 1 to {} px", self.sector_size));
        }
        self.fill = fill;
        self.edge = edge;
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
        self.edge = self.edge.min(size);
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

    /// Moves tiles by whole pixels.
    pub fn move_tiles(&mut self, ids: &[u64], dx: i64, dy: i64) -> Result<(), Refused> {
        for &id in ids {
            let t = self.tile_mut(id)?;
            t.frame.c2[0] += 2 * dx;
            t.frame.c2[1] += 2 * dy;
        }
        Ok(())
    }

    /// Scales a tile uniformly to `new_w` pixels wide before rotation, along its own axes,
    /// keeping `anchor` fixed. The height follows the original's aspect ratio; both sides are
    /// at least 1 pixel.
    pub fn scale_tile(&mut self, id: u64, new_w: i64, anchor: Corner) -> Result<(), Refused> {
        let t = self.tile_mut(id)?;
        let w = new_w.max(1);
        let h = ((w as f64) * (t.h0 as f64) / (t.w0 as f64)).round().max(1.0) as i64;
        t.frame.scale_to(w, h, anchor);
        Ok(())
    }

    /// Rotates tiles a quarter turn about their own pivots. Without tilt this is exact, so
    /// four turns return to the starting frame.
    pub fn rotate_tiles(&mut self, ids: &[u64], clockwise: bool) -> Result<(), Refused> {
        for &id in ids {
            self.tile_mut(id)?.frame.quarter_turn(clockwise);
        }
        Ok(())
    }

    /// Turns a tile about its pivot to a total angle in degrees, clockwise.
    pub fn turn_tile(&mut self, id: u64, angle: f64) -> Result<(), Refused> {
        if !angle.is_finite() {
            return refuse("The angle must be a number");
        }
        self.tile_mut(id)?.frame.turn_to(angle);
        Ok(())
    }

    /// Moves a tile's pivot (whole pixels from the unrotated top-left) or resets it with `None`.
    /// The tile itself does not move.
    pub fn set_pivot(&mut self, id: u64, pivot: Option<[i64; 2]>) -> Result<(), Refused> {
        self.tile_mut(id)?.frame.pivot = pivot;
        Ok(())
    }

    /// Puts a tile back into an earlier frame (undo).
    pub fn set_frame(&mut self, id: u64, frame: Frame) -> Result<(), Refused> {
        self.tile_mut(id)?.frame = frame;
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
        Tile { id, item: format!("{id}.png"), w0: w as u32, h0: h as u32, frame: Frame::from_rect(Rect { x, y, w, h }), source: Provenance::default() }
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
        let r = m.tiles[0].rect();
        assert_eq!((r.w, r.h, m.tiles[0].frame.rot), (200, 400, 1));
        assert_eq!((r.x + r.w / 2, r.y + r.h / 2), (200, -200), "centre kept");
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
        // The width is given before rotation: 400 x 200 becomes 100 x 50, shown as 50 x 100.
        m.scale_tile(1, 100, Corner::TopLeft).unwrap();
        let r = m.tiles[0].rect();
        assert_eq!((r.w, r.h), (50, 100));
    }

    fn close(a: [f64; 2], b: [f64; 2], eps: f64) -> bool {
        (a[0] - b[0]).abs() <= eps && (a[1] - b[1]).abs() <= eps
    }

    #[test]
    fn four_turns_about_any_pivot_are_exact() {
        for pivot in [Some([0, 0]), Some([400, 200]), Some([-37, 911]), Some([3, 1]), None] {
            for (w, h) in [(400, 200), (5, 2), (7, 7), (1, 9)] {
                let mut m = Manifest::default();
                m.tiles.push(tile(1, 13, -301, w, h));
                m.set_pivot(1, pivot).unwrap();
                let start = m.tiles[0].clone();
                for _ in 0..4 {
                    m.rotate_tiles(&[1], true).unwrap();
                }
                assert_eq!(m.tiles[0], start, "cw pivot {pivot:?} size {w}x{h}");
                for _ in 0..4 {
                    m.rotate_tiles(&[1], false).unwrap();
                }
                assert_eq!(m.tiles[0], start, "ccw pivot {pivot:?} size {w}x{h}");
            }
        }
    }

    #[test]
    fn rotate_about_a_corner_keeps_that_corner() {
        let mut m = Manifest::default();
        m.tiles.push(tile(1, 0, -300, 400, 200));
        m.set_pivot(1, Some([0, 0])).unwrap();
        m.rotate_tiles(&[1], true).unwrap();
        let f = &m.tiles[0].frame;
        assert!(close(f.corners()[0], [0.0, -300.0], 1e-9), "former top-left stays");
        assert_eq!(m.tiles[0].rect(), Rect { x: -200, y: -300, w: 200, h: 400 });
    }

    #[test]
    fn quarter_turn_adds_to_a_free_angle() {
        let mut m = Manifest::default();
        m.tiles.push(tile(1, 0, -300, 400, 200));
        m.turn_tile(1, 17.0).unwrap();
        m.rotate_tiles(&[1], true).unwrap();
        assert!((m.tiles[0].frame.angle() - 107.0).abs() < 1e-9);
        assert_eq!(m.tiles[0].frame.rot, 1);
    }

    #[test]
    fn free_turn_and_back() {
        let mut m = Manifest::default();
        m.tiles.push(tile(1, 10, -310, 401, 199));
        m.set_pivot(1, Some([30, 40])).unwrap();
        let start = m.tiles[0].frame.clone();
        m.turn_tile(1, 30.0).unwrap();
        assert!((m.tiles[0].frame.tilt - 30.0).abs() < 1e-9);
        assert!(close(m.tiles[0].frame.pivot_canvas(), start.pivot_canvas(), 0.5), "pivot stays put");
        m.turn_tile(1, 0.0).unwrap();
        let f = &m.tiles[0].frame;
        assert_eq!(f.tilt, 0.0, "exact zero tilt");
        assert!(close(f.centre(), start.centre(), 0.5));
        m.turn_tile(1, 90.0).unwrap();
        assert_eq!((m.tiles[0].frame.rot, m.tiles[0].frame.tilt), (1, 0.0), "90 degrees is lossless");
        m.turn_tile(1, -50.0).unwrap();
        assert_eq!(m.tiles[0].frame.rot, 3);
        assert!((m.tiles[0].frame.tilt - 40.0).abs() < 1e-9);
    }

    #[test]
    fn pivot_moves_with_the_tile() {
        let mut m = Manifest::default();
        m.tiles.push(tile(1, 0, -300, 400, 200));
        m.set_pivot(1, Some([10, 20])).unwrap();
        let p = m.tiles[0].frame.pivot_canvas();
        m.move_tiles(&[1], 100, -5).unwrap();
        assert!(close(m.tiles[0].frame.pivot_canvas(), [p[0] + 100.0, p[1] - 5.0], 1e-9));
    }

    #[test]
    fn tilted_bounding_box_covers_corners() {
        let mut m = Manifest::default();
        m.tiles.push(tile(1, 0, -300, 400, 200));
        m.turn_tile(1, 33.0).unwrap();
        let r = m.tiles[0].rect();
        for c in m.tiles[0].frame.corners() {
            assert!(c[0] >= r.x as f64 && c[0] <= r.right() as f64 && c[1] >= r.y as f64 && c[1] <= r.bottom() as f64);
        }
    }

    #[test]
    fn scale_a_rotated_tile_about_the_opposite_corner() {
        let mut m = Manifest::default();
        m.tiles.push(tile(1, 0, -500, 512, 256));
        m.turn_tile(1, 30.0).unwrap();
        let fixed = m.tiles[0].frame.corners()[0];
        // Dragging the bottom-right handle keeps the top-left corner.
        m.scale_tile(1, 1024, Corner::TopLeft).unwrap();
        let f = &m.tiles[0].frame;
        assert_eq!((f.w, f.h), (1024, 512));
        assert!((f.angle() - 30.0).abs() < 1e-9, "still turned");
        assert!(close(f.corners()[0], fixed, 0.5));
    }

    #[test]
    fn scale_about_the_pivot_keeps_the_centre() {
        let mut m = Manifest::default();
        m.tiles.push(tile(1, 0, -500, 512, 256));
        let c = m.tiles[0].frame.centre();
        m.scale_tile(1, 1024, Corner::Pivot).unwrap();
        let f = &m.tiles[0].frame;
        assert_eq!((f.w, f.h), (1024, 512));
        assert_eq!(f.centre(), c);
        // A moved pivot keeps its place on the canvas and scales with the tile.
        m.set_pivot(1, Some([100, 50])).unwrap();
        let p = m.tiles[0].frame.pivot_canvas();
        m.scale_tile(1, 512, Corner::Pivot).unwrap();
        assert_eq!(m.tiles[0].frame.pivot, Some([50, 25]));
        assert!(close(m.tiles[0].frame.pivot_canvas(), p, 0.5));
    }

    const V1: &str = r#"{"format":1,"revision":4,"name":"cafe","sector_size":1024,"padding":8,
        "sectors":[{"u":0,"v":0}],"active":null,"next_id":3,
        "tiles":[{"id":1,"item":"a.png","w0":400,"h0":200,"x":10,"y":-500,"w":400,"h":200,"rot":0},
                 {"id":2,"item":"b.png","w0":5,"h0":2,"x":7,"y":-33,"w":2,"h":5,"rot":1,"source":{"stem":"s","quad":2}}]}"#;

    #[test]
    fn format_one_is_upgraded() {
        let m = parse_manifest(V1).unwrap();
        assert_eq!(m.format, FORMAT);
        assert_eq!(m.tiles[0].rect(), Rect { x: 10, y: -500, w: 400, h: 200 });
        assert_eq!(m.tiles[1].rect(), Rect { x: 7, y: -33, w: 2, h: 5 });
        assert_eq!((m.tiles[1].frame.w, m.tiles[1].frame.h, m.tiles[1].frame.rot), (5, 2, 1));
        assert_eq!((m.tiles[1].frame.tilt, m.tiles[1].frame.pivot), (0.0, None));
        assert_eq!(m.tiles[1].source.quad, 2);
        let again = parse_manifest(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(again, m);
    }

    #[test]
    fn guide_and_fill_defaults() {
        for m in [Manifest::default(), parse_manifest(V1).unwrap()] {
            assert!(m.guides.is_empty());
            assert!(m.guide_show);
            assert_eq!(m.guide_opacity, 50);
            assert_eq!((m.fill, m.edge), (Fill::Transparent, 16));
        }
    }

    #[test]
    fn view_and_fill_ranges() {
        let mut m = Manifest::default();
        m.sector_size = 1024;
        assert!(m.set_guide_view(None, Some(101)).is_err());
        m.set_guide_view(Some(false), Some(30)).unwrap();
        assert_eq!((m.guide_show, m.guide_opacity), (false, 30));
        assert!(m.set_fill(Fill::Extend, 0).is_err());
        assert!(m.set_fill(Fill::Extend, 1025).is_err());
        m.set_fill(Fill::Extend, 1024).unwrap();
        assert_eq!((m.fill, m.edge), (Fill::Extend, 1024));
        m.set_sector_size(512).unwrap();
        assert_eq!(m.edge, 512, "edge follows a smaller sector");
    }

    fn g(item: &str, w: u32, h: u32) -> (String, u32, u32) {
        (item.to_string(), w, h)
    }

    fn load(m: &mut Manifest, files: &[(&str, u32, u32)], target: Option<Sector>) -> Result<GuideReport, Refused> {
        let loads: Vec<_> = files
            .iter()
            .map(|(n, w, h)| {
                let t = match udim_in_name(n) {
                    Some(u) => GuideTarget::Udim(u),
                    None => GuideTarget::Single(target),
                };
                let (i, w, h) = g(n, *w, *h);
                (t, i, w, h)
            })
            .collect();
        m.place_guides(&loads)
    }

    #[test]
    fn udim_names() {
        assert_eq!(udim_in_name("body.1001.png"), Some(1001));
        assert_eq!(udim_in_name("Body.UV.1012.PNG"), Some(1012));
        assert_eq!(udim_in_name("body_uv.png"), None);
        assert_eq!(udim_in_name("body.1000.png"), None);
        assert_eq!(udim_in_name("body.2001.png"), None);
        assert_eq!(udim_in_name("body.101.png"), None);
    }

    #[test]
    fn set_into_a_one_sector_workspace_creates_sectors() {
        let mut m = Manifest::default();
        let files = [("body.1001.png", 4096, 4096), ("body.1002.png", 4096, 4096), ("body.1003.png", 4096, 4096), ("body.1004.png", 4096, 4096)];
        let r = load(&mut m, &files, None).unwrap();
        assert_eq!((r.loaded, r.created.len(), r.not_square), (4, 3, 0));
        assert_eq!(m.sectors.len(), 4);
        for u in 0..4 {
            assert_eq!(m.guide(s(u, 0)).unwrap().item, format!("body.100{}.png", u + 1));
        }
    }

    #[test]
    fn numbering_starts_at_the_grid_bottom_left_and_ignores_the_selection() {
        let mut m = Manifest::default();
        m.add_sector(s(0, 0), Side::Left).unwrap(); // grid is now u = -1..0
        load(&mut m, &[("body.1001.png", 8, 8)], Some(s(0, 0))).unwrap();
        assert!(m.guide(s(-1, 0)).is_some(), "1001 is the bottom-left sector");
        assert!(m.guide(s(0, 0)).is_none());
        // Second row: created above the bottom-left sector.
        let r = load(&mut m, &[("body.1011.png", 8, 8)], None).unwrap();
        assert_eq!(r.created, vec![s(-1, 1)]);
    }

    #[test]
    fn single_guide_goes_to_target_then_active() {
        let mut m = Manifest::default();
        m.add_sector(s(0, 0), Side::Right).unwrap();
        load(&mut m, &[("uv.png", 64, 64)], Some(s(1, 0))).unwrap();
        assert_eq!(m.guide(s(1, 0)).unwrap().item, "uv.png");
        m.set_active(s(0, 0)).unwrap();
        load(&mut m, &[("other.png", 64, 32)], None).unwrap();
        assert_eq!(m.guide(s(0, 0)).unwrap().item, "other.png");
        // Replacing keeps one guide per sector and reports the old one.
        let r = load(&mut m, &[("again.png", 64, 64)], Some(s(1, 0))).unwrap();
        assert_eq!(m.guides.iter().filter(|g| g.sector == s(1, 0)).count(), 1);
        assert_eq!(r.replaced[0].1.as_ref().unwrap().item, "uv.png");
        assert!(load(&mut m, &[("a.png", 1, 1), ("b.png", 1, 1)], None).is_err(), "two singles refused");
    }

    #[test]
    fn non_square_guide_is_counted() {
        let mut m = Manifest::default();
        let r = load(&mut m, &[("uv.png", 2048, 1024)], None).unwrap();
        assert_eq!(r.not_square, 1);
    }

    #[test]
    fn removing_a_sector_removes_its_guide() {
        let mut m = Manifest::default();
        m.add_sector(s(0, 0), Side::Right).unwrap();
        load(&mut m, &[("body.1002.png", 8, 8)], None).unwrap();
        m.remove_sector(s(1, 0)).unwrap();
        assert!(m.guides.is_empty());
    }

    #[test]
    fn rename_checks_the_name() {
        let mut m = Manifest::default();
        m.rename("  cafe facades ").unwrap();
        assert_eq!(m.name, "cafe facades");
        for bad in ["", "   ", "a/b", "a:b", "what?", "dot.", &"x".repeat(101)] {
            assert!(m.rename(bad).is_err(), "{bad:?}");
        }
        assert_eq!(m.name, "cafe facades", "a refused name changes nothing");
    }

    #[test]
    fn newer_format_is_refused() {
        let v = r#"{"format":3,"revision":0,"name":"x","sector_size":1024,"padding":8,"sectors":[],"active":null,"tiles":[],"next_id":1}"#;
        assert_eq!(parse_manifest(v), Err(ParseError::Newer(3)));
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
