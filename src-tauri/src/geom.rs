//! Quad geometry: the square-to-quad homography, quad validity, and output size rules.
//!
//! Coordinates are source-image pixels with y pointing down. A quad's corners are
//! stored top-left, top-right, bottom-right, bottom-left, i.e. clockwise on screen.
//! `ui/js/geom.js` is a line-for-line port of this file used only for the preview;
//! `tests/fixtures/quads.json` keeps the two in agreement.

use serde::{Deserialize, Serialize};

/// A 2D point in source-image pixels.
pub type Pt = [f64; 2];

/// A 3x3 matrix, row-major. Maps a column vector `[u, v, 1]` to homogeneous `[x, y, w]`.
pub type Mat3 = [[f64; 3]; 3];

/// Smallest accepted quad area, in square source pixels.
pub const MIN_AREA: f64 = 4.0;
/// Smallest accepted interior angle, in degrees. Angles above `180 - MIN_ANGLE_DEG`
/// count as three nearly collinear corners.
pub const MIN_ANGLE_DEG: f64 = 2.0;
/// Largest output side, in pixels. Guards against corners dragged far outside the image.
pub const MAX_SIDE: u32 = 16384;

/// Homography mapping the unit square onto the quad (Heckbert's closed form):
/// (0,0) -> c[0], (1,0) -> c[1], (1,1) -> c[2], (0,1) -> c[3].
///
/// Returns `None` when the quad is degenerate and no projective map exists.
pub fn square_to_quad(c: &[Pt; 4]) -> Option<Mat3> {
    let [[x0, y0], [x1, y1], [x2, y2], [x3, y3]] = *c;
    let sx = x0 - x1 + x2 - x3;
    let sy = y0 - y1 + y2 - y3;
    let dx1 = x1 - x2;
    let dx2 = x3 - x2;
    let dy1 = y1 - y2;
    let dy2 = y3 - y2;
    let den = dx1 * dy2 - dx2 * dy1;
    if den == 0.0 || !den.is_finite() {
        return None;
    }
    let g = (sx * dy2 - dx2 * sy) / den;
    let h = (dx1 * sy - sx * dy1) / den;
    let m = [
        [x1 - x0 + g * x1, x3 - x0 + h * x3, x0],
        [y1 - y0 + g * y1, y3 - y0 + h * y3, y0],
        [g, h, 1.0],
    ];
    if m.iter().flatten().all(|v| v.is_finite()) {
        Some(m)
    } else {
        None
    }
}

/// Homography mapping output pixel coordinates (`0..w`, `0..h`) to source pixels.
pub fn output_to_source(c: &[Pt; 4], w: u32, h: u32) -> Option<Mat3> {
    let m = square_to_quad(c)?;
    let sx = 1.0 / w.max(1) as f64;
    let sy = 1.0 / h.max(1) as f64;
    Some([
        [m[0][0] * sx, m[0][1] * sy, m[0][2]],
        [m[1][0] * sx, m[1][1] * sy, m[1][2]],
        [m[2][0] * sx, m[2][1] * sy, m[2][2]],
    ])
}

/// Applies a homography to a point. Returns `None` for points at infinity.
pub fn apply(m: &Mat3, p: Pt) -> Option<Pt> {
    let x = m[0][0] * p[0] + m[0][1] * p[1] + m[0][2];
    let y = m[1][0] * p[0] + m[1][1] * p[1] + m[1][2];
    let w = m[2][0] * p[0] + m[2][1] * p[1] + m[2][2];
    if w.abs() < 1e-12 {
        None
    } else {
        Some([x / w, y / w])
    }
}

/// Inverse via the adjugate. Returns `None` for a singular matrix.
pub fn invert(m: &Mat3) -> Option<Mat3> {
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    let co00 = e * i - f * h;
    let co01 = f * g - d * i;
    let co02 = d * h - e * g;
    let det = a * co00 + b * co01 + c * co02;
    if det.abs() < 1e-300 || !det.is_finite() {
        return None;
    }
    let inv = 1.0 / det;
    Some([
        [co00 * inv, (c * h - b * i) * inv, (b * f - c * e) * inv],
        [co01 * inv, (a * i - c * g) * inv, (c * d - a * f) * inv],
        [co02 * inv, (b * g - a * h) * inv, (a * e - b * d) * inv],
    ])
}

/// Signed area (shoelace). Positive when the corners run clockwise on screen (y down).
pub fn signed_area(c: &[Pt; 4]) -> f64 {
    let mut s = 0.0;
    for i in 0..4 {
        let p = c[i];
        let q = c[(i + 1) % 4];
        s += p[0] * q[1] - q[0] * p[1];
    }
    0.5 * s
}

/// Puts the corners in clockwise screen order while keeping the first corner first.
/// A quad clicked counter-clockwise would otherwise come out mirrored.
pub fn clockwise(c: &[Pt; 4]) -> [Pt; 4] {
    if signed_area(c) < 0.0 {
        [c[0], c[3], c[2], c[1]]
    } else {
        *c
    }
}

/// Why a quad cannot be exported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Invalid {
    /// Coordinates are not finite numbers.
    NotFinite,
    /// Edges cross or the shape is concave.
    NotConvex,
    /// Area below [`MIN_AREA`].
    TooSmall,
    /// An interior angle is below [`MIN_ANGLE_DEG`] or above `180 - MIN_ANGLE_DEG`.
    Collinear,
    /// The quad's shape covers no area (add-quad-shapes D3).
    EmptyShape,
}

/// Checks a quad given in clockwise order (see [`clockwise`]).
pub fn validate(c: &[Pt; 4]) -> Result<(), Invalid> {
    if c.iter().flatten().any(|v| !v.is_finite()) {
        return Err(Invalid::NotFinite);
    }
    // Every turn must go the same (clockwise) way; a bow-tie or concave quad fails.
    for i in 0..4 {
        let a = c[i];
        let b = c[(i + 1) % 4];
        let d = c[(i + 2) % 4];
        let cross = (b[0] - a[0]) * (d[1] - b[1]) - (b[1] - a[1]) * (d[0] - b[0]);
        if cross <= 0.0 {
            // A zero turn is three collinear corners; report it as such.
            return Err(if cross == 0.0 { Invalid::Collinear } else { Invalid::NotConvex });
        }
    }
    if signed_area(c) < MIN_AREA {
        return Err(Invalid::TooSmall);
    }
    let lo = MIN_ANGLE_DEG.to_radians();
    let hi = (180.0 - MIN_ANGLE_DEG).to_radians();
    for i in 0..4 {
        let p = c[(i + 3) % 4];
        let v = c[i];
        let n = c[(i + 1) % 4];
        let a = [p[0] - v[0], p[1] - v[1]];
        let b = [n[0] - v[0], n[1] - v[1]];
        let cross = a[0] * b[1] - a[1] * b[0];
        let dot = a[0] * b[0] + a[1] * b[1];
        let ang = cross.abs().atan2(dot);
        if !(lo..=hi).contains(&ang) {
            return Err(Invalid::Collinear);
        }
    }
    Ok(())
}

/// How a quad's output aspect is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Aspect {
    /// Width and height from the mean lengths of opposite edges.
    Edges,
    /// Width from the edges, height from the ratio `w : h`.
    Ratio { w: f64, h: f64 },
}

/// Mean lengths of the top/bottom and left/right edges, in source pixels.
pub fn edge_size(c: &[Pt; 4]) -> (f64, f64) {
    let len = |a: Pt, b: Pt| ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    let top = len(c[0], c[1]);
    let bottom = len(c[3], c[2]);
    let left = len(c[0], c[3]);
    let right = len(c[1], c[2]);
    ((top + bottom) * 0.5, (left + right) * 0.5)
}

/// Nearest power of two, ties rounding up, minimum 1.
pub fn nearest_pot(n: u32) -> u32 {
    if n <= 1 {
        return 1;
    }
    if n.is_power_of_two() {
        return n;
    }
    let hi = n.next_power_of_two();
    let lo = hi / 2;
    if n - lo < hi - n {
        lo
    } else {
        hi
    }
}

fn to_side(v: f64) -> u32 {
    if !v.is_finite() {
        return 1;
    }
    (v.round().max(1.0) as u64).min(MAX_SIDE as u64) as u32
}

/// Output size of a quad (clockwise corners): edge-based width, height from the aspect
/// rule, then optional power-of-two snapping. Each side is at least 1 and at most
/// [`MAX_SIDE`].
pub fn output_size(c: &[Pt; 4], aspect: Aspect, pot: bool) -> (u32, u32) {
    let (we, he) = edge_size(c);
    let w = to_side(we);
    let h = match aspect {
        Aspect::Edges => to_side(he),
        Aspect::Ratio { w: rw, h: rh } if rw > 0.0 && rh > 0.0 => to_side(w as f64 * rh / rw),
        Aspect::Ratio { .. } => to_side(he),
    };
    if pot {
        (nearest_pot(w).min(MAX_SIDE), nearest_pot(h).min(MAX_SIDE))
    } else {
        (w, h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Pt, b: Pt, eps: f64) -> bool {
        (a[0] - b[0]).abs() <= eps && (a[1] - b[1]).abs() <= eps
    }

    const SKEWED: [Pt; 4] = [[120.5, 80.25], [910.0, 140.0], [870.0, 610.0], [60.0, 700.0]];

    #[test]
    fn corners_map_exactly() {
        let m = square_to_quad(&SKEWED).unwrap();
        let unit = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        for (u, c) in unit.iter().zip(SKEWED.iter()) {
            assert!(close(apply(&m, *u).unwrap(), *c, 1e-9), "{u:?} -> {c:?}");
        }
    }

    #[test]
    fn output_corners_map_to_quad() {
        let m = output_to_source(&SKEWED, 512, 256).unwrap();
        let outs = [[0.0, 0.0], [512.0, 0.0], [512.0, 256.0], [0.0, 256.0]];
        for (o, c) in outs.iter().zip(SKEWED.iter()) {
            assert!(close(apply(&m, *o).unwrap(), *c, 1e-9));
        }
    }

    #[test]
    fn inverse_round_trip() {
        let m = square_to_quad(&SKEWED).unwrap();
        let inv = invert(&m).unwrap();
        for p in [[0.25, 0.75], [0.5, 0.5], [0.9, 0.1], [0.0, 1.0]] {
            let back = apply(&inv, apply(&m, p).unwrap()).unwrap();
            assert!(close(back, p, 1e-9), "{p:?} -> {back:?}");
        }
    }

    #[test]
    fn parallelogram_is_affine() {
        let c = [[10.0, 10.0], [110.0, 30.0], [130.0, 130.0], [30.0, 110.0]];
        let m = square_to_quad(&c).unwrap();
        assert!(m[2][0].abs() < 1e-12 && m[2][1].abs() < 1e-12);
    }

    #[test]
    fn straight_lines_stay_straight() {
        let m = square_to_quad(&SKEWED).unwrap();
        let a = apply(&m, [0.1, 0.3]).unwrap();
        let b = apply(&m, [0.5, 0.5]).unwrap();
        let c = apply(&m, [0.9, 0.7]).unwrap();
        let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        assert!(cross.abs() < 1e-6);
    }

    #[test]
    fn clockwise_keeps_first_and_fixes_ccw() {
        let ccw = [SKEWED[0], SKEWED[3], SKEWED[2], SKEWED[1]];
        assert!(signed_area(&ccw) < 0.0);
        assert_eq!(clockwise(&ccw), SKEWED);
        assert_eq!(clockwise(&SKEWED), SKEWED);
    }

    #[test]
    fn valid_quad_passes() {
        assert_eq!(validate(&SKEWED), Ok(()));
    }

    #[test]
    fn bow_tie_is_rejected() {
        let c = [[0.0, 0.0], [100.0, 0.0], [0.0, 100.0], [100.0, 100.0]];
        assert_eq!(validate(&clockwise(&c)), Err(Invalid::NotConvex));
    }

    #[test]
    fn concave_is_rejected() {
        let c = [[0.0, 0.0], [100.0, 0.0], [30.0, 30.0], [0.0, 100.0]];
        assert_eq!(validate(&clockwise(&c)), Err(Invalid::NotConvex));
    }

    #[test]
    fn collinear_is_rejected() {
        let exact = [[0.0, 0.0], [50.0, 0.0], [100.0, 0.0], [50.0, 80.0]];
        assert_eq!(validate(&clockwise(&exact)), Err(Invalid::Collinear));
        // Nearly collinear: the corner at (50, -0.5) bulges out by about 1.15 degrees.
        let near = [[0.0, 0.0], [50.0, -0.5], [100.0, 0.0], [50.0, 80.0]];
        assert_eq!(validate(&clockwise(&near)), Err(Invalid::Collinear));
    }

    #[test]
    fn tiny_is_rejected() {
        let c = [[0.0, 0.0], [1.5, 0.0], [1.5, 1.5], [0.0, 1.5]];
        assert_eq!(validate(&c), Err(Invalid::TooSmall));
    }

    #[test]
    fn duplicate_corner_is_rejected() {
        let c = [[0.0, 0.0], [0.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
        assert!(validate(&clockwise(&c)).is_err());
    }

    #[test]
    fn non_finite_is_rejected() {
        let c = [[f64::NAN, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
        assert_eq!(validate(&c), Err(Invalid::NotFinite));
    }

    /// Spec quad-unwarp: top 400, bottom 300, sides 200 and 200.
    fn spec_quad() -> [Pt; 4] {
        let ys = 200.0_f64;
        // Top edge 400 long, bottom edge 300 long, both sides exactly 200 long.
        let dx = 50.0; // (400 - 300) / 2
        let dy = (ys * ys - dx * dx).sqrt();
        [[0.0, 0.0], [400.0, 0.0], [400.0 - dx, dy], [dx, dy]]
    }

    #[test]
    fn spec_edge_size_350x200() {
        assert_eq!(output_size(&spec_quad(), Aspect::Edges, false), (350, 200));
    }

    #[test]
    fn spec_lock_1_1_350x350() {
        let a = Aspect::Ratio { w: 1.0, h: 1.0 };
        assert_eq!(output_size(&spec_quad(), a, false), (350, 350));
    }

    #[test]
    fn spec_custom_3_1_600x200() {
        let c = [[0.0, 0.0], [600.0, 0.0], [600.0, 90.0], [0.0, 90.0]];
        let a = Aspect::Ratio { w: 3.0, h: 1.0 };
        assert_eq!(output_size(&c, a, false), (600, 200));
    }

    #[test]
    fn spec_pot_256x256() {
        assert_eq!(output_size(&spec_quad(), Aspect::Edges, true), (256, 256));
    }

    #[test]
    fn pot_rounding() {
        assert_eq!(nearest_pot(0), 1);
        assert_eq!(nearest_pot(1), 1);
        assert_eq!(nearest_pot(3), 4); // tie between 2 and 4 rounds up
        assert_eq!(nearest_pot(5), 4);
        assert_eq!(nearest_pot(384), 512); // tie rounds up
        assert_eq!(nearest_pot(383), 256);
        assert_eq!(nearest_pot(1024), 1024);
    }

    #[test]
    fn sizes_are_clamped() {
        let c = [[0.0, 0.0], [1e9, 0.0], [1e9, 1e9], [0.0, 1e9]];
        assert_eq!(output_size(&c, Aspect::Edges, false), (MAX_SIDE, MAX_SIDE));
        let tiny = [[0.0, 0.0], [0.2, 0.0], [0.2, 0.2], [0.0, 0.2]];
        assert_eq!(output_size(&tiny, Aspect::Edges, false), (1, 1));
    }
}
