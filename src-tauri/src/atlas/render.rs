//! Export rendering (design D5 of add-texture-atlas). Every tile is drawn from its original:
//! an exact pixel permutation at scale 1, otherwise the RIP export sampler
//! ([`crate::warp::warp`]) with its Jacobian-adaptive supersampling. Outputs start fully
//! transparent; tiles are composited source-over in manifest order and clipped per sector.

use super::model::{udim, Manifest, Rect, Refused, Sector, Tile};
use crate::warp::{self, Rgba};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Largest side of a GROUP FILE sheet; 16384^2 RGBA is 1 GiB.
pub const MAX_SHEET: i64 = 16384;
/// UDIM sets have ten columns.
pub const MAX_UDIM_COLUMNS: i32 = 10;

/// A straight-alpha RGBA8 image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn transparent(w: u32, h: u32) -> Image {
        Image { w, h, rgba: vec![0; w as usize * h as usize * 4] }
    }
}

/// Source corner for each output corner (TL, TR, BR, BL) after `rot` clockwise quarter
/// turns of a `w0 x h0` original.
fn rotated_corners(w0: u32, h0: u32, rot: u8) -> [[f64; 2]; 4] {
    let (w, h) = (w0 as f64, h0 as f64);
    let base = [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]];
    let k = (rot % 4) as usize;
    [base[(4 - k) % 4], base[(5 - k) % 4], base[(6 - k) % 4], base[(7 - k) % 4]]
}

/// Lossless quarter-turn rotation of an original.
pub fn rotate_exact(src: &Image, rot: u8) -> Image {
    let (w0, h0) = (src.w as usize, src.h as usize);
    let rot = rot % 4;
    let (w, h) = if rot % 2 == 1 { (h0, w0) } else { (w0, h0) };
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = match rot {
                0 => (x, y),
                1 => (y, h0 - 1 - x),
                2 => (w0 - 1 - x, h0 - 1 - y),
                _ => (w0 - 1 - y, x),
            };
            let s = (sy * w0 + sx) * 4;
            let d = (y * w + x) * 4;
            out[d..d + 4].copy_from_slice(&src.rgba[s..s + 4]);
        }
    }
    Image { w: w as u32, h: h as u32, rgba: out }
}

/// Renders a tile's original at its placed size and rotation.
pub fn render_tile(original: &Image, rot: u8, w: u32, h: u32) -> Image {
    let (rw, rh) = if rot % 2 == 1 { (original.h, original.w) } else { (original.w, original.h) };
    if (rw, rh) == (w, h) {
        return rotate_exact(original, rot);
    }
    let src = Rgba { w: original.w, h: original.h, data: &original.rgba };
    let corners = rotated_corners(original.w, original.h, rot);
    let rgba = warp::warp(&src, &corners, w, h).unwrap_or_else(|| vec![0; w as usize * h as usize * 4]);
    Image { w, h, rgba }
}

/// Straight-alpha source-over of one pixel.
#[inline]
fn over(dst: &mut [u8], src: &[u8]) {
    let sa = src[3] as f32 / 255.0;
    if sa <= 0.0 {
        return;
    }
    if sa >= 1.0 {
        dst.copy_from_slice(src);
        return;
    }
    let da = dst[3] as f32 / 255.0;
    let a = sa + da * (1.0 - sa);
    for k in 0..3 {
        let c = (src[k] as f32 * sa + dst[k] as f32 * da * (1.0 - sa)) / a;
        dst[k] = c.round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
}

/// Draws the part of `tile` (rendered as `img`) that lies in `clip` into `out`, whose
/// top-left corner is at canvas position `origin`.
fn blit(out: &mut Image, origin: (i64, i64), clip: &Rect, tile: &Tile, img: &Image) {
    let Some(r) = clip.intersection(&tile.rect()) else { return };
    let ow = out.w as usize;
    let tw = img.w as usize;
    for y in r.y..r.bottom() {
        let oy = (y - origin.1) as usize;
        let ty = (y - tile.y) as usize;
        for x in r.x..r.right() {
            let o = (oy * ow + (x - origin.0) as usize) * 4;
            let t = (ty * tw + (x - tile.x) as usize) * 4;
            over(&mut out.rgba[o..o + 4], &img.rgba[t..t + 4]);
        }
    }
}

/// Renders the cells `cells` of a canvas area starting at `origin` into an image of
/// `w x h`. Tiles are drawn once each and clipped per cell, so a tile across two selected
/// cells joins without a seam. Everything outside the cells stays transparent.
pub fn render_area(
    m: &Manifest,
    origin: (i64, i64),
    w: u32,
    h: u32,
    cells: &[Rect],
    load: &mut dyn FnMut(&Tile) -> Result<Image, String>,
) -> Result<Image, String> {
    let mut out = Image::transparent(w, h);
    let mut cache: HashMap<u64, Image> = HashMap::new();
    for t in &m.tiles {
        if !cells.iter().any(|c| c.intersects(&t.rect())) {
            continue;
        }
        if !cache.contains_key(&t.id) {
            let original = load(t)?;
            cache.insert(t.id, render_tile(&original, t.rot, t.w as u32, t.h as u32));
        }
        let img = &cache[&t.id];
        for c in cells {
            blit(&mut out, origin, c, t, img);
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// One PNG per selected sector.
    Sector,
    /// One PNG covering the selection's bounding rectangle.
    GroupFile,
    /// One PNG per selected sector, named `<name>.<udim>.png`.
    GroupSet,
}

/// One file of an export: the canvas area it covers and the cells drawn into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// File name without the collision suffix: `stem` plus `tail` (e.g. ".1001.png").
    pub tail: String,
    pub area: Rect,
    pub cells: Vec<Rect>,
}

/// Works out the files an export will write, before anything is rendered.
pub fn plan(m: &Manifest, selected: &[Sector], scope: Scope) -> Result<Vec<Output>, Refused> {
    let mut sel: Vec<Sector> = selected.iter().copied().filter(|s| m.has_sector(*s)).collect();
    sel.sort_by_key(|s| (s.v, s.u));
    sel.dedup();
    if sel.is_empty() {
        return Err(Refused("Select one or more sectors to export".into()));
    }
    let size = m.sector_size;
    let u0 = sel.iter().map(|s| s.u).min().unwrap();
    let v0 = sel.iter().map(|s| s.v).min().unwrap();
    let u1 = sel.iter().map(|s| s.u).max().unwrap();
    let v1 = sel.iter().map(|s| s.v).max().unwrap();
    let numbered = |s: &Sector| format!(".{}.png", udim(s.u - u0, s.v - v0));
    match scope {
        Scope::Sector if sel.len() == 1 => {
            let r = sel[0].rect(size);
            Ok(vec![Output { tail: ".png".into(), area: r, cells: vec![r] }])
        }
        Scope::Sector | Scope::GroupSet => {
            if u1 - u0 + 1 > MAX_UDIM_COLUMNS {
                return Err(Refused(format!(
                    "A UDIM set is at most {MAX_UDIM_COLUMNS} sectors wide; this selection is {}",
                    u1 - u0 + 1
                )));
            }
            Ok(sel
                .iter()
                .map(|s| {
                    let r = s.rect(size);
                    Output { tail: numbered(s), area: r, cells: vec![r] }
                })
                .collect())
        }
        Scope::GroupFile => {
            let s = size as i64;
            let area = Rect { x: u0 as i64 * s, y: -(v1 as i64 + 1) * s, w: (u1 - u0 + 1) as i64 * s, h: (v1 - v0 + 1) as i64 * s };
            if area.w > MAX_SHEET || area.h > MAX_SHEET {
                return Err(Refused(format!(
                    "A single sheet can be at most {MAX_SHEET} px per side; this one would be {} x {}. Use GROUP SET instead",
                    area.w, area.h
                )));
            }
            Ok(vec![Output { tail: ".png".into(), area, cells: sel.iter().map(|c| c.rect(size)).collect() }])
        }
    }
}

/// Smallest suffix so that no file of the export exists: `""`, then `"_2"`, `"_3"`...
/// The same suffix applies to every file, so a UDIM set keeps one name.
pub fn free_stem(base: &str, outputs: &[Output], exists: &dyn Fn(&str) -> bool) -> String {
    for n in 1u32.. {
        let stem = if n == 1 { base.to_string() } else { format!("{base}_{n}") };
        if !outputs.iter().any(|o| exists(&format!("{stem}{}", o.tail))) {
            return stem;
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::model::{Provenance, Side};

    fn img(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Image {
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&f(x, y));
            }
        }
        Image { w, h, rgba }
    }

    fn noise(w: u32, h: u32) -> Image {
        img(w, h, |x, y| {
            let v = (x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663)) as u8;
            [v, v.wrapping_mul(3), v.wrapping_add(x as u8), 128 + (y % 128) as u8]
        })
    }

    fn tile(id: u64, x: i64, y: i64, o: &Image, rot: u8) -> Tile {
        let (w, h) = if rot % 2 == 1 { (o.h, o.w) } else { (o.w, o.h) };
        Tile { id, item: format!("{id}"), w0: o.w, h0: o.h, x, y, w: w as i64, h: h as i64, rot, source: Provenance::default() }
    }

    fn px(i: &Image, x: u32, y: u32) -> [u8; 4] {
        let k = ((y * i.w + x) * 4) as usize;
        i.rgba[k..k + 4].try_into().unwrap()
    }

    #[test]
    fn four_quarter_turns_are_identity_and_match_warp() {
        let o = noise(37, 21);
        let mut r = o.clone();
        for _ in 0..4 {
            r = rotate_exact(&r, 1);
        }
        assert_eq!(r, o);
        // The exact path agrees with the general sampler for opaque content.
        let opaque = img(9, 5, |x, y| [(x * 20) as u8, (y * 40) as u8, 7, 255]);
        for rot in 0..4u8 {
            let exact = rotate_exact(&opaque, rot);
            let src = Rgba { w: opaque.w, h: opaque.h, data: &opaque.rgba };
            let warped = warp::warp(&src, &rotated_corners(opaque.w, opaque.h, rot), exact.w, exact.h).unwrap();
            assert_eq!(exact.rgba, warped, "rot {rot}");
        }
    }

    #[test]
    fn shrink_and_back_reproduces_original() {
        let o = noise(160, 90);
        let small = render_tile(&o, 0, 20, 11);
        assert_eq!((small.w, small.h), (20, 11));
        // Back to full size renders from the original, not from the small copy.
        assert_eq!(render_tile(&o, 0, 160, 90), o);
    }

    #[test]
    fn shrunk_stripes_stay_smooth() {
        let o = img(512, 64, |x, _| if x % 2 == 0 { [0, 0, 0, 255] } else { [255, 255, 255, 255] });
        let out = render_tile(&o, 0, 128, 16);
        let worst = out.rgba.chunks(4).map(|p| (p[0] as f64 - 127.5).abs()).fold(0.0, f64::max);
        assert!(worst <= 0.5, "worst deviation {worst}");
    }

    fn sectors(m: &mut Manifest, size: u32) {
        m.sector_size = size;
        m.add_sector(Sector { u: 0, v: 0 }, Side::Right).unwrap();
        m.add_sector(Sector { u: 0, v: 0 }, Side::Up).unwrap();
        m.add_sector(Sector { u: 1, v: 0 }, Side::Up).unwrap();
    }

    fn export(m: &Manifest, sel: &[Sector], scope: Scope, originals: &HashMap<u64, Image>) -> Vec<(String, Image)> {
        let outs = plan(m, sel, scope).unwrap();
        outs.iter()
            .map(|o| {
                let mut load = |t: &Tile| Ok(originals[&t.id].clone());
                let i = render_area(m, (o.area.x, o.area.y), o.area.w as u32, o.area.h as u32, &o.cells, &mut load).unwrap();
                (o.tail.clone(), i)
            })
            .collect()
    }

    #[test]
    fn single_sector_and_empty_sector() {
        let mut m = Manifest::default();
        m.sector_size = 64;
        let o = noise(10, 6);
        m.tiles.push(tile(1, 5, -60, &o, 0));
        let originals = HashMap::from([(1, o.clone())]);
        let out = export(&m, &[Sector { u: 0, v: 0 }], Scope::Sector, &originals);
        assert_eq!(out.len(), 1);
        let (tail, i) = &out[0];
        assert_eq!(tail, ".png");
        assert_eq!((i.w, i.h), (64, 64));
        assert_eq!(px(i, 5, 4), px(&o, 0, 0));
        assert_eq!(px(i, 0, 0), [0, 0, 0, 0]);
        let mut empty = Manifest::default();
        empty.sector_size = 64;
        let e = export(&empty, &[Sector { u: 0, v: 0 }], Scope::Sector, &HashMap::new());
        assert!(e[0].1.rgba.iter().all(|&b| b == 0), "fully transparent");
    }

    #[test]
    fn group_file_joins_border_tile_and_clears_unselected_cell() {
        let mut m = Manifest::default();
        sectors(&mut m, 32);
        let o = img(20, 8, |x, y| [x as u8 * 10, y as u8 * 20, 99, 255]);
        // Across the border of 1001 | 1002 in the bottom row.
        m.tiles.push(tile(1, 22, -20, &o, 0));
        // A tile in sector (1, 1) = 1012, which is left out of the selection below.
        m.tiles.push(tile(2, 40, -60, &img(8, 8, |_, _| [255, 0, 0, 255]), 0));
        let originals = HashMap::from([(1, o.clone()), (2, img(8, 8, |_, _| [255, 0, 0, 255]))]);
        let sel = [Sector { u: 0, v: 0 }, Sector { u: 1, v: 0 }, Sector { u: 0, v: 1 }];
        let out = export(&m, &sel, Scope::GroupFile, &originals);
        assert_eq!(out.len(), 1);
        let i = &out[0].1;
        assert_eq!((i.w, i.h), (64, 64), "two by two cells");
        // The border tile is whole: sheet origin is canvas (0, -64).
        for x in 0..20u32 {
            assert_eq!(px(i, 22 + x, 44), px(&o, x, 0));
        }
        // The cell of 1012 (top right of the sheet) is fully transparent.
        for y in 0..32 {
            for x in 32..64 {
                assert_eq!(px(i, x, y), [0, 0, 0, 0]);
            }
        }
    }

    #[test]
    fn border_halves_join_in_separate_sector_files() {
        let mut m = Manifest::default();
        sectors(&mut m, 32);
        let o = noise(20, 8);
        m.tiles.push(tile(1, 22, -20, &o, 0));
        let originals = HashMap::from([(1, o.clone())]);
        let out = export(&m, &[Sector { u: 0, v: 0 }, Sector { u: 1, v: 0 }], Scope::GroupSet, &originals);
        assert_eq!(out.iter().map(|o| o.0.as_str()).collect::<Vec<_>>(), [".1001.png", ".1002.png"]);
        let (left, right) = (&out[0].1, &out[1].1);
        for y in 0..8u32 {
            for x in 0..10u32 {
                assert_eq!(px(left, 22 + x, 12 + y), px(&o, x, y), "left half");
                assert_eq!(px(right, x, 12 + y), px(&o, 10 + x, y), "right half");
            }
        }
    }

    #[test]
    fn set_numbered_from_bottom_left_and_width_limit() {
        let mut m = Manifest::default();
        m.sector_size = 512;
        m.add_sector(Sector { u: 0, v: 0 }, Side::Left).unwrap();
        let outs = plan(&m, &[Sector { u: -1, v: 0 }, Sector { u: 0, v: 0 }], Scope::GroupSet).unwrap();
        assert_eq!(outs[0].tail, ".1001.png");
        assert_eq!(outs[0].area.x, -512, "left sector is 1001");
        assert_eq!(outs[1].tail, ".1002.png");
        let mut wide = Manifest::default();
        for u in 1..11 {
            wide.add_sector(Sector { u: u - 1, v: 0 }, Side::Right).unwrap();
        }
        let all: Vec<Sector> = wide.sectors.clone();
        assert_eq!(all.len(), 11);
        assert!(plan(&wide, &all, Scope::GroupSet).unwrap_err().0.contains("10"));
    }

    #[test]
    fn sheet_too_large_is_refused() {
        let mut m = Manifest::default();
        m.sector_size = 8192;
        m.add_sector(Sector { u: 0, v: 0 }, Side::Right).unwrap();
        m.add_sector(Sector { u: 1, v: 0 }, Side::Right).unwrap();
        let e = plan(&m, &m.sectors.clone(), Scope::GroupFile).unwrap_err();
        assert!(e.0.contains("16384") && e.0.contains("GROUP SET"));
    }

    #[test]
    fn tray_tiles_never_appear() {
        let mut m = Manifest::default();
        m.sector_size = 32;
        let o = img(8, 8, |_, _| [9, 9, 9, 255]);
        m.tiles.push(tile(1, 40, -20, &o, 0)); // right of the only sector
        let originals = HashMap::from([(1, o)]);
        let out = export(&m, &[Sector { u: 0, v: 0 }], Scope::Sector, &originals);
        assert!(out[0].1.rgba.iter().all(|&b| b == 0));
    }

    #[test]
    fn collision_suffix_applies_to_whole_set() {
        let outs = vec![
            Output { tail: ".1001.png".into(), area: Rect { x: 0, y: 0, w: 1, h: 1 }, cells: vec![] },
            Output { tail: ".1002.png".into(), area: Rect { x: 0, y: 0, w: 1, h: 1 }, cells: vec![] },
        ];
        let existing = ["cafe.1001.png"];
        assert_eq!(free_stem("cafe", &outs, &|n| existing.contains(&n)), "cafe_2");
        assert_eq!(free_stem("shop", &outs, &|n| existing.contains(&n)), "shop");
    }

    #[test]
    fn rotated_tile_renders_turned() {
        let o = img(4, 2, |x, y| [x as u8, y as u8, 0, 255]);
        let t = render_tile(&o, 1, 2, 4);
        // Clockwise: the original's bottom-left lands at the top-left.
        assert_eq!(px(&t, 0, 0), px(&o, 0, 1));
        assert_eq!(px(&t, 1, 0), px(&o, 0, 0));
    }
}
