//! Export sampler (design D6): perspective unwarp with Jacobian-adaptive supersampling.
//!
//! For every output pixel the local footprint of the projective map is estimated from its
//! Jacobian. The pixel is covered with an `nu x nv` grid of sub-samples (each at most
//! [`MAX_SUB`]), each mapped exactly through the homography and read with bilinear
//! filtering. Sums are done in premultiplied alpha so transparent texels do not bleed
//! colour. Anything that maps outside the source rectangle is transparent.

use crate::geom::{self, Mat3, Pt};
use rayon::prelude::*;

/// Maximum sub-samples per axis and output pixel.
pub const MAX_SUB: u32 = 8;

/// Straight-alpha RGBA8 image borrowed as a byte slice, row-major, `w * h * 4` bytes.
#[derive(Clone, Copy)]
pub struct Rgba<'a> {
    pub w: u32,
    pub h: u32,
    pub data: &'a [u8],
}

impl Rgba<'_> {
    /// Premultiplied texel with coordinates clamped to the image.
    #[inline]
    fn texel(&self, x: i64, y: i64) -> [f32; 4] {
        let x = x.clamp(0, self.w as i64 - 1) as usize;
        let y = y.clamp(0, self.h as i64 - 1) as usize;
        let i = (y * self.w as usize + x) * 4;
        let p = &self.data[i..i + 4];
        let a = p[3] as f32 * (1.0 / 255.0);
        [p[0] as f32 * a, p[1] as f32 * a, p[2] as f32 * a, p[3] as f32]
    }

    /// Bilinear sample at a source position in pixel units (texel centres at +0.5).
    /// Positions outside the image rectangle are fully transparent; positions inside it
    /// clamp to the edge texels, so a quad on the image border does not fade.
    #[inline]
    fn sample(&self, sx: f64, sy: f64) -> [f32; 4] {
        if !(sx >= 0.0 && sy >= 0.0 && sx <= self.w as f64 && sy <= self.h as f64) {
            return [0.0; 4];
        }
        let fx = sx - 0.5;
        let fy = sy - 0.5;
        let x0 = fx.floor();
        let y0 = fy.floor();
        let tx = (fx - x0) as f32;
        let ty = (fy - y0) as f32;
        let (x0, y0) = (x0 as i64, y0 as i64);
        let a = self.texel(x0, y0);
        let b = self.texel(x0 + 1, y0);
        let c = self.texel(x0, y0 + 1);
        let d = self.texel(x0 + 1, y0 + 1);
        let mut out = [0.0f32; 4];
        for k in 0..4 {
            let top = a[k] + (b[k] - a[k]) * tx;
            let bot = c[k] + (d[k] - c[k]) * tx;
            out[k] = top + (bot - top) * ty;
        }
        out
    }
}

/// Sub-sample counts along the output x and y axes at output point `(u, v)`.
#[inline]
fn footprint(m: &Mat3, u: f64, v: f64) -> (u32, u32) {
    let x = m[0][0] * u + m[0][1] * v + m[0][2];
    let y = m[1][0] * u + m[1][1] * v + m[1][2];
    let w = m[2][0] * u + m[2][1] * v + m[2][2];
    if w.abs() < 1e-12 {
        return (MAX_SUB, MAX_SUB);
    }
    let (px, py) = (x / w, y / w);
    // Partial derivatives of the projected point with respect to u and v.
    let dxu = (m[0][0] - px * m[2][0]) / w;
    let dyu = (m[1][0] - py * m[2][0]) / w;
    let dxv = (m[0][1] - px * m[2][1]) / w;
    let dyv = (m[1][1] - py * m[2][1]) / w;
    let lu = (dxu * dxu + dyu * dyu).sqrt();
    let lv = (dxv * dxv + dyv * dyv).sqrt();
    let n = |l: f64| -> u32 {
        if l.is_finite() {
            (l.ceil() as u32).clamp(1, MAX_SUB)
        } else {
            MAX_SUB
        }
    };
    (n(lu), n(lv))
}

/// Unwarps a quad (clockwise corners, see [`geom::clockwise`]) into a `out_w x out_h`
/// straight-alpha RGBA8 image. Returns `None` if the quad has no projective map.
pub fn warp(src: &Rgba, corners: &[Pt; 4], out_w: u32, out_h: u32) -> Option<Vec<u8>> {
    let m = geom::output_to_source(corners, out_w, out_h)?;
    let (ow, oh) = (out_w as usize, out_h as usize);
    let mut out = vec![0u8; ow * oh * 4];
    out.par_chunks_mut(ow * 4).enumerate().for_each(|(y, row)| {
        for x in 0..ow {
            let (nu, nv) = footprint(&m, x as f64 + 0.5, y as f64 + 0.5);
            let mut acc = [0.0f32; 4];
            for j in 0..nv {
                let v = y as f64 + (j as f64 + 0.5) / nv as f64;
                for i in 0..nu {
                    let u = x as f64 + (i as f64 + 0.5) / nu as f64;
                    let s = match geom::apply(&m, [u, v]) {
                        Some(p) => src.sample(p[0], p[1]),
                        None => [0.0; 4],
                    };
                    for k in 0..4 {
                        acc[k] += s[k];
                    }
                }
            }
            let inv_n = 1.0 / (nu * nv) as f32;
            let a = acc[3] * inv_n;
            let px = &mut row[x * 4..x * 4 + 4];
            if a > 0.0 {
                // acc[0..3] are colour * alpha/255 sums; divide by the alpha sum to unpremultiply.
                let un = 255.0 / acc[3];
                px[0] = (acc[0] * un).round().clamp(0.0, 255.0) as u8;
                px[1] = (acc[1] * un).round().clamp(0.0, 255.0) as u8;
                px[2] = (acc[2] * un).round().clamp(0.0, 255.0) as u8;
                px[3] = a.round().clamp(0.0, 255.0) as u8;
            }
        }
    });
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
        px.iter().copied().cycle().take((w * h * 4) as usize).collect()
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> [Pt; 4] {
        [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
    }

    #[test]
    fn identity_copy_is_exact() {
        let (w, h) = (7u32, 5u32);
        // Varied colours, every texel opaque so the straight-alpha round trip is exact.
        let data: Vec<u8> = (0..w * h)
            .flat_map(|i| [(i * 37 % 256) as u8, (i * 91 % 256) as u8, (i * 13 % 256) as u8, 255])
            .collect();
        let src = Rgba { w, h, data: &data };
        let out = warp(&src, &rect(0.0, 0.0, w as f64, h as f64), w, h).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn uniform_alpha_is_preserved() {
        let data = solid(64, 64, [255, 0, 0, 128]);
        let src = Rgba { w: 64, h: 64, data: &data };
        let quad = [[3.0, 5.0], [60.0, 2.0], [55.0, 61.0], [8.0, 50.0]];
        let out = warp(&src, &quad, 40, 30).unwrap();
        for p in out.chunks(4) {
            assert_eq!(p, [255, 0, 0, 128]);
        }
    }

    #[test]
    fn transparent_texels_do_not_bleed() {
        // Alternate opaque red and fully transparent green columns, then shrink 8x.
        let (w, h) = (256u32, 32u32);
        let mut data = Vec::with_capacity((w * h * 4) as usize);
        for _y in 0..h {
            for x in 0..w {
                data.extend_from_slice(if x % 2 == 0 { &[255, 0, 0, 255] } else { &[0, 255, 0, 0] });
            }
        }
        let src = Rgba { w, h, data: &data };
        let out = warp(&src, &rect(0.0, 0.0, w as f64, h as f64), 32, 4).unwrap();
        for p in out.chunks(4) {
            assert_eq!(p[1], 0, "green bled into {p:?}");
            assert_eq!(p[0], 255);
            assert!((p[3] as i32 - 128).abs() <= 1, "{p:?}");
        }
    }

    #[test]
    fn outside_source_is_transparent() {
        let data = solid(100, 100, [255, 255, 255, 255]);
        let src = Rgba { w: 100, h: 100, data: &data };
        // Left half of the quad lies outside the image.
        let out = warp(&src, &rect(-100.0, 0.0, 100.0, 100.0), 200, 100).unwrap();
        for y in 0..100usize {
            for x in 0..200usize {
                let a = out[(y * 200 + x) * 4 + 3];
                if x < 98 {
                    assert_eq!(a, 0, "({x},{y}) should be outside");
                } else if x >= 102 {
                    assert_eq!(a, 255, "({x},{y}) should be inside");
                }
            }
        }
    }

    #[test]
    fn border_quad_does_not_fade() {
        let data = solid(50, 40, [10, 20, 30, 255]);
        let src = Rgba { w: 50, h: 40, data: &data };
        let out = warp(&src, &rect(0.0, 0.0, 50.0, 40.0), 50, 40).unwrap();
        assert!(out.chunks(4).all(|p| p == [10, 20, 30, 255]));
    }

    #[test]
    fn footprint_counts_follow_scale() {
        // Output 4x smaller than the quad along x, same size along y.
        let m = geom::output_to_source(&rect(0.0, 0.0, 400.0, 100.0), 100, 100).unwrap();
        assert_eq!(footprint(&m, 50.0, 50.0), (4, 1));
        // Enlargement never asks for more than one sample.
        let m = geom::output_to_source(&rect(0.0, 0.0, 10.0, 10.0), 100, 100).unwrap();
        assert_eq!(footprint(&m, 50.0, 50.0), (1, 1));
        // Extreme shrink is capped.
        let m = geom::output_to_source(&rect(0.0, 0.0, 10_000.0, 10.0), 10, 10).unwrap();
        assert_eq!(footprint(&m, 5.0, 5.0), (MAX_SUB, 1));
    }
}
