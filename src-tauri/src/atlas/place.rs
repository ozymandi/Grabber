//! Automatic placement of new tiles (design D4 of add-texture-atlas): append-only, the
//! existing tiles never move. A deterministic candidate search over the free space respects
//! whatever the user arranged by hand.

use super::model::{Manifest, Rect, Sector};

/// Where a new tile went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub rect: Rect,
    /// The sector it landed in, or `None` for the tray (canvas outside all sectors).
    pub sector: Option<Sector>,
    /// True when the tile had to be scaled down to fit a sector.
    pub fitted: bool,
}

/// First top-left position in `area` where a `w x h` rect overlaps none of `occupied`.
/// Candidates are the area's top-left and the right/bottom edges of occupied rects; they are
/// tried top to bottom, then left to right.
pub fn find_spot(area: Rect, occupied: &[Rect], w: i64, h: i64) -> Option<(i64, i64)> {
    if w > area.w || h > area.h {
        return None;
    }
    let mut xs: Vec<i64> = std::iter::once(area.x).chain(occupied.iter().map(|r| r.right())).collect();
    let mut ys: Vec<i64> = std::iter::once(area.y).chain(occupied.iter().map(|r| r.bottom())).collect();
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    for &y in &ys {
        for &x in &xs {
            let r = Rect { x, y, w, h };
            if area.contains(&r) && !occupied.iter().any(|o| o.intersects(&r)) {
                return Some((x, y));
            }
        }
    }
    None
}

/// Size a `w0 x h0` original takes on arrival: full size, or scaled down uniformly (sizes
/// rounded down) so it fits a sector minus twice the padding.
pub fn arrival_size(w0: u32, h0: u32, sector: u32, padding: u32) -> (i64, i64, bool) {
    let room = (sector as i64 - 2 * padding as i64).max(1);
    let (w, h) = (w0 as i64, h0 as i64);
    if w <= room && h <= room {
        return (w, h, false);
    }
    let s = (room as f64 / w as f64).min(room as f64 / h as f64);
    let fw = ((w as f64 * s).floor() as i64).clamp(1, room);
    let fh = ((h as f64 * s).floor() as i64).clamp(1, room);
    (fw, fh, true)
}

/// Places a new `w0 x h0` tile: the active sector first, then the other sectors in UDIM
/// order, never across a sector border; otherwise the tray beside the grid.
pub fn place_new(m: &Manifest, w0: u32, h0: u32) -> Placement {
    let size = m.sector_size;
    let pad = m.padding as i64;
    let (w, h, fitted) = arrival_size(w0, h0, size, m.padding);
    let mut order = Vec::new();
    if let Some(a) = m.active.filter(|a| m.has_sector(*a)) {
        order.push(a);
    }
    order.extend(m.sectors_udim_order().into_iter().filter(|s| Some(*s) != m.active));
    for s in order {
        let sr = s.rect(size);
        let area = sr.inflate(-pad);
        let occupied: Vec<Rect> = m.tiles.iter().map(|t| t.rect()).filter(|r| r.intersects(&sr)).map(|r| r.inflate(pad)).collect();
        if let Some((x, y)) = find_spot(area, &occupied, w, h) {
            return Placement { rect: Rect { x, y, w, h }, sector: Some(s), fitted };
        }
    }
    // Tray: to the right of the grid, starting at its top, clear of every tile and sector.
    let g = m.grid_bounds();
    let gap = (size / 8) as i64;
    let area = Rect { x: g.right() + gap, y: g.y, w: i64::MAX / 4, h: i64::MAX / 4 };
    let mut occupied: Vec<Rect> = m.tiles.iter().map(|t| t.rect().inflate(pad)).collect();
    occupied.extend(m.sectors.iter().map(|s| s.rect(size)));
    let (x, y) = find_spot(area, &occupied, w, h).unwrap_or((area.x, area.y));
    Placement { rect: Rect { x, y, w, h }, sector: None, fitted }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::model::{Provenance, Side, Tile};

    fn tile(id: u64, r: Rect) -> Tile {
        Tile { id, item: format!("{id}.png"), w0: r.w as u32, h0: r.h as u32, x: r.x, y: r.y, w: r.w, h: r.h, rot: 0, source: Provenance::default() }
    }

    fn add(m: &mut Manifest, p: Placement) {
        let id = m.next_id;
        m.next_id += 1;
        m.tiles.push(tile(id, p.rect));
    }

    fn s(u: i32, v: i32) -> Sector {
        Sector { u, v }
    }

    #[test]
    fn tile_fits_in_active_sector_without_moving_others() {
        let mut m = Manifest::default(); // 2048, padding 8
        let first = place_new(&m, 1024, 1024);
        add(&mut m, first);
        let before = m.tiles[0].clone();
        let p = place_new(&m, 512, 512);
        assert_eq!(p.sector, Some(s(0, 0)));
        let sr = s(0, 0).rect(2048);
        assert!(sr.inflate(-8).contains(&p.rect), "inside the sector with padding");
        assert!(!p.rect.intersects(&before.rect().inflate(8)), "padding kept from the first tile");
        add(&mut m, p);
        assert_eq!(m.tiles[0], before, "first tile not moved");
    }

    #[test]
    fn full_active_sector_goes_to_next_in_udim_order() {
        let mut m = Manifest::default();
        m.sector_size = 1024;
        m.add_sector(s(0, 0), Side::Right).unwrap();
        m.set_active(s(0, 0)).unwrap();
        let big = place_new(&m, 1000, 1000);
        assert_eq!(big.sector, Some(s(0, 0)));
        add(&mut m, big);
        let p = place_new(&m, 300, 300);
        assert_eq!(p.sector, Some(s(1, 0)));
    }

    #[test]
    fn active_sector_is_tried_first() {
        let mut m = Manifest::default();
        m.add_sector(s(0, 0), Side::Right).unwrap();
        m.set_active(s(1, 0)).unwrap();
        assert_eq!(place_new(&m, 100, 100).sector, Some(s(1, 0)));
    }

    #[test]
    fn large_capture_is_fitted_down() {
        let mut m = Manifest::default();
        m.sector_size = 1024;
        let p = place_new(&m, 1500, 900);
        assert!(p.fitted);
        assert_eq!((p.rect.w, p.rect.h), (1008, 604));
        assert_eq!(p.sector, Some(s(0, 0)));
        // The original keeps its size: add_tile stores w0 x h0 = 1500 x 900 (see ops tests).
    }

    #[test]
    fn all_sectors_full_goes_to_tray() {
        let mut m = Manifest::default();
        m.sector_size = 512;
        let p = place_new(&m, 490, 490);
        add(&mut m, p);
        let before = m.tiles.clone();
        let t = place_new(&m, 200, 100);
        assert_eq!(t.sector, None);
        let g = m.grid_bounds();
        assert!(t.rect.x >= g.right(), "beside the grid");
        assert!(!m.sectors.iter().any(|c| c.rect(512).intersects(&t.rect)), "outside every sector");
        add(&mut m, t);
        let t2 = place_new(&m, 200, 100);
        assert_eq!(t2.sector, None);
        assert!(!t2.rect.intersects(&t.rect), "tray tiles do not overlap");
        assert_eq!(&m.tiles[..1], &before[..], "sector content unchanged");
    }

    #[test]
    fn automatic_placement_never_crosses_a_border() {
        let mut m = Manifest::default();
        m.sector_size = 512;
        m.add_sector(s(0, 0), Side::Right).unwrap();
        m.add_sector(s(0, 0), Side::Up).unwrap();
        for i in 0..40 {
            let p = place_new(&m, 60 + (i * 37) % 200, 40 + (i * 53) % 180);
            if let Some(sec) = p.sector {
                assert!(sec.rect(512).contains(&p.rect), "tile {i} inside its sector");
                for other in &m.sectors {
                    if *other != sec {
                        assert!(!other.rect(512).intersects(&p.rect), "tile {i} does not touch {other:?}");
                    }
                }
            }
            for t in &m.tiles {
                assert!(!t.rect().intersects(&p.rect), "tile {i} overlaps nothing");
            }
            add(&mut m, p);
        }
    }

    #[test]
    fn manual_tile_across_border_is_respected() {
        let mut m = Manifest::default();
        m.sector_size = 512;
        m.add_sector(s(0, 0), Side::Right).unwrap();
        // User dragged a tile onto the border between 1001 and 1002, at the top.
        m.tiles.push(tile(1, Rect { x: 400, y: -512, w: 224, h: 200 }));
        m.next_id = 2;
        let p = place_new(&m, 100, 100);
        assert!(!p.rect.intersects(&m.tiles[0].rect().inflate(8)));
    }
}
