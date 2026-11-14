//! Quad shapes (design D1-D2 of add-quad-shapes): the mask a quad cuts out of its plane.
//!
//! A shape lives in the quad's unit square: (0, 0) is the first corner (the texture's
//! top-left), u runs right and v down. `outline` turns it into a closed polygon in output
//! pixels. `ui/js/shape.js` is a line-for-line port; `tests/fixtures/shapes.json` keeps the
//! two in agreement.

use crate::geom::Pt;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Largest distance between a flattened outline and the true curve, in output pixels.
pub const TOLERANCE: f64 = 0.05;
/// Circle handle length as a fraction of the radius for a 4-arc cubic approximation.
pub const KAPPA: f64 = 0.552_284_749_8;
/// Smallest and largest N of a regular polygon.
pub const NGON_MIN: u32 = 3;
pub const NGON_MAX: u32 = 12;
/// A shape whose outline covers less than this, in square output pixels, has no area.
pub const MIN_SHAPE_AREA: f64 = 1.0;

/// One anchor of a path, in unit-square coordinates. `hin` and `hout` are absolute handle
/// positions; `None` means no handle. A smooth anchor keeps both handles on one line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub p: Pt,
    #[serde(default)]
    pub hin: Option<Pt>,
    #[serde(default)]
    pub hout: Option<Pt>,
    #[serde(default)]
    pub smooth: bool,
}

impl Node {
    pub fn corner(p: Pt) -> Self {
        Node { p, hin: None, hout: None, smooth: false }
    }
}

/// The shape of a quad. Rectangle is the default, so requests without a shape keep working.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    #[default]
    Rect,
    Circle,
    Ngon {
        n: u32,
    },
    Path {
        nodes: Vec<Node>,
    },
}

/// Number of chords that keep a circle of radius `r` within [`TOLERANCE`], a multiple of 4
/// so the outline passes through the ellipse's four extremes.
fn circle_segments(r: f64) -> usize {
    if r <= TOLERANCE {
        return 8;
    }
    let n = (PI / (1.0 - TOLERANCE / r).acos()).ceil() as usize;
    (n.div_ceil(4) * 4).clamp(8, 4096)
}

/// Vertices of a regular N-gon scaled per axis so its bounding box is `[0, w] x [0, h]`:
/// N = 3 points up, even N has a flat top side.
fn ngon(n: u32, w: f64, h: f64) -> Vec<Pt> {
    let n = n.clamp(NGON_MIN, NGON_MAX);
    let off = if n % 2 == 0 { PI / n as f64 } else { 0.0 };
    let raw: Vec<Pt> = (0..n)
        .map(|k| {
            let a = -PI / 2.0 + 2.0 * PI * k as f64 / n as f64 + off;
            [a.cos(), a.sin()]
        })
        .collect();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in &raw {
        x0 = x0.min(p[0]);
        y0 = y0.min(p[1]);
        x1 = x1.max(p[0]);
        y1 = y1.max(p[1]);
    }
    raw.iter().map(|p| [(p[0] - x0) / (x1 - x0) * w, (p[1] - y0) / (y1 - y0) * h]).collect()
}

fn lerp(a: Pt, b: Pt, t: f64) -> Pt {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// Distance from `p` to the line through `a` and `b` (to `a` when they coincide).
fn line_dist(p: Pt, a: Pt, b: Pt) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-12 {
        return ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt();
    }
    ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs() / len
}

/// Appends the points of a cubic after its start, flattened by halving until flat.
fn flatten(p0: Pt, p1: Pt, p2: Pt, p3: Pt, tol: f64, depth: u32, out: &mut Vec<Pt>) {
    if depth >= 16 || (line_dist(p1, p0, p3) <= tol && line_dist(p2, p0, p3) <= tol) {
        out.push(p3);
        return;
    }
    let p01 = lerp(p0, p1, 0.5);
    let p12 = lerp(p1, p2, 0.5);
    let p23 = lerp(p2, p3, 0.5);
    let p012 = lerp(p01, p12, 0.5);
    let p123 = lerp(p12, p23, 0.5);
    let m = lerp(p012, p123, 0.5);
    flatten(p0, p01, p012, m, tol, depth + 1, out);
    flatten(m, p123, p23, p3, tol, depth + 1, out);
}

/// The four control points of path segment `i` (from anchor `i` to the next), in unit coords.
pub fn segment(nodes: &[Node], i: usize) -> [Pt; 4] {
    let a = &nodes[i];
    let b = &nodes[(i + 1) % nodes.len()];
    [a.p, a.hout.unwrap_or(a.p), b.hin.unwrap_or(b.p), b.p]
}

/// Closed outline of `shape` for a `w x h` output, in output pixels, flattened to `tol`.
pub fn outline_tol(shape: &Shape, w: f64, h: f64, tol: f64) -> Vec<Pt> {
    match shape {
        Shape::Rect => vec![[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]],
        Shape::Circle => {
            let (rx, ry) = (w / 2.0, h / 2.0);
            let n = circle_segments(rx.max(ry) * TOLERANCE / tol);
            (0..n)
                .map(|k| {
                    let a = -PI / 2.0 + 2.0 * PI * k as f64 / n as f64;
                    [rx + rx * a.cos(), ry + ry * a.sin()]
                })
                .collect()
        }
        Shape::Ngon { n } => ngon(*n, w, h),
        Shape::Path { nodes } => {
            if nodes.is_empty() {
                return Vec::new();
            }
            let sc = |p: Pt| [p[0] * w, p[1] * h];
            let mut out = vec![sc(nodes[0].p)];
            for i in 0..nodes.len() {
                let [p0, p1, p2, p3] = segment(nodes, i);
                flatten(sc(p0), sc(p1), sc(p2), sc(p3), tol, 0, &mut out);
            }
            out.pop(); // the last point repeats the first
            out
        }
    }
}

/// Closed outline of `shape` for a `w x h` output at [`TOLERANCE`].
pub fn outline(shape: &Shape, w: f64, h: f64) -> Vec<Pt> {
    outline_tol(shape, w, h, TOLERANCE)
}

/// Absolute polygon area (shoelace).
pub fn area(poly: &[Pt]) -> f64 {
    let n = poly.len();
    let mut s = 0.0;
    for i in 0..n {
        let p = poly[i];
        let q = poly[(i + 1) % n];
        s += p[0] * q[1] - q[0] * p[1];
    }
    (0.5 * s).abs()
}

/// True when the shape cuts out nothing in a `w x h` output.
pub fn is_empty(shape: &Shape, w: u32, h: u32) -> bool {
    match shape {
        Shape::Rect => false,
        Shape::Path { nodes } if nodes.len() < 2 => true,
        _ => area(&outline(shape, w as f64, h as f64)) < MIN_SHAPE_AREA,
    }
}

/// The shape as path anchors (design D2): Rect gives 4 corners, Circle 4 smooth anchors
/// with handles at [`KAPPA`], N-gon its N vertices; a Path is returned as it is.
pub fn to_path(shape: &Shape) -> Vec<Node> {
    match shape {
        Shape::Rect => [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]].into_iter().map(Node::corner).collect(),
        Shape::Circle => {
            let k = KAPPA * 0.5;
            let s = |p: Pt, hin: Pt, hout: Pt| Node { p, hin: Some(hin), hout: Some(hout), smooth: true };
            vec![
                s([0.5, 0.0], [0.5 - k, 0.0], [0.5 + k, 0.0]),
                s([1.0, 0.5], [1.0, 0.5 - k], [1.0, 0.5 + k]),
                s([0.5, 1.0], [0.5 + k, 1.0], [0.5 - k, 1.0]),
                s([0.0, 0.5], [0.0, 0.5 + k], [0.0, 0.5 - k]),
            ]
        }
        Shape::Ngon { n } => ngon(*n, 1.0, 1.0).into_iter().map(Node::corner).collect(),
        Shape::Path { nodes } => nodes.clone(),
    }
}

/// Inserts an anchor on segment `i` at parameter `t` without changing the curve
/// (de Casteljau). The new anchor is smooth unless the segment is straight.
pub fn split(nodes: &[Node], i: usize, t: f64) -> Vec<Node> {
    let n = nodes.len();
    let j = (i + 1) % n;
    let [p0, p1, p2, p3] = segment(nodes, i);
    let mut out = nodes.to_vec();
    let straight = nodes[i].hout.is_none() && nodes[j].hin.is_none();
    let node = if straight {
        Node::corner(lerp(p0, p3, t))
    } else {
        let p01 = lerp(p0, p1, t);
        let p12 = lerp(p1, p2, t);
        let p23 = lerp(p2, p3, t);
        let p012 = lerp(p01, p12, t);
        let p123 = lerp(p12, p23, t);
        if out[i].hout.is_some() {
            out[i].hout = Some(p01);
        }
        if out[j].hin.is_some() {
            out[j].hin = Some(p23);
        }
        Node { p: lerp(p012, p123, t), hin: Some(p012), hout: Some(p123), smooth: true }
    };
    out.insert(i + 1, node);
    out
}

/// Point on a cubic at `t`.
pub fn cubic_at(c: &[Pt; 4], t: f64) -> Pt {
    let u = 1.0 - t;
    let a = u * u * u;
    let b = 3.0 * u * u * t;
    let d = 3.0 * u * t * t;
    let e = t * t * t;
    [
        a * c[0][0] + b * c[1][0] + d * c[2][0] + e * c[3][0],
        a * c[0][1] + b * c[1][1] + d * c[2][1] + e * c[3][1],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Pt, b: Pt, eps: f64) -> bool {
        (a[0] - b[0]).abs() <= eps && (a[1] - b[1]).abs() <= eps
    }

    fn bbox(poly: &[Pt]) -> [f64; 4] {
        let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for p in poly {
            b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])];
        }
        b
    }

    #[test]
    fn octagon_touches_all_sides_with_flat_top() {
        let o = outline(&Shape::Ngon { n: 8 }, 512.0, 512.0);
        assert_eq!(o.len(), 8);
        let b = bbox(&o);
        assert!(close([b[0], b[1]], [0.0, 0.0], 1e-9) && close([b[2], b[3]], [512.0, 512.0], 1e-9));
        // Flat top and bottom: two vertices each on y = 0 and y = 512.
        assert_eq!(o.iter().filter(|p| p[1].abs() < 1e-9).count(), 2);
        assert_eq!(o.iter().filter(|p| (p[1] - 512.0).abs() < 1e-9).count(), 2);
        assert_eq!(o.iter().filter(|p| p[0].abs() < 1e-9).count(), 2);
    }

    #[test]
    fn triangle_points_up() {
        let t = outline(&Shape::Ngon { n: 3 }, 300.0, 200.0);
        assert!(close(t[0], [150.0, 0.0], 1e-9), "{:?}", t[0]);
        assert!((t[1][1] - 200.0).abs() < 1e-9 && (t[2][1] - 200.0).abs() < 1e-9);
        let b = bbox(&t);
        assert!(close([b[0], b[2]], [0.0, 300.0], 1e-9));
    }

    #[test]
    fn ellipse_extremes_on_the_edges() {
        let e = outline(&Shape::Circle, 400.0, 200.0);
        let b = bbox(&e);
        assert!(close([b[0], b[1]], [0.0, 0.0], 1e-9) && close([b[2], b[3]], [400.0, 200.0], 1e-9));
        // Chord error within the tolerance: every chord midpoint lies near the ellipse.
        for i in 0..e.len() {
            let m = lerp(e[i], e[(i + 1) % e.len()], 0.5);
            let r = ((m[0] - 200.0) / 200.0).powi(2) + ((m[1] - 100.0) / 100.0).powi(2);
            assert!(r <= 1.0 && r > 1.0 - 2.0 * TOLERANCE / 100.0, "{r}");
        }
    }

    #[test]
    fn circle_to_path_stays_on_the_circle() {
        let nodes = to_path(&Shape::Circle);
        let r = 4096.0;
        let w = 2.0 * r;
        for i in 0..4 {
            let c = segment(&nodes, i).map(|p| [p[0] * w, p[1] * w]);
            for k in 0..=64 {
                let p = cubic_at(&c, k as f64 / 64.0);
                let d = ((p[0] - r).powi(2) + (p[1] - r).powi(2)).sqrt();
                assert!((d - r).abs() <= 0.0003 * r, "{} at {k}", d - r);
            }
        }
    }

    #[test]
    fn split_keeps_the_curve() {
        let nodes = to_path(&Shape::Circle);
        let before = segment(&nodes, 1);
        let after = split(&nodes, 1, 0.3);
        assert_eq!(after.len(), 5);
        let a = segment(&after, 1);
        let b = segment(&after, 2);
        for k in 0..=20 {
            let t = k as f64 / 20.0;
            let want = cubic_at(&before, t);
            let got = if t <= 0.3 { cubic_at(&a, t / 0.3) } else { cubic_at(&b, (t - 0.3) / 0.7) };
            assert!(close(got, want, 1e-9), "{t}: {got:?} vs {want:?}");
        }
    }

    #[test]
    fn split_straight_segment_adds_a_corner() {
        let nodes = to_path(&Shape::Rect);
        let after = split(&nodes, 0, 0.25);
        assert_eq!(after[1], Node::corner([0.25, 0.0]));
        assert_eq!(after[0], nodes[0]);
    }

    #[test]
    fn rect_and_ngon_paths() {
        assert_eq!(to_path(&Shape::Rect).len(), 4);
        let o = to_path(&Shape::Ngon { n: 6 });
        assert_eq!(o.len(), 6);
        assert!(o.iter().all(|n| n.hin.is_none() && !n.smooth));
    }

    #[test]
    fn empty_shapes() {
        let two_corners = Shape::Path { nodes: vec![Node::corner([0.1, 0.1]), Node::corner([0.9, 0.9])] };
        assert!(is_empty(&two_corners, 256, 256));
        assert!(!is_empty(&Shape::Circle, 256, 256));
        assert!(is_empty(&Shape::Path { nodes: vec![Node::corner([0.5, 0.5])] }, 256, 256));
    }

    #[test]
    fn json_shapes() {
        let s: Shape = serde_json::from_str(r#"{"kind":"ngon","n":8}"#).unwrap();
        assert_eq!(s, Shape::Ngon { n: 8 });
        let p: Shape = serde_json::from_str(r#"{"kind":"path","nodes":[{"p":[0,0]},{"p":[1,0],"hin":[0.5,0.5],"smooth":true}]}"#).unwrap();
        assert!(matches!(p, Shape::Path { ref nodes } if nodes.len() == 2 && nodes[1].smooth));
        assert_eq!(serde_json::from_str::<Shape>(r#"{"kind":"rect"}"#).unwrap(), Shape::Rect);
    }
}
