//! Workspace edits and their inverses (design D6 of add-atlas-fine-work). Every forward edit
//! returns the edits that undo it, computed from the manifest before it changes, so undo is an
//! ordinary edit applied to the workspace as it is now (never a snapshot that would drop a
//! tile a ShareX capture added in between).

use super::model::{Corner, Fill, Frame, Guide, Manifest, Refused, Sector, Side, Tile};
use serde::{Deserialize, Serialize};

/// One edit of a workspace. The last five exist to undo the others.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Edit {
    SetSize { size: u32 },
    SetPadding { padding: u32 },
    AddSector { sector: Sector, side: Side },
    RemoveSector { sector: Sector },
    SetActive { sector: Sector },
    Move { ids: Vec<u64>, dx: i64, dy: i64 },
    Scale { id: u64, w: i64, anchor: Corner },
    Rotate { ids: Vec<u64>, clockwise: bool },
    Turn { id: u64, angle: f64 },
    SetPivot { id: u64, pivot: Option<[i64; 2]> },
    Delete { ids: Vec<u64> },
    SetGuideView { show: Option<bool>, opacity: Option<u8> },
    SetFill { fill: Fill, edge: u32 },
    RemoveGuide { sector: Sector },
    SetFrame { id: u64, frame: Frame },
    RestoreTile { index: usize, tile: Tile },
    AddSectors { cells: Vec<Sector> },
    RemoveSectors { cells: Vec<Sector> },
    SetGuide { sector: Sector, guide: Option<Guide> },
    /// Paste History pictures as new tiles (add-mcp-editing D8); undone by a Delete.
    PasteStates { tiles: Vec<super::history::PasteTile> },
}

/// What applying an edit did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Applied {
    /// Edits that undo this one, in the order to apply them. Empty for settings, which are
    /// not part of the history.
    pub inverse: Vec<Edit>,
    /// Status label of the step, e.g. "move 3 tiles".
    pub label: String,
    /// Tiles removed by a Delete.
    pub removed: usize,
    /// Items that left the manifest but may come back through undo.
    pub released: Vec<String>,
}

fn plural(n: usize, one: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {one}s")
    }
}

/// UDIM number of a sector in the grid, as the canvas labels it.
pub fn sector_label(m: &Manifest, s: Sector) -> u32 {
    let o = m.grid_origin();
    (1001 + (s.u - o.u) + 10 * (s.v - o.v)).max(0) as u32
}

fn frames(m: &Manifest, ids: &[u64]) -> Vec<Edit> {
    ids.iter().filter_map(|&id| m.tile(id).map(|t| Edit::SetFrame { id, frame: t.frame.clone() })).collect()
}

/// Applies one edit, all or nothing for a single object; a refusal leaves `m` unchanged only
/// when the edit touches one object (callers that need atomicity apply to a clone).
pub fn apply(m: &mut Manifest, e: Edit) -> Result<Applied, Refused> {
    let mut a = Applied::default();
    match e {
        Edit::SetSize { size } => m.set_sector_size(size)?,
        Edit::SetPadding { padding } => m.set_padding(padding)?,
        Edit::SetActive { sector } => m.set_active(sector)?,
        Edit::SetGuideView { show, opacity } => m.set_guide_view(show, opacity)?,
        Edit::SetFill { fill, edge } => m.set_fill(fill, edge)?,
        Edit::AddSector { sector, side } => {
            let n = m.add_sector(sector, side)?;
            a.inverse = vec![Edit::RemoveSectors { cells: vec![n] }];
            a.label = "add sector".into();
        }
        Edit::RemoveSector { sector } => {
            a = remove_sectors(m, &[sector], false)?;
            a.label = "remove sector".into();
        }
        Edit::Move { ids, dx, dy } => {
            a.inverse = frames(m, &ids);
            m.move_tiles(&ids, dx, dy)?;
            a.label = format!("move {}", plural(ids.len(), "tile"));
        }
        Edit::Scale { id, w, anchor } => {
            a.inverse = frames(m, &[id]);
            m.scale_tile(id, w, anchor)?;
            a.label = "scale tile".into();
        }
        Edit::Rotate { ids, clockwise } => {
            a.inverse = frames(m, &ids);
            m.rotate_tiles(&ids, clockwise)?;
            a.label = format!("rotate {}", plural(ids.len(), "tile"));
        }
        Edit::Turn { id, angle } => {
            a.inverse = frames(m, &[id]);
            m.turn_tile(id, angle)?;
            a.label = "rotate tile".into();
        }
        Edit::SetPivot { id, pivot } => {
            a.inverse = frames(m, &[id]);
            m.set_pivot(id, pivot)?;
            a.label = if pivot.is_some() { "move pivot".into() } else { "reset pivot".into() };
        }
        Edit::SetFrame { id, frame } => {
            a.inverse = frames(m, &[id]);
            m.set_frame(id, frame)?;
            a.label = "tile placement".into();
        }
        Edit::Delete { ids } => {
            let gone: Vec<(usize, Tile)> =
                m.tiles.iter().enumerate().filter(|(_, t)| ids.contains(&t.id)).map(|(i, t)| (i, t.clone())).collect();
            a.removed = m.delete_tiles(&ids);
            a.released = gone.iter().flat_map(|(_, t)| t.items()).collect();
            a.inverse = gone.into_iter().map(|(index, tile)| Edit::RestoreTile { index, tile }).collect();
            a.label = format!("delete {}", plural(a.removed, "tile"));
        }
        Edit::PasteStates { tiles } => {
            for t in &tiles {
                if !m.items().contains(t.state.item.as_str()) {
                    return Err(Refused("The copied picture is no longer in this workspace".into()));
                }
            }
            let ids = m.paste_states(tiles)?;
            a.label = format!("paste {}", plural(ids.len(), "tile"));
            a.inverse = vec![Edit::Delete { ids }];
        }
        Edit::RestoreTile { index, tile } => {
            if m.tile(tile.id).is_some() {
                return Err(Refused(format!("Tile {} is already there", tile.id)));
            }
            a.inverse = vec![Edit::Delete { ids: vec![tile.id] }];
            let i = index.min(m.tiles.len());
            m.next_id = m.next_id.max(tile.id + 1);
            m.tiles.insert(i, tile);
            a.label = "restore tile".into();
        }
        Edit::RemoveGuide { sector } => {
            let old = m.set_guide(sector, None)?;
            let Some(old) = old else { return Err(Refused("That sector has no guide".into())) };
            a.released = vec![old.item.clone()];
            a.inverse = vec![Edit::SetGuide { sector, guide: Some(old) }];
            a.label = "remove guide".into();
        }
        Edit::SetGuide { sector, guide } => {
            let old = m.set_guide(sector, guide)?;
            a.released = old.iter().map(|g| g.item.clone()).collect();
            a.inverse = vec![Edit::SetGuide { sector, guide: old }];
            a.label = "guide".into();
        }
        Edit::AddSectors { cells } => {
            let mut added = Vec::new();
            for s in cells {
                if m.has_sector(s) {
                    return Err(Refused(format!("Sector {} is already there", sector_label(m, s))));
                }
                m.sectors.push(s);
                added.push(s);
            }
            a.inverse = vec![Edit::RemoveSectors { cells: added }];
            a.label = "add sector".into();
        }
        Edit::RemoveSectors { cells } => {
            a = remove_sectors(m, &cells, true)?;
            a.label = "remove sector".into();
        }
    }
    Ok(a)
}

fn remove_sectors(m: &mut Manifest, cells: &[Sector], undoing: bool) -> Result<Applied, Refused> {
    let mut a = Applied::default();
    let mut back = Vec::new();
    let mut guides = Vec::new();
    for &s in cells {
        let guide = m.guide(s).cloned();
        if let Err(Refused(why)) = m.remove_sector(s) {
            if undoing && m.tiles_on(s).next().is_some() {
                return Err(Refused(format!("sector {} kept, a tile arrived there", sector_label(m, s))));
            }
            return Err(Refused(why));
        }
        back.push(s);
        if let Some(g) = guide {
            a.released.push(g.item.clone());
            guides.push(Edit::SetGuide { sector: s, guide: Some(g) });
        }
    }
    a.inverse = std::iter::once(Edit::AddSectors { cells: back }).chain(guides).collect();
    Ok(a)
}

/// Undo or redo: applies `edits` best effort. Each edit is applied to a copy and kept only if
/// it succeeds; a refused one is skipped and its reason collected. Returns the inverse of what
/// was applied (in the order to apply it) and the skipped reasons.
pub fn apply_steps(m: &mut Manifest, edits: Vec<Edit>) -> (Vec<Edit>, Vec<String>, Vec<String>) {
    let mut inverses: Vec<Vec<Edit>> = Vec::new();
    let mut skipped = Vec::new();
    let mut released = Vec::new();
    for e in edits {
        let mut trial = m.clone();
        match apply(&mut trial, e) {
            Ok(a) => {
                *m = trial;
                released.extend(a.released);
                inverses.push(a.inverse);
            }
            Err(Refused(why)) => skipped.push(why),
        }
    }
    (inverses.into_iter().rev().flatten().collect(), skipped, released)
}

/// The edits that undo a guide LOAD: restore the replaced guides, then remove each sector the
/// load created, one edit per sector so a sector that received a tile meanwhile is skipped
/// alone.
pub fn undo_guide_load(replaced: &[(Sector, Option<Guide>)], created: &[Sector]) -> Vec<Edit> {
    let mut v: Vec<Edit> = replaced.iter().map(|(s, g)| Edit::SetGuide { sector: *s, guide: g.clone() }).collect();
    v.extend(created.iter().map(|&s| Edit::RemoveSectors { cells: vec![s] }));
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::model::{Provenance, Rect};

    fn s(u: i32, v: i32) -> Sector {
        Sector { u, v }
    }

    fn tile(id: u64, x: i64, y: i64) -> Tile {
        Tile { id, item: format!("{id}.png"), w0: 40, h0: 20, frame: Frame::from_rect(Rect { x, y, w: 40, h: 20 }), source: Provenance::default(), history: Vec::new(), current: 0 }
    }

    fn base() -> Manifest {
        let mut m = Manifest::default();
        m.sector_size = 512;
        for (i, x) in [10, 100, 200].into_iter().enumerate() {
            m.tiles.push(tile(i as u64 + 1, x, -300));
        }
        m.next_id = 4;
        m.add_sector(s(0, 0), Side::Right).unwrap();
        m.guides.push(Guide { sector: s(1, 0), item: "g.png".into(), w: 8, h: 8 });
        m
    }

    /// Applying an edit, then its inverse, gives back the manifest.
    fn round_trip(e: Edit) {
        let start = base();
        let mut m = start.clone();
        let a = apply(&mut m, e.clone()).unwrap();
        assert!(!a.inverse.is_empty(), "{e:?} has an inverse");
        let (_, skipped, _) = apply_steps(&mut m, a.inverse);
        assert!(skipped.is_empty(), "{e:?}: {skipped:?}");
        assert_eq!(m.tiles, start.tiles, "{e:?}");
        let mut sa = m.sectors.clone();
        let mut sb = start.sectors.clone();
        sa.sort_by_key(|c| (c.v, c.u));
        sb.sort_by_key(|c| (c.v, c.u));
        assert_eq!(sa, sb, "{e:?}");
        assert_eq!(m.guides, start.guides, "{e:?}");
    }

    #[test]
    fn every_step_has_an_exact_inverse() {
        round_trip(Edit::Move { ids: vec![1, 3], dx: 17, dy: -4 });
        round_trip(Edit::Scale { id: 2, w: 100, anchor: Corner::BottomRight });
        round_trip(Edit::Rotate { ids: vec![1, 2], clockwise: true });
        round_trip(Edit::Turn { id: 3, angle: 33.0 });
        round_trip(Edit::SetPivot { id: 1, pivot: Some([3, 4]) });
        round_trip(Edit::Delete { ids: vec![1, 3] });
        round_trip(Edit::AddSector { sector: s(0, 0), side: Side::Up });
        round_trip(Edit::RemoveSector { sector: s(1, 0) });
        round_trip(Edit::RemoveGuide { sector: s(1, 0) });
        round_trip(Edit::SetGuide { sector: s(0, 0), guide: Some(Guide { sector: s(0, 0), item: "n.png".into(), w: 1, h: 1 }) });
    }

    #[test]
    fn delete_restores_order_and_releases_items() {
        let mut m = base();
        let a = apply(&mut m, Edit::Delete { ids: vec![1, 3] }).unwrap();
        assert_eq!(a.removed, 2);
        assert_eq!(a.released, vec!["1.png".to_string(), "3.png".to_string()]);
        apply_steps(&mut m, a.inverse);
        assert_eq!(m.tiles.iter().map(|t| t.id).collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn settings_have_no_inverse() {
        let mut m = base();
        assert!(apply(&mut m, Edit::SetGuideView { show: Some(false), opacity: Some(20) }).unwrap().inverse.is_empty());
        assert!(apply(&mut m, Edit::SetFill { fill: Fill::Extend, edge: 8 }).unwrap().inverse.is_empty());
        assert!(apply(&mut m, Edit::SetPadding { padding: 4 }).unwrap().inverse.is_empty());
        assert!(apply(&mut m, Edit::SetActive { sector: s(0, 0) }).unwrap().inverse.is_empty());
    }

    #[test]
    fn sector_that_received_a_tile_is_kept() {
        let mut m = base();
        let a = apply(&mut m, Edit::AddSector { sector: s(1, 0), side: Side::Right }).unwrap();
        // A ShareX tile lands in the new sector (u = 2).
        m.tiles.push(tile(9, 1100, -300));
        let (inv, skipped, _) = apply_steps(&mut m, a.inverse);
        assert!(inv.is_empty());
        assert_eq!(skipped.len(), 1);
        assert!(skipped[0].contains("1003") && skipped[0].contains("tile arrived"), "{skipped:?}");
        assert!(m.has_sector(s(2, 0)) && m.tile(9).is_some());
    }

    #[test]
    fn tile_deleted_elsewhere_is_skipped() {
        let mut m = base();
        let a = apply(&mut m, Edit::Move { ids: vec![2], dx: 5, dy: 5 }).unwrap();
        m.delete_tiles(&[2]);
        let before = m.clone();
        let (_, skipped, _) = apply_steps(&mut m, a.inverse);
        assert_eq!(skipped, vec!["Tile 2 no longer exists".to_string()]);
        assert_eq!(m, before);
    }

    #[test]
    fn capture_between_edits_survives_undo() {
        let mut m = base();
        let a = apply(&mut m, Edit::Move { ids: vec![1], dx: 50, dy: 0 }).unwrap();
        m.tiles.push(tile(9, 300, -100)); // ShareX
        apply_steps(&mut m, a.inverse);
        assert!(m.tile(9).is_some());
        assert_eq!(m.tile(1).unwrap().rect().x, 10);
    }

    #[test]
    fn redo_is_the_inverse_of_the_undo() {
        let mut m = base();
        let a = apply(&mut m, Edit::Rotate { ids: vec![1], clockwise: true }).unwrap();
        let after = m.clone();
        let (redo, _, _) = apply_steps(&mut m, a.inverse);
        let (undo_again, _, _) = apply_steps(&mut m, redo);
        assert_eq!(m.tiles, after.tiles);
        apply_steps(&mut m, undo_again);
        assert_eq!(m.tiles, base().tiles);
    }

    #[test]
    fn undo_a_guide_set_load() {
        use crate::atlas::model::GuideTarget;
        let mut m = Manifest::default();
        let start = m.clone();
        let loads: Vec<_> = (1001..=1004).map(|n| (GuideTarget::Udim(n), format!("{n}.png"), 8, 8)).collect();
        let rep = m.place_guides(&loads).unwrap();
        let (_, skipped, released) = apply_steps(&mut m, undo_guide_load(&rep.replaced, &rep.created));
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(m.sectors, start.sectors);
        assert!(m.guides.is_empty());
        assert_eq!(released.len(), 4, "loaded guide images are held for redo");
    }

    #[test]
    fn set_load_undo_keeps_only_the_sector_that_received_a_tile() {
        use crate::atlas::model::GuideTarget;
        let mut m = Manifest::default();
        m.sector_size = 512;
        let loads: Vec<_> = (1001..=1004).map(|n| (GuideTarget::Udim(n), format!("{n}.png"), 8, 8)).collect();
        let rep = m.place_guides(&loads).unwrap();
        m.tiles.push(tile(9, 2 * 512 + 50, -300)); // a capture lands in 1003
        let (_, skipped, _) = apply_steps(&mut m, undo_guide_load(&rep.replaced, &rep.created));
        assert_eq!(skipped.len(), 1, "{skipped:?}");
        assert!(skipped[0].contains("1003"));
        let mut cells = m.sectors.clone();
        cells.sort_by_key(|c| c.u);
        assert_eq!(cells, vec![s(0, 0), s(2, 0)], "1002 and 1004 removed, 1003 kept");
        assert!(m.guides.is_empty());
    }
}
