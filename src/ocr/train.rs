//! Train glyph prototypes from a TrueType or OpenType font.
//!
//! Gated by the `ocr-train` Cargo feature. Pulls in `ab_glyph` for font
//! rasterization. Typical workflow:
//!
//! 1. Pick a TTF/OTF matching the typography you need to recognize.
//! 2. Call [`train_from_ttf_bytes`] with a pixel size close to the rendered
//!    glyph height in your real images.
//! 3. Persist the result with
//!    [`crate::ocr::prototypes::save_prototypes_json`] so you don't retrain
//!    on every run.
//! 4. Load at runtime via
//!    [`crate::ocr::prototypes::load_prototypes_json`] and pass into
//!    [`crate::ocr::recognize::FeatureRecognizer::new`].
//!
//! Accuracy depends heavily on how closely the training font matches the
//! rendered text. One font produces one good recognizer; you'll need separate
//! prototype sets per font family.

use crate::ocr::error::{OcrError, OcrResult};
use crate::ocr::features::extract;
use crate::ocr::recognize::Prototype;
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{GrayImage, Luma};

/// Rasterize one glyph into a tight-cropped binary image.
///
/// Returns `None` when the font has no renderable outline for the character
/// (e.g. whitespace). The output image uses 0 for ink, 255 for background,
/// which matches the convention used by the recognizer's feature extractor.
pub fn rasterize_glyph(font: &FontRef<'_>, ch: char, px_size: f32) -> Option<GrayImage> {
    let scaled = font.as_scaled(PxScale::from(px_size));
    let glyph = scaled.scaled_glyph(ch);
    let outlined = scaled.outline_glyph(glyph)?;
    let bounds = outlined.px_bounds();
    let w = bounds.width().ceil() as u32;
    let h = bounds.height().ceil() as u32;
    if w == 0 || h == 0 {
        return None;
    }

    // Start with a white canvas, draw ink by subtracting coverage.
    let mut img = GrayImage::from_pixel(w, h, Luma([255]));
    outlined.draw(|x, y, coverage| {
        if x < w && y < h {
            let ink = (coverage.clamp(0.0, 1.0) * 255.0) as u8;
            let current = img.get_pixel(x, y)[0];
            let new_val = current.saturating_sub(ink);
            img.put_pixel(x, y, Luma([new_val]));
        }
    });

    // Binarize via Otsu so features line up with the recognizer's post-
    // preprocess crops.
    let t = imageproc::contrast::otsu_level(&img);
    let bin = imageproc::contrast::threshold(&img, t);
    Some(bin)
}

/// Build a labeled prototype set from raw TTF/OTF bytes.
///
/// Characters without outlines (spaces, undefined code points) are silently
/// skipped. Duplicate labels in `chars` are preserved so callers can train
/// multiple variants per class if desired.
pub fn train_from_ttf_bytes(
    font_bytes: &[u8],
    chars: &str,
    px_size: f32,
) -> OcrResult<Vec<Prototype>> {
    let font = FontRef::try_from_slice(font_bytes)
        .map_err(|e| OcrError::Config(format!("load font: {e}")))?;

    let mut out = Vec::with_capacity(chars.chars().count());
    for ch in chars.chars() {
        if let Some(img) = rasterize_glyph(&font, ch, px_size) {
            out.push(Prototype {
                label: ch,
                features: extract(&img),
            });
        }
    }
    if out.is_empty() {
        return Err(OcrError::Config(format!(
            "font produced no usable outlines for the supplied character set ({} chars)",
            chars.chars().count()
        )));
    }
    Ok(out)
}

/// Convenience wrapper around [`train_from_ttf_bytes`] that reads the font
/// from disk.
pub fn train_from_ttf_path(
    font_path: impl AsRef<std::path::Path>,
    chars: &str,
    px_size: f32,
) -> OcrResult<Vec<Prototype>> {
    let bytes = std::fs::read(font_path.as_ref()).map_err(OcrError::Io)?;
    train_from_ttf_bytes(&bytes, chars, px_size)
}

/// Train prototypes at several pixel sizes and return the concatenated set.
///
/// Features are meant to be scale-invariant, but the feature extractor's zone
/// and projection bins quantize differently at small vs. large sizes. Stacking
/// prototypes across scales lets the 1-NN classifier match whichever trained
/// size is closest to the glyph it sees at runtime.
pub fn train_multiscale(
    font_bytes: &[u8],
    chars: &str,
    px_sizes: &[f32],
) -> OcrResult<Vec<Prototype>> {
    if px_sizes.is_empty() {
        return Err(OcrError::Config("at least one px size required".into()));
    }
    let mut out = Vec::new();
    for &px in px_sizes {
        let mut set = train_from_ttf_bytes(font_bytes, chars, px)?;
        out.append(&mut set);
    }
    Ok(out)
}

/// File-path variant of [`train_multiscale`].
pub fn train_multiscale_from_path(
    font_path: impl AsRef<std::path::Path>,
    chars: &str,
    px_sizes: &[f32],
) -> OcrResult<Vec<Prototype>> {
    let bytes = std::fs::read(font_path.as_ref()).map_err(OcrError::Io)?;
    train_multiscale(&bytes, chars, px_sizes)
}

/// Train across multiple fonts AND multiple sizes, merging everything into a
/// single prototype set. Use when the runtime input may be in any of several
/// typefaces — the 1-NN / k-NN classifier simply picks whichever bundled
/// glyph is closest in feature space.
pub fn train_multifont_multiscale(
    fonts: &[(&str, &[u8])],
    chars: &str,
    px_sizes: &[f32],
) -> OcrResult<Vec<Prototype>> {
    if fonts.is_empty() {
        return Err(OcrError::Config("at least one font required".into()));
    }
    let mut out = Vec::new();
    for (name, bytes) in fonts {
        for &px in px_sizes {
            match train_from_ttf_bytes(bytes, chars, px) {
                Ok(mut set) => out.append(&mut set),
                Err(e) => eprintln!(
                    "omniparse: skipping font '{}' @ {}px: {}",
                    name, px, e
                ),
            }
        }
    }
    if out.is_empty() {
        return Err(OcrError::Config(
            "no usable font/size combinations produced prototypes".into(),
        ));
    }
    Ok(out)
}

/// File-path variant of [`train_multifont_multiscale`].
pub fn train_multifont_multiscale_from_paths(
    font_paths: &[&str],
    chars: &str,
    px_sizes: &[f32],
) -> OcrResult<Vec<Prototype>> {
    let mut buffers: Vec<(String, Vec<u8>)> = Vec::new();
    for p in font_paths {
        let bytes = std::fs::read(p).map_err(OcrError::Io)?;
        buffers.push((p.to_string(), bytes));
    }
    let refs: Vec<(&str, &[u8])> = buffers
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    train_multifont_multiscale(&refs, chars, px_sizes)
}

/// Default character set used when callers don't supply one: uppercase,
/// lowercase, digits, and common ASCII punctuation.
pub const DEFAULT_CHAR_SET: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.,:;!?'\"()[]{}-_/\\@#&*+=<>%$";
