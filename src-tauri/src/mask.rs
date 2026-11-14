//! Shape mask for export (design D3 of add-quad-shapes): 4 x 4 supersampled, nonzero-winding
//! coverage of a closed polygon in output pixels, multiplied into straight alpha.

use crate::geom::Pt;

/// Samples per pixel side; a pixel has `SS * SS` samples.
const SS: usize = 4;

/// Per-pixel coverage of `poly` over a `w x h` grid, from 0 to `SS * SS`.
pub fn coverage(poly: &[Pt], w: u32, h: u32) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let mut cov = vec![0u8; w * h];
    if poly.len() < 3 || w == 0 || h == 0 {
        return cov;
    }
    // Edges as (x0, y0, x1, y1, dir), only the non-horizontal ones, sorted by their top.
    let mut edges: Vec<(f64, f64, f64, f64, i32)> = Vec::with_capacity(poly.len());
    for i in 0..poly.len() {
        let p = poly[i];
        let q = poly[(i + 1) % poly.len()];
        if p[1] == q[1] || !p.iter().chain(q.iter()).all(|v| v.is_finite()) {
            continue;
        }
        if p[1] < q[1] {
            edges.push((p[0], p[1], q[0], q[1], 1));
        } else {
            edges.push((q[0], q[1], p[0], p[1], -1));
        }
    }
    edges.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut active: Vec<usize> = Vec::new();
    let mut next = 0;
    let mut xs: Vec<(f64, i32)> = Vec::new();
    let scale = SS as f64;
    for py in 0..h {
        for j in 0..SS {
            let ys = py as f64 + (j as f64 + 0.5) / scale;
            while next < edges.len() && edges[next].1 <= ys {
                active.push(next);
                next += 1;
            }
            active.retain(|&e| edges[e].3 > ys);
            xs.clear();
            for &e in &active {
                let (x0, y0, x1, y1, dir) = edges[e];
                if y0 <= ys {
                    xs.push((x0 + (ys - y0) * (x1 - x0) / (y1 - y0), dir));
                }
            }
            if xs.len() < 2 {
                continue;
            }
            xs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let row = &mut cov[py * w..(py + 1) * w];
            let mut wind = 0;
            for k in 0..xs.len() - 1 {
                wind += xs[k].1;
                if wind != 0 {
                    add_span(row, xs[k].0, xs[k + 1].0, w);
                }
            }
        }
    }
    cov
}

/// Adds the samples of one sample row whose x lies in `[xa, xb)` to the row's pixels.
fn add_span(row: &mut [u8], xa: f64, xb: f64, w: usize) {
    let scale = SS as f64;
    // Sample k sits at x = (k + 0.5) / SS; it is inside when xa <= x < xb.
    let first = ((xa * scale - 0.5).ceil().max(0.0)) as usize;
    let end = ((xb * scale - 0.5).ceil().max(0.0) as usize).min(w * SS);
    if first >= end {
        return;
    }
    let (p0, p1) = (first / SS, (end - 1) / SS);
    if p0 == p1 {
        row[p0] += (end - first) as u8;
        return;
    }
    row[p0] += (SS * (p0 + 1) - first) as u8;
    for v in &mut row[p0 + 1..p1] {
        *v += SS as u8;
    }
    row[p1] += (end - SS * p1) as u8;
}

/// Multiplies the alpha of straight-alpha RGBA8 `rgba` by the coverage of `poly`.
pub fn apply(rgba: &mut [u8], w: u32, h: u32, poly: &[Pt]) {
    let cov = coverage(poly, w, h);
    let full = (SS * SS) as u32;
    for (px, c) in rgba.chunks_exact_mut(4).zip(cov) {
        let c = c as u32;
        if c < full {
            px[3] = ((px[3] as u32 * c + full / 2) / full) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::{outline, Node, Shape};

    fn alpha_after(shape: &Shape, w: u32, h: u32) -> Vec<u8> {
        let mut rgba = vec![255u8; (w * h * 4) as usize];
        apply(&mut rgba, w, h, &outline(shape, w as f64, h as f64));
        rgba.chunks(4).map(|p| p[3]).collect()
    }

    #[test]
    fn rect_outline_covers_everything() {
        assert!(alpha_after(&Shape::Rect, 37, 21).iter().all(|&a| a == 255));
    }

    #[test]
    fn circle_has_solid_centre_clear_corners_soft_edge() {
        let a = alpha_after(&Shape::Circle, 64, 64);
        let at = |x: usize, y: usize| a[y * 64 + x];
        assert_eq!(at(32, 32), 255);
        assert_eq!(at(0, 0), 0);
        assert_eq!(at(63, 63), 0);
        assert!(a.iter().any(|&v| v > 0 && v < 255), "partial-alpha edge pixels");
        // Covered area matches the disc: pi * 32^2 = 3216.99 px.
        let area: f64 = a.iter().map(|&v| v as f64 / 255.0).sum();
        // The chords run inside the circle by up to 0.05 px, so the area is a little smaller.
        assert!((area - std::f64::consts::PI * 1024.0).abs() < 8.0, "{area}");
    }

    #[test]
    fn octagon_corners_are_clear() {
        let a = alpha_after(&Shape::Ngon { n: 8 }, 100, 100);
        for (x, y) in [(0, 0), (99, 0), (0, 99), (99, 99)] {
            assert_eq!(a[y * 100 + x], 0, "corner {x},{y}");
        }
        assert_eq!(a[50 * 100 + 50], 255);
        assert_eq!(a[50 * 100], 255, "flat side on the left edge");
    }

    #[test]
    fn self_crossing_path_fills_without_holes() {
        // A bow-tie path: nonzero winding fills both lobes.
        let s = Shape::Path {
            nodes: [[0.0, 0.0], [1.0, 1.0], [1.0, 0.0], [0.0, 1.0]].into_iter().map(Node::corner).collect(),
        };
        let a = alpha_after(&s, 40, 40);
        assert_eq!(a[20 * 40 + 5], 255);
        assert_eq!(a[20 * 40 + 34], 255);
    }

    #[test]
    fn spans_count_samples_exactly() {
        // A rectangle from x 1.25 to 3.75 covers pixel 1 by 3/4 and pixel 3 by 3/4.
        let poly = [[1.25, 0.0], [3.75, 0.0], [3.75, 1.0], [1.25, 1.0]];
        let c = coverage(&poly, 5, 1);
        assert_eq!(c, vec![0, 12, 16, 12, 0]);
    }

    #[test]
    fn big_circle_is_fast() {
        let t = std::time::Instant::now();
        let poly = outline(&Shape::Circle, 4096.0, 4096.0);
        let c = coverage(&poly, 4096, 4096);
        assert_eq!(c[2048 * 4096 + 2048], 16);
        let dt = t.elapsed();
        println!("4096^2 circle mask: {dt:?}");
        assert!(dt.as_secs_f64() < 5.0);
    }
}
