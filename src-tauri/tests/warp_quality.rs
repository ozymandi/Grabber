//! Export quality checks for the warp sampler (spec quad-unwarp; tasks 3.1-3.3).

use ripper_core::geom::{self, Pt};
use ripper_core::warp::{warp, Rgba};

/// Board: 8 x 8 squares of 64 px, white where (col + row) is even.
const SQ: f64 = 64.0;
const BOARD: u32 = 512;

fn board_value(x: f64, y: f64) -> f64 {
    let c = (x / SQ).floor() as i64 + (y / SQ).floor() as i64;
    if c.rem_euclid(2) == 0 {
        255.0
    } else {
        0.0
    }
}

/// Renders the board into a `w x h` grey source through the perspective of `quad`,
/// 4 x 4 supersampled. Outside the board the source is mid grey.
fn render_board(quad: &[Pt; 4], w: u32, h: u32) -> Vec<u8> {
    let to_src = geom::output_to_source(quad, BOARD, BOARD).unwrap();
    let to_board = geom::invert(&to_src).unwrap();
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0;
            for j in 0..4 {
                for i in 0..4 {
                    let p = [x as f64 + (i as f64 + 0.5) / 4.0, y as f64 + (j as f64 + 0.5) / 4.0];
                    let b = geom::apply(&to_board, p).unwrap();
                    let inside = (0.0..BOARD as f64).contains(&b[0]) && (0.0..BOARD as f64).contains(&b[1]);
                    acc += if inside { board_value(b[0], b[1]) } else { 128.0 };
                }
            }
            let v = (acc / 16.0).round() as u8;
            data.extend_from_slice(&[v, v, v, 255]);
        }
    }
    data
}

/// Positions (sub-pixel) where a scanline crosses mid grey, between `from` and `to`.
fn crossings(line: &[f64], from: usize, to: usize) -> Vec<f64> {
    let mut out = Vec::new();
    for x in from.max(1)..to.min(line.len()) {
        let (a, b) = (line[x - 1] - 127.5, line[x] - 127.5);
        if a * b < 0.0 {
            let t = a / (a - b);
            out.push(x as f64 - 0.5 + t);
        }
    }
    out
}

/// Spec quad-unwarp, scenario "Rectify a known rectangle": every square edge in the
/// output is axis-aligned within 1 px.
#[test]
fn perspective_checkerboard_edges_within_1px() {
    let (w, h) = (1200u32, 900u32);
    let quad: [Pt; 4] = [[300.0, 150.0], [900.0, 220.0], [1050.0, 800.0], [150.0, 700.0]];
    let src = render_board(&quad, w, h);
    let out = warp(&Rgba { w, h, data: &src }, &quad, BOARD, BOARD).unwrap();
    let px = |x: usize, y: usize| out[(y * BOARD as usize + x) * 4] as f64;
    let n = BOARD as usize;
    let mut worst: f64 = 0.0;
    let mut lines = 0;
    for k in 0..n {
        // Skip scanlines that run along an edge, where the crossing is undefined.
        let along = (k as f64 + 0.5) % SQ;
        if along < 4.0 || along > SQ - 4.0 {
            continue;
        }
        let row: Vec<f64> = (0..n).map(|x| px(x, k)).collect();
        let col: Vec<f64> = (0..n).map(|y| px(k, y)).collect();
        for line in [row, col] {
            // Interior edges only: the outer border meets the grey surround.
            let xs = crossings(&line, 8, n - 8);
            assert_eq!(xs.len(), 7, "scanline {k}: expected 7 interior edges, got {xs:?}");
            for x in xs {
                let err = (x - (x / SQ).round() * SQ).abs();
                worst = worst.max(err);
            }
            lines += 1;
        }
    }
    assert!(lines > 700, "too few scanlines checked: {lines}");
    assert!(worst <= 1.0, "worst edge offset {worst:.3} px");
    eprintln!("checkerboard: {lines} scanlines, worst edge offset {worst:.3} px");
}

fn stripes(w: u32, h: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..h {
        for x in 0..w {
            let v = if x % 2 == 0 { 255 } else { 0 };
            data.extend_from_slice(&[v, v, v, 255]);
        }
    }
    data
}

/// Max deviation of the output's grey from 127.5, the box-filtered reference of 1 px stripes.
fn max_dev(out: &[u8]) -> f64 {
    out.chunks(4).map(|p| (p[0] as f64 - 127.5).abs()).fold(0.0, f64::max)
}

/// Spec quad-unwarp, scenario "Strong minification": no moire against a box-filtered
/// reference. Axis-aligned 8x shrink lands exactly on texel centres: error <= 0.5.
#[test]
fn stripes_axis_aligned_minification() {
    let (w, h) = (2048u32, 256u32);
    let src = stripes(w, h);
    let quad: [Pt; 4] = [[0.0, 0.0], [w as f64, 0.0], [w as f64, h as f64], [0.0, h as f64]];
    let out = warp(&Rgba { w, h, data: &src }, &quad, 256, 32).unwrap();
    let dev = max_dev(&out);
    assert!(dev <= 0.5, "max deviation {dev}");
}

/// Box-filtered reference for output pixel `(x, y)`: the mean of 32 x 32 point samples
/// (nearest texel) spread over the pixel's exact footprint in the source.
fn box_reference(src: &[u8], w: u32, h: u32, m: &geom::Mat3, x: usize, y: usize) -> f64 {
    const N: usize = 32;
    let mut acc = 0.0;
    for j in 0..N {
        for i in 0..N {
            let p = [x as f64 + (i as f64 + 0.5) / N as f64, y as f64 + (j as f64 + 0.5) / N as f64];
            let s = geom::apply(m, p).unwrap();
            let tx = (s[0].floor() as i64).clamp(0, w as i64 - 1) as usize;
            let ty = (s[1].floor() as i64).clamp(0, h as i64 - 1) as usize;
            acc += src[(ty * w as usize + tx) * 4] as f64;
        }
    }
    acc / (N * N) as f64
}

/// Same, under perspective with 2-6x shrink, against a true box-filtered reference.
/// A single point sample per pixel would be off by up to ~127 here; the bound is 16
/// (about 6% of full range).
#[test]
fn stripes_perspective_minification() {
    let (w, h) = (2048u32, 2048u32);
    let src = stripes(w, h);
    let quad: [Pt; 4] = [[100.0, 100.0], [1900.0, 300.0], [1800.0, 1700.0], [300.0, 1900.0]];
    let (ow, oh) = (300u32, 300u32);
    let out = warp(&Rgba { w, h, data: &src }, &quad, ow, oh).unwrap();
    let m = geom::output_to_source(&quad, ow, oh).unwrap();
    let mut worst: f64 = 0.0;
    let mut sum = 0.0;
    let mut count = 0.0;
    for y in (0..oh as usize).step_by(3) {
        for x in (0..ow as usize).step_by(3) {
            let r = box_reference(&src, w, h, &m, x, y);
            let d = (out[(y * ow as usize + x) * 4] as f64 - r).abs();
            worst = worst.max(d);
            sum += d;
            count += 1.0;
        }
    }
    eprintln!("perspective stripes vs box reference: worst {worst:.2}, mean {:.2}", sum / count);
    assert!(worst <= 16.0, "worst deviation from box reference {worst:.2}");
}

/// Task 3.3: a 2048^2 output from a 4K source. Run in release:
///   cargo test --release --test warp_quality -- --ignored --nocapture timing
#[test]
#[ignore]
fn timing_2048_from_4k() {
    let (w, h) = (3840u32, 2160u32);
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let src: Vec<u8> = (0..w * h * 4)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 56) as u8
        })
        .collect();
    let quad: [Pt; 4] = [[400.0, 200.0], [3500.0, 350.0], [3300.0, 2000.0], [600.0, 1900.0]];
    let img = Rgba { w, h, data: &src };
    let _ = warp(&img, &quad, 256, 256); // warm the thread pool
    let mut best = f64::MAX;
    for _ in 0..5 {
        let t = std::time::Instant::now();
        let out = warp(&img, &quad, 2048, 2048).unwrap();
        best = best.min(t.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(out.len(), 2048 * 2048 * 4);
    }
    eprintln!("warp 2048x2048 from 3840x2160: best of 5 = {best:.1} ms");
    assert!(best < 150.0, "{best:.1} ms exceeds the 150 ms target");
}
