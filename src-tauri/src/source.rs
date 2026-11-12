//! Source images: decoding files and clipboard images into straight-alpha RGBA8.

use chrono::NaiveDateTime;
use image::{ImageFormat, ImageReader};
use std::fmt;
use std::path::Path;

/// A decoded source image held for the whole session.
#[derive(Clone)]
pub struct Source {
    pub w: u32,
    pub h: u32,
    /// Straight-alpha RGBA8, row-major, `w * h * 4` bytes.
    pub rgba: Vec<u8>,
    /// File name stem used for output names (`<stem>_q<N>.png`).
    pub stem: String,
}

/// Why a source could not be loaded. `Display` names the file so the status row can show it.
#[derive(Debug)]
pub enum LoadError {
    Missing(String),
    Unreadable(String, String),
    Unsupported(String),
    Decode(String, String),
    NoClipboardImage,
    Clipboard(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Missing(n) => write!(f, "File not found: {n}"),
            LoadError::Unreadable(n, e) => write!(f, "Cannot read {n}: {e}"),
            LoadError::Unsupported(n) => {
                write!(f, "Unsupported image format: {n} (use PNG, JPEG, BMP or WebP)")
            }
            LoadError::Decode(n, e) => write!(f, "Cannot decode {n}: {e}"),
            LoadError::NoClipboardImage => write!(f, "No image in the clipboard to paste"),
            LoadError::Clipboard(e) => write!(f, "Cannot read the clipboard: {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

const SUPPORTED: [ImageFormat; 4] = [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Bmp, ImageFormat::WebP];

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Decodes a PNG, JPEG, BMP or WebP file. The format is detected from the content.
pub fn load_path(path: &Path) -> Result<Source, LoadError> {
    let name = display_name(path);
    if !path.is_file() {
        return Err(LoadError::Missing(name));
    }
    let reader = ImageReader::open(path)
        .map_err(|e| LoadError::Unreadable(name.clone(), e.to_string()))?
        .with_guessed_format()
        .map_err(|e| LoadError::Unreadable(name.clone(), e.to_string()))?;
    match reader.format() {
        Some(f) if SUPPORTED.contains(&f) => {}
        _ => return Err(LoadError::Unsupported(name)),
    }
    let img = reader.decode().map_err(|e| LoadError::Decode(name.clone(), e.to_string()))?;
    let rgba = img.into_rgba8();
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "image".to_string());
    Ok(Source { w: rgba.width(), h: rgba.height(), rgba: rgba.into_raw(), stem })
}

/// Stem for a pasted image: `clipboard_<yyyyMMdd-HHmmss>`.
pub fn clipboard_stem(t: NaiveDateTime) -> String {
    format!("clipboard_{}", t.format("%Y%m%d-%H%M%S"))
}

/// Wraps raw clipboard pixels (RGBA8) as a source.
pub fn from_clipboard_pixels(w: u32, h: u32, rgba: Vec<u8>, now: NaiveDateTime) -> Result<Source, LoadError> {
    if w == 0 || h == 0 || rgba.len() != (w as usize) * (h as usize) * 4 {
        return Err(LoadError::NoClipboardImage);
    }
    Ok(Source { w, h, rgba, stem: clipboard_stem(now) })
}

/// Reads the image currently on the clipboard.
pub fn read_clipboard() -> Result<Source, LoadError> {
    let mut cb = arboard::Clipboard::new().map_err(|e| LoadError::Clipboard(e.to_string()))?;
    match cb.get_image() {
        Ok(img) => from_clipboard_pixels(
            img.width as u32,
            img.height as u32,
            img.bytes.into_owned(),
            chrono::Local::now().naive_local(),
        ),
        Err(arboard::Error::ContentNotAvailable) => Err(LoadError::NoClipboardImage),
        Err(e) => Err(LoadError::Clipboard(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ripper-test-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sample() -> RgbaImage {
        RgbaImage::from_fn(13, 7, |x, y| Rgba([(x * 19) as u8, (y * 31) as u8, 200, 255]))
    }

    fn round_trip(fmt: ImageFormat, ext: &str, lossy: bool) {
        let dir = temp_dir(ext);
        let path = dir.join(format!("shot.{ext}"));
        let img = sample();
        match fmt {
            // JPEG and BMP have no alpha channel in these encoders; write RGB.
            ImageFormat::Jpeg | ImageFormat::Bmp => {
                image::DynamicImage::ImageRgba8(img.clone()).to_rgb8().save_with_format(&path, fmt).unwrap()
            }
            _ => img.save_with_format(&path, fmt).unwrap(),
        }
        let src = load_path(&path).unwrap();
        assert_eq!((src.w, src.h), (13, 7));
        assert_eq!(src.stem, "shot");
        assert_eq!(src.rgba.len(), 13 * 7 * 4);
        let tol = if lossy { 24 } else { 0 };
        for (a, b) in src.rgba.chunks(4).zip(img.pixels()) {
            for k in 0..4 {
                assert!((a[k] as i32 - b.0[k] as i32).abs() <= tol, "{ext}: {a:?} vs {:?}", b.0);
            }
        }
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn loads_png() {
        round_trip(ImageFormat::Png, "png", false);
    }

    #[test]
    fn loads_jpeg() {
        round_trip(ImageFormat::Jpeg, "jpg", true);
    }

    #[test]
    fn loads_bmp() {
        round_trip(ImageFormat::Bmp, "bmp", false);
    }

    #[test]
    fn loads_webp() {
        round_trip(ImageFormat::WebP, "webp", false);
    }

    #[test]
    fn detects_format_from_content() {
        let dir = temp_dir("misnamed");
        let path = dir.join("actually_png.jpg");
        sample().save_with_format(&path, ImageFormat::Png).unwrap();
        assert!(load_path(&path).is_ok());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn missing_file_names_it() {
        let err = load_path(Path::new(r"C:\definitely\not\here\a.png")).err().unwrap();
        assert!(matches!(err, LoadError::Missing(_)));
        assert!(err.to_string().contains("a.png"));
    }

    #[test]
    fn garbage_file_is_rejected_with_its_name() {
        let dir = temp_dir("garbage");
        let path = dir.join("broken.png");
        std::fs::write(&path, b"this is not an image").unwrap();
        let err = load_path(&path).err().unwrap();
        assert!(err.to_string().contains("broken.png"), "{err}");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn clipboard_stem_format() {
        let t = chrono::NaiveDate::from_ymd_opt(2026, 11, 13).unwrap().and_hms_opt(9, 5, 7).unwrap();
        assert_eq!(clipboard_stem(t), "clipboard_20261113-090507");
    }

    #[test]
    fn clipboard_pixels_validated() {
        let t = chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap().and_hms_opt(3, 4, 5).unwrap();
        assert!(from_clipboard_pixels(2, 2, vec![0; 16], t).is_ok());
        assert!(from_clipboard_pixels(2, 2, vec![0; 15], t).is_err());
        assert!(from_clipboard_pixels(0, 2, vec![], t).is_err());
    }
}
