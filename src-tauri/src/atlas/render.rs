//! Export rendering (design D5 of add-texture-atlas). Every tile is drawn from its original:
//! an exact pixel permutation at scale 1, otherwise the RIP export sampler
//! ([`crate::warp::warp`]) with its Jacobian-adaptive supersampling. Outputs start fully
//! transparent; tiles are composited source-over in manifest order and clipped per sector.

use super::model::{turn, udim, Fill, Filter, Frame, Manifest, Rect, Refused, Sector, Tile};
use rayon::prelude::*;
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

/// Scales `src` to `w x h` without smoothing: each output pixel takes the source pixel under
/// its centre, so pixel art keeps hard blocks (add-mcp-editing D6).
pub fn scale_nearest(src: &Image, w: u32, h: u32) -> Image {
    let (sw, sh) = (src.w as usize, src.h as usize);
    let (w, h) = (w as usize, h as usize);
    let mut out = vec![0u8; w * h * 4];
    if sw > 0 && sh > 0 {
        for y in 0..h {
            let sy = ((y * 2 + 1) * sh / (h * 2)).min(sh - 1);
            for x in 0..w {
                let sx = ((x * 2 + 1) * sw / (w * 2)).min(sw - 1);
                let s = (sy * sw + sx) * 4;
                out[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&src.rgba[s..s + 4]);
            }
        }
    }
    Image { w: w as u32, h: h as u32, rgba: out }
}

/// [`render_frame`] with the picture's [`Filter`]: `Nearest` scales the picture to the frame
/// size without smoothing first, then places it exactly (or through the tilt sampler).
pub fn render_frame_filtered(original: &Image, f: &Frame, filter: Filter) -> Image {
    if filter == Filter::Smooth || (original.w as i64, original.h as i64) == (f.w, f.h) {
        return render_frame(original, f);
    }
    let scaled = scale_nearest(original, f.w.max(1) as u32, f.h.max(1) as u32);
    render_frame(&scaled, f)
}

/// Sub-samples per axis for tilted tiles: 16 coverage levels on their edges.
pub const TILT_SUB: u32 = 4;

/// Renders a tile into its bounding box ([`Frame::rect`]). Without tilt this is
/// [`render_tile`]: exact quarter turns, filtered scaling. With tilt the box is sampled
/// through the inverse rotation, with at least [`TILT_SUB`]^2 sub-samples per pixel so the
/// edges are antialiased; outside the tile stays transparent.
pub fn render_frame(original: &Image, f: &Frame) -> Image {
    if f.tilt == 0.0 {
        let (rw, rh) = f.rotated_size();
        return render_tile(original, f.rot, rw as u32, rh as u32);
    }
    let r = f.rect();
    let c = f.centre();
    let (sx, sy) = (original.w as f64 / f.w as f64, original.h as f64 / f.h as f64);
    let to_source = |p: [f64; 2]| -> [f64; 2] {
        let l = turn([p[0] - c[0], p[1] - c[1]], -f.angle());
        [(l[0] + f.w as f64 / 2.0) * sx, (l[1] + f.h as f64 / 2.0) * sy]
    };
    let (x0, y0, x1, y1) = (r.x as f64, r.y as f64, r.right() as f64, r.bottom() as f64);
    let corners = [to_source([x0, y0]), to_source([x1, y0]), to_source([x1, y1]), to_source([x0, y1])];
    let src = Rgba { w: original.w, h: original.h, data: &original.rgba };
    let (w, h) = (r.w as u32, r.h as u32);
    let rgba = warp::warp_sub(&src, &corners, w, h, TILT_SUB).unwrap_or_else(|| vec![0; w as usize * h as usize * 4]);
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
    let tr = tile.rect();
    let Some(r) = clip.intersection(&tr) else { return };
    let ow = out.w as usize;
    let tw = img.w as usize;
    for y in r.y..r.bottom() {
        let oy = (y - origin.1) as usize;
        let ty = (y - tr.y) as usize;
        for x in r.x..r.right() {
            let o = (oy * ow + (x - origin.0) as usize) * 4;
            let t = (ty * tw + (x - tr.x) as usize) * 4;
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
        if let std::collections::hash_map::Entry::Vacant(e) = cache.entry(t.id) {
            let original = load(t)?;
            e.insert(render_frame_filtered(&original, &t.frame, t.filter()));
        }
        let img = &cache[&t.id];
        for c in cells {
            blit(&mut out, origin, c, t, img);
        }
    }
    if m.fill == Fill::Extend {
        // Per cell, so colour never crosses a sector border and every scope gives the same
        // pixels for a sector (design D3 of add-atlas-fine-work).
        for c in cells {
            let (cx, cy) = ((c.x - origin.0) as usize, (c.y - origin.1) as usize);
            let (cw, ch) = (c.w as usize, c.h as usize);
            let mut cell = Image::transparent(c.w as u32, c.h as u32);
            for y in 0..ch {
                let src = ((cy + y) * w as usize + cx) * 4;
                cell.rgba[y * cw * 4..(y + 1) * cw * 4].copy_from_slice(&out.rgba[src..src + cw * 4]);
            }
            extend_edges(&mut cell, m.edge);
            for y in 0..ch {
                let dst = ((cy + y) * w as usize + cx) * 4;
                out.rgba[dst..dst + cw * 4].copy_from_slice(&cell.rgba[y * cw * 4..(y + 1) * cw * 4]);
            }
        }
    }
    Ok(out)
}

/// Pixels with at least this alpha are tile colour that edge fill spreads.
pub const SEED_ALPHA: u8 = 128;

/// Edge fill (design D3): every pixel that is not fully opaque and lies within `edge` pixels
/// (Euclidean, between pixel centres) of a seed pixel is composited over the nearest seed's
/// colour, made opaque. Uses an exact nearest-seed transform: a column pass, then the
/// Felzenszwalb-Huttenlocher lower envelope per row.
pub fn extend_edges(img: &mut Image, edge: u32) {
    let (w, h) = (img.w as usize, img.h as usize);
    if w == 0 || h == 0 || edge == 0 {
        return;
    }
    const NONE: u32 = u32::MAX;
    let seed = |i: usize| img.rgba[i * 4 + 3] >= SEED_ALPHA;
    // Column pass: nearest seed row in the same column, for every pixel.
    let mut near_row = vec![NONE; w * h];
    let mut last = vec![NONE; w];
    for y in 0..h {
        for (x, above) in last.iter_mut().enumerate() {
            if seed(y * w + x) {
                *above = y as u32;
            }
        }
        near_row[y * w..(y + 1) * w].copy_from_slice(&last);
    }
    last.fill(NONE);
    for y in (0..h).rev() {
        let yu = y as u32;
        for (x, below) in last.iter_mut().enumerate() {
            let i = y * w + x;
            if img.rgba[i * 4 + 3] >= SEED_ALPHA {
                *below = yu;
            }
            let above = near_row[i];
            if *below != NONE && (above == NONE || *below - yu < yu - above) {
                near_row[i] = *below;
            }
        }
    }
    let limit = edge as i64 * edge as i64;
    let src = img.rgba.clone();
    img.rgba.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        // g(x) = squared vertical distance to the column's nearest seed; lower envelope of the
        // parabolas (x - q)^2 + g(q) gives the nearest seed column for every x.
        let g = |q: usize| -> Option<i64> {
            let r = near_row[y * w + q];
            (r != NONE).then(|| {
                let d = r as i64 - y as i64;
                d * d
            })
        };
        let qs: Vec<usize> = (0..w).filter(|&q| g(q).is_some()).collect();
        if qs.is_empty() {
            return;
        }
        let f = |q: usize| g(q).unwrap() as f64 + (q * q) as f64;
        let mut v: Vec<usize> = Vec::with_capacity(qs.len());
        let mut z: Vec<f64> = Vec::with_capacity(qs.len() + 1);
        for &q in &qs {
            loop {
                match v.last() {
                    None => {
                        v.push(q);
                        z.clear();
                        z.push(f64::NEG_INFINITY);
                        break;
                    }
                    Some(&p) => {
                        let s = (f(q) - f(p)) / (2.0 * (q as f64 - p as f64));
                        if s <= *z.last().unwrap() {
                            v.pop();
                            z.pop();
                            if v.is_empty() {
                                continue;
                            }
                        } else {
                            v.push(q);
                            z.push(s);
                            break;
                        }
                    }
                }
            }
        }
        let mut k = 0;
        for x in 0..w {
            while k + 1 < v.len() && z[k + 1] < x as f64 {
                k += 1;
            }
            let p = &mut row[x * 4..x * 4 + 4];
            if p[3] == 255 {
                continue;
            }
            let q = v[k];
            let dx = x as i64 - q as i64;
            let d2 = dx * dx + g(q).unwrap();
            if d2 > limit {
                continue;
            }
            let si = (near_row[y * w + q] as usize * w + q) * 4;
            let a = p[3] as f32 / 255.0;
            for c in 0..3 {
                p[c] = (p[c] as f32 * a + src[si + c] as f32 * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
            }
            p[3] = 255;
        }
    });
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

    fn frame(x: i64, y: i64, o: &Image, rot: u8) -> Frame {
        let mut f = Frame::from_rect(Rect { x, y, w: o.w as i64, h: o.h as i64 });
        for _ in 0..rot {
            f.quarter_turn(true);
        }
        // Keep the top-left of the turned box at (x, y), as the tests describe it.
        let r = f.rect();
        f.c2[0] += 2 * (x - r.x);
        f.c2[1] += 2 * (y - r.y);
        f
    }

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
        Tile { id, item: format!("{id}"), w0: o.w, h0: o.h, frame: frame(x, y, o, rot), source: Provenance::default(), history: Vec::new(), current: 0 }
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
    fn nearest_state_renders_hard_blocks() {
        let o = noise(64, 64);
        let f = Frame::from_rect(Rect { x: 0, y: 0, w: 512, h: 512 });
        let out = render_frame_filtered(&o, &f, Filter::Nearest);
        assert_eq!((out.w, out.h), (512, 512));
        for y in 0..512usize {
            for x in 0..512usize {
                let s = ((y / 8) * 64 + x / 8) * 4;
                assert_eq!(&out.rgba[(y * 512 + x) * 4..(y * 512 + x) * 4 + 4], &o.rgba[s..s + 4], "({x}, {y})");
            }
        }
        // Smooth is the old path, byte for byte.
        assert_eq!(render_frame_filtered(&o, &f, Filter::Smooth), render_frame(&o, &f));
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
    fn format_one_tiles_export_the_same_pixels() {
        // A 2.0.0 manifest: the tile is turned once, its turned box at (3, -40) is 6 x 9.
        let v1 = r#"{"format":1,"revision":1,"name":"x","sector_size":64,"padding":8,"sectors":[{"u":0,"v":0}],"active":null,"next_id":2,
            "tiles":[{"id":1,"item":"a","w0":9,"h0":6,"x":3,"y":-40,"w":6,"h":9,"rot":1}]}"#;
        let m = crate::atlas::model::parse_manifest(v1).unwrap();
        let o = noise(9, 6);
        let out = export(&m, &[Sector { u: 0, v: 0 }], Scope::Sector, &HashMap::from([(1, o.clone())]));
        let turned = rotate_exact(&o, 1);
        for y in 0..9 {
            for x in 0..6 {
                assert_eq!(px(&out[0].1, 3 + x, 24 + y), px(&turned, x, y));
            }
        }
    }

    #[test]
    fn freely_rotated_tile_has_soft_edges() {
        let mut m = Manifest::default();
        m.sector_size = 128;
        let o = img(40, 20, |_, _| [200, 50, 10, 255]);
        let mut t = tile(1, 40, -80, &o, 0);
        t.frame.turn_to(30.0);
        m.tiles.push(t);
        let out = export(&m, &[Sector { u: 0, v: 0 }], Scope::Sector, &HashMap::from([(1, o)]));
        let i = &out[0].1;
        let alphas: Vec<u8> = i.rgba.chunks(4).map(|p| p[3]).collect();
        assert!(alphas.iter().any(|&a| a > 0 && a < 255), "partial-alpha edge pixels");
        assert!(alphas.iter().filter(|&&a| a == 255).count() > 500, "solid inside");
        assert_eq!(px(i, 0, 0), [0, 0, 0, 0], "transparent around");
        // Colour stays the tile's colour on the soft edge (no dark fringe).
        for p in i.rgba.chunks(4).filter(|p| p[3] > 0) {
            assert!((p[0] as i32 - 200).abs() <= 1 && (p[1] as i32 - 50).abs() <= 1, "{p:?}");
        }
    }

    fn extend(m: &mut Manifest, edge: u32) {
        m.set_fill(Fill::Extend, edge).unwrap();
    }

    #[test]
    fn extend_by_16_pixels() {
        let mut m = Manifest::default();
        m.sector_size = 128;
        extend(&mut m, 16);
        let red = img(20, 10, |_, _| [255, 0, 0, 255]);
        m.tiles.push(tile(1, 50, -80, &red, 0)); // box x 50..70, y 48..58 in the sector image
        let out = export(&m, &[Sector { u: 0, v: 0 }], Scope::Sector, &HashMap::from([(1, red)]));
        let i = &out[0].1;
        assert_eq!(px(i, 70 + 15, 52), [255, 0, 0, 255], "16th pixel to the right");
        assert_eq!(px(i, 70 + 16, 52), [0, 0, 0, 0], "17th pixel stays transparent");
        assert_eq!(px(i, 50 - 16, 52), [255, 0, 0, 255]);
        assert_eq!(px(i, 50 - 17, 52), [0, 0, 0, 0]);
        assert_eq!(px(i, 60, 48 - 16), [255, 0, 0, 255]);
        assert_eq!(px(i, 60, 48 - 17), [0, 0, 0, 0]);
        // Diagonal beyond the radius stays empty: (12, 12) from the corner is 17 px away.
        assert_eq!(px(i, 70 + 11, 58 + 11), [0, 0, 0, 0]);
    }

    #[test]
    fn two_tiles_meet_halfway() {
        let mut m = Manifest::default();
        m.sector_size = 128;
        extend(&mut m, 16);
        let red = img(10, 10, |_, _| [255, 0, 0, 255]);
        let blue = img(10, 10, |_, _| [0, 0, 255, 255]);
        m.tiles.push(tile(1, 20, -100, &red, 0)); // x 20..30
        m.tiles.push(tile(2, 38, -100, &blue, 0)); // x 38..48, gap 30..38
        let out = export(&m, &[Sector { u: 0, v: 0 }], Scope::Sector, &HashMap::from([(1, red), (2, blue)]));
        let i = &out[0].1;
        for x in 30..34 {
            assert_eq!(px(i, x, 32), [255, 0, 0, 255], "x {x}");
        }
        for x in 34..38 {
            assert_eq!(px(i, x, 32), [0, 0, 255, 255], "x {x}");
        }
    }

    #[test]
    fn soft_edge_becomes_solid() {
        let mut m = Manifest::default();
        m.sector_size = 128;
        extend(&mut m, 16);
        let o = img(40, 20, |_, _| [200, 50, 10, 255]);
        let mut t = tile(1, 40, -80, &o, 0);
        t.frame.turn_to(30.0);
        let r = t.rect();
        m.tiles.push(t);
        let out = export(&m, &[Sector { u: 0, v: 0 }], Scope::Sector, &HashMap::from([(1, o)]));
        let i = &out[0].1;
        // Every visible pixel is solid tile colour: no soft or dark fringe anywhere.
        for p in i.rgba.chunks(4).filter(|p| p[3] > 0) {
            assert_eq!(p[3], 255, "{p:?}");
            assert!((p[0] as i32 - 200).abs() <= 1 && (p[1] as i32 - 50).abs() <= 1, "{p:?}");
        }
        // And the tile's own edge pixels (inside its box, next to it) are covered.
        let c = [(r.x + r.w / 2) as u32, (r.y + 128 + r.h / 2) as u32];
        assert_eq!(px(i, c[0], c[1])[3], 255);
    }

    #[test]
    fn edge_fill_never_crosses_a_sector_border_and_scopes_agree() {
        let mut m = Manifest::default();
        sectors(&mut m, 64);
        extend(&mut m, 16);
        let red = img(10, 10, |_, _| [255, 0, 0, 255]);
        m.tiles.push(tile(1, 50, -40, &red, 0)); // ends at x = 60, 4 px before the border at 64
        let originals = HashMap::from([(1, red)]);
        let sel = [Sector { u: 0, v: 0 }, Sector { u: 1, v: 0 }];
        let set = export(&m, &sel, Scope::GroupSet, &originals);
        let right = &set[1].1;
        assert!(right.rgba.iter().all(|&b| b == 0), "no colour in 1002");
        assert_eq!(px(&set[0].1, 63, 30), [255, 0, 0, 255], "1001 filled up to its border");
        let sheet = &export(&m, &sel, Scope::GroupFile, &originals)[0].1;
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(px(sheet, x, y), px(&set[0].1, x, y));
                assert_eq!(px(sheet, 64 + x, y), px(right, x, y));
            }
        }
    }

    #[test]
    fn empty_sector_stays_transparent_with_extend() {
        let mut m = Manifest::default();
        m.sector_size = 64;
        extend(&mut m, 16);
        let e = export(&m, &[Sector { u: 0, v: 0 }], Scope::Sector, &HashMap::new());
        assert!(e[0].1.rgba.iter().all(|&b| b == 0));
    }

    /// Task 3.3 timings: `cargo test --release --lib -- --ignored --nocapture export_timings`.
    #[test]
    #[ignore]
    fn export_timings() {
        use std::time::Instant;
        let mut m = Manifest::default();
        m.sector_size = 4096;
        m.add_sector(Sector { u: 0, v: 0 }, Side::Right).unwrap();
        m.add_sector(Sector { u: 0, v: 0 }, Side::Up).unwrap();
        m.add_sector(Sector { u: 1, v: 0 }, Side::Up).unwrap();
        let mut originals = HashMap::new();
        for k in 0..20u64 {
            let o = noise(700, 450);
            let mut t = tile(k + 1, 100 + (k as i64 % 5) * 780, -4000 + (k as i64 / 5) * 980, &o, 0);
            if k % 4 == 0 {
                t.frame.turn_to(17.0 + k as f64);
            }
            m.tiles.push(t);
            originals.insert(k + 1, o);
        }
        let one = [Sector { u: 0, v: 0 }];
        for fill in [Fill::Transparent, Fill::Extend] {
            m.set_fill(fill, 16).unwrap();
            let t0 = Instant::now();
            export(&m, &one, Scope::Sector, &originals);
            println!("SECTOR 4096, 20 tiles (5 tilted), {fill:?}: {:?}", t0.elapsed());
        }
        let t0 = Instant::now();
        export(&m, &m.sectors.clone(), Scope::GroupFile, &originals);
        println!("GROUP FILE 8192^2, Extend: {:?}", t0.elapsed());
    }

    #[test]
    fn quarter_turns_stay_exact_through_frames() {
        let o = noise(7, 3);
        for rot in 0..4u8 {
            let f = frame(0, 0, &o, rot);
            assert_eq!(render_frame(&o, &f), rotate_exact(&o, rot), "rot {rot}");
        }
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
