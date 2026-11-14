//! Export (spec texture-export): render quads, encode PNG, write to folders without
//! overwriting, and report every failure without blocking the other outputs.

use crate::geom::{self, Aspect, Invalid, Pt};
use crate::mask;
use crate::shape::{self, Shape};
use crate::settings::Folder;
use crate::source::Source;
use crate::warp::{self, Rgba};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// One quad as sent by the UI. Rust recomputes order, validity and size itself (design D3).
#[derive(Debug, Clone, Deserialize)]
pub struct QuadReq {
    pub corners: [Pt; 4],
    pub aspect: Aspect,
    pub pot: bool,
    /// The mask on the quad's plane; Rectangle when the UI sends none (add-quad-shapes D1).
    #[serde(default)]
    pub shape: Shape,
    /// 1-based number shown in the Quads panel, used in the file name.
    pub number: u32,
}

/// A rendered quad: straight-alpha RGBA8.
pub struct Rendered {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// Renders a quad from the source, or says why it cannot be exported.
pub fn render(src: &Source, q: &QuadReq) -> Result<Rendered, Invalid> {
    let cw = geom::clockwise(&q.corners);
    geom::validate(&cw)?;
    let (w, h) = geom::output_size(&cw, q.aspect, q.pot);
    let img = Rgba { w: src.w, h: src.h, data: &src.rgba };
    if shape::is_empty(&q.shape, w, h) {
        return Err(Invalid::EmptyShape);
    }
    let mut rgba = warp::warp(&img, &cw, w, h).ok_or(Invalid::NotConvex)?;
    // A rectangle keeps the warp untouched, byte for byte (design D3).
    if q.shape != Shape::Rect {
        mask::apply(&mut rgba, w, h, &shape::outline(&q.shape, w as f64, h as f64));
    }
    Ok(Rendered { w, h, rgba })
}

pub fn encode_png(r: &Rendered) -> Vec<u8> {
    let mut out = Vec::new();
    let enc = image::codecs::png::PngEncoder::new(&mut out);
    image::ImageEncoder::write_image(enc, &r.rgba, r.w, r.h, image::ExtendedColorType::Rgba8)
        .expect("PNG encoding to memory cannot fail for valid RGBA8");
    out
}

/// File name for quad `number` of `stem`, with `_2`, `_3`... appended for `attempt` > 1.
pub fn file_name(stem: &str, number: u32, attempt: u32) -> String {
    if attempt <= 1 {
        format!("{stem}_q{number}.png")
    } else {
        format!("{stem}_q{number}_{attempt}.png")
    }
}

/// Writes `bytes` into `dir` under the first free name; never overwrites. Returns the path.
pub fn write_unique(dir: &Path, stem: &str, number: u32, bytes: &[u8]) -> io::Result<PathBuf> {
    if !dir.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "folder not found"));
    }
    for attempt in 1..=10_000 {
        let path = dir.join(file_name(stem, number, attempt));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                if let Err(e) = f.write_all(bytes).and_then(|_| f.sync_all()) {
                    drop(f);
                    let _ = std::fs::remove_file(&path);
                    return Err(e);
                }
                return Ok(path);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, "too many files with this name"))
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Failure {
    /// "Local", "Google Drive", "Clipboard" or "ShareX output".
    pub target: String,
    pub path: Option<String>,
    pub error: String,
}

#[derive(Debug, Default, Serialize)]
pub struct Outcome {
    pub written: Vec<String>,
    pub failures: Vec<Failure>,
}

/// Writes one PNG to every folder; a failing folder is recorded and the others still run.
pub fn write_to_folders(folders: &[Folder], stem: &str, number: u32, png: &[u8], out: &mut Outcome) {
    for f in folders {
        match write_unique(&f.dir, stem, number, png) {
            Ok(p) => out.written.push(p.display().to_string()),
            Err(e) => out.failures.push(Failure {
                target: f.label.to_string(),
                path: Some(f.dir.display().to_string()),
                error: e.to_string(),
            }),
        }
    }
}

/// Whether a Send in ShareX mode may close the app: only when the ShareX output was
/// written and nothing failed, including the atlas when its output is on. Any failure keeps
/// the window open so the warning is seen (spec texture-export).
pub fn exit_after_send(send: bool, sharex_output_written: bool, failures: usize, atlas_failed: bool) -> bool {
    send && sharex_output_written && failures == 0 && !atlas_failed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharex_send_stays_open_on_any_failure() {
        assert!(exit_after_send(true, true, 0, false));
        assert!(!exit_after_send(true, true, 0, true), "atlas failure keeps the app open");
        assert!(!exit_after_send(true, true, 1, false), "folder failure keeps the app open");
        assert!(!exit_after_send(true, false, 0, false), "standalone or unwritten output");
        assert!(!exit_after_send(false, true, 0, false), "Save All never exits");
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ripper-export-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn solid_source(w: u32, h: u32) -> Source {
        Source { w, h, rgba: [200u8, 100, 50, 255].repeat((w * h) as usize), stem: "shot".into() }
    }

    fn quad(corners: [Pt; 4], number: u32) -> QuadReq {
        QuadReq { corners, aspect: Aspect::Edges, pot: false, shape: Shape::Rect, number }
    }

    /// A gradient source, so a changed sampler or mask would show in the bytes.
    fn gradient_source(w: u32, h: u32) -> Source {
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&[(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255]);
            }
        }
        Source { w, h, rgba, stem: "grad".into() }
    }

    const SKEW: [Pt; 4] = [[40.5, 30.25], [610.0, 70.0], [580.0, 450.0], [20.0, 500.0]];

    #[test]
    fn rect_is_byte_identical_to_the_plain_warp() {
        // Spec "Rectangle is unchanged": before add-quad-shapes, render was exactly this warp.
        let src = gradient_source(640, 520);
        let r = render(&src, &quad(SKEW, 1)).unwrap();
        let cw = geom::clockwise(&SKEW);
        let (w, h) = geom::output_size(&cw, Aspect::Edges, false);
        let img = Rgba { w: src.w, h: src.h, data: &src.rgba };
        assert_eq!((r.w, r.h), (w, h));
        assert!(r.rgba == warp::warp(&img, &cw, w, h).unwrap());
    }

    #[test]
    fn circle_on_a_square_output() {
        // Spec "Circle on a square output": 512 x 512, opaque inside r 256, clear corners.
        let src = solid_source(600, 600);
        let mut q = quad([[0.0, 0.0], [512.0, 0.0], [512.0, 512.0], [0.0, 512.0]], 1);
        q.shape = Shape::Circle;
        let r = render(&src, &q).unwrap();
        assert_eq!((r.w, r.h), (512, 512));
        let a = |x: u32, y: u32| r.rgba[((y * 512 + x) * 4 + 3) as usize];
        assert_eq!(a(256, 256), 255);
        assert_eq!(a(256, 2), 255, "just inside the top");
        assert_eq!(a(0, 0), 0);
        assert_eq!(a(511, 511), 0);
        assert_eq!(a(60, 60), 0, "outside the circle near a corner (278 px from the centre)");
        assert!((0..512).any(|x| a(x, 0) > 0 && a(x, 0) < 255), "soft edge along the top row");
    }

    #[test]
    fn empty_shape_is_rejected() {
        let src = solid_source(100, 100);
        let mut q = quad([[0.0, 0.0], [90.0, 0.0], [90.0, 90.0], [0.0, 90.0]], 1);
        q.shape = Shape::Path { nodes: vec![shape::Node::corner([0.2, 0.2]), shape::Node::corner([0.8, 0.8])] };
        assert_eq!(render(&src, &q).err(), Some(Invalid::EmptyShape));
    }

    #[test]
    fn old_request_without_shape_is_a_rectangle() {
        let q: QuadReq = serde_json::from_str(r#"{"corners":[[0,0],[10,0],[10,10],[0,10]],"aspect":{"mode":"edges"},"pot":false,"number":1}"#).unwrap();
        assert_eq!(q.shape, Shape::Rect);
        let q: QuadReq = serde_json::from_str(r#"{"corners":[[0,0],[10,0],[10,10],[0,10]],"aspect":{"mode":"edges"},"pot":false,"shape":{"kind":"ngon","n":8},"number":1}"#).unwrap();
        assert_eq!(q.shape, Shape::Ngon { n: 8 });
    }

    #[test]
    fn names() {
        assert_eq!(file_name("shot", 1, 1), "shot_q1.png");
        assert_eq!(file_name("shot", 1, 2), "shot_q1_2.png");
        assert_eq!(file_name("clipboard_20261113-090507", 12, 3), "clipboard_20261113-090507_q12_3.png");
    }

    /// Spec: "Name collision" - existing shot_q1.png is kept, new file is shot_q1_2.png.
    #[test]
    fn collision_appends_suffix_and_keeps_existing() {
        let d = temp_dir("collision");
        std::fs::write(d.join("shot_q1.png"), b"old").unwrap();
        let p = write_unique(&d, "shot", 1, b"new").unwrap();
        assert_eq!(p.file_name().unwrap(), "shot_q1_2.png");
        assert_eq!(std::fs::read(d.join("shot_q1.png")).unwrap(), b"old");
        let p3 = write_unique(&d, "shot", 1, b"newer").unwrap();
        assert_eq!(p3.file_name().unwrap(), "shot_q1_3.png");
        std::fs::remove_dir_all(d).ok();
    }

    /// Spec: an unavailable folder does not block the other folder.
    #[test]
    fn failing_folder_does_not_block_others() {
        let good = temp_dir("good");
        let folders = [
            Folder { label: "Local", dir: good.clone() },
            Folder { label: "Google Drive", dir: PathBuf::from(r"Q:\definitely\not\mounted") },
        ];
        let mut out = Outcome::default();
        write_to_folders(&folders, "shot", 2, b"png", &mut out);
        assert_eq!(out.written.len(), 1);
        assert!(good.join("shot_q2.png").exists());
        assert_eq!(out.failures.len(), 1);
        assert_eq!(out.failures[0].target, "Google Drive");
        std::fs::remove_dir_all(good).ok();
    }

    #[test]
    fn render_valid_quad_and_reject_bow_tie() {
        let src = solid_source(64, 48);
        let r = render(&src, &quad([[4.0, 4.0], [60.0, 6.0], [58.0, 44.0], [6.0, 40.0]], 1)).unwrap();
        assert_eq!(r.rgba.len(), (r.w * r.h * 4) as usize);
        assert!(r.rgba.chunks(4).all(|p| p == [200, 100, 50, 255]));
        let bow = quad([[0.0, 0.0], [40.0, 0.0], [0.0, 40.0], [40.0, 40.0]], 2);
        assert_eq!(render(&src, &bow).err(), Some(Invalid::NotConvex));
    }

    /// The export size is Rust's own, whatever the UI believes (design D3).
    #[test]
    fn render_uses_rust_size_rules() {
        let src = solid_source(800, 600);
        let mut q = quad([[0.0, 0.0], [600.0, 0.0], [600.0, 90.0], [0.0, 90.0]], 1);
        q.aspect = Aspect::Ratio { w: 3.0, h: 1.0 };
        let r = render(&src, &q).unwrap();
        assert_eq!((r.w, r.h), (600, 200));
    }

    #[test]
    fn png_round_trip() {
        let r = Rendered { w: 3, h: 2, rgba: (0..24).map(|i| (i * 10) as u8).collect() };
        let png = encode_png(&r);
        let back = image::load_from_memory_with_format(&png, image::ImageFormat::Png).unwrap().into_rgba8();
        assert_eq!((back.width(), back.height()), (3, 2));
        assert_eq!(back.into_raw(), r.rgba);
    }
}
