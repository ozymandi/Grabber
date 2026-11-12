//! Clipboard output (design D9). On Windows `arboard` writes both the registered `PNG`
//! format (keeps transparency in Photoshop, Figma, browsers) and `CF_DIBV5`, as real data
//! rather than delayed rendering, so the image stays on the clipboard after ripper exits.

use std::borrow::Cow;

/// Puts a straight-alpha RGBA8 image on the clipboard.
pub fn put_image(w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_image(arboard::ImageData { width: w as usize, height: h as usize, bytes: Cow::Borrowed(rgba) })
        .map_err(|e| e.to_string())
}
