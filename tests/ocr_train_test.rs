//! Tests for the TTF-based prototype trainer (`ocr-train` feature).

#![cfg(feature = "ocr-train")]

use omniparse::ocr::recognize::{FeatureRecognizer, Recognizer};
use omniparse::ocr::layout::TextRegion;
use omniparse::ocr::train::{rasterize_glyph, train_from_ttf_bytes};
use ab_glyph::FontRef;

const FONT_PATHS: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/Library/Fonts/Arial.ttf",
];

fn load_system_font() -> Option<Vec<u8>> {
    for path in FONT_PATHS {
        if let Ok(bytes) = std::fs::read(path) {
            return Some(bytes);
        }
    }
    None
}

#[test]
fn rasterize_returns_non_empty_image_for_visible_glyph() {
    let Some(font_bytes) = load_system_font() else {
        eprintln!("skipping: no system font found");
        return;
    };
    let font = FontRef::try_from_slice(&font_bytes).unwrap();
    let img = rasterize_glyph(&font, 'H', 48.0).expect("rasterize H");
    assert!(img.width() > 0 && img.height() > 0);
}

#[test]
fn rasterize_skips_whitespace() {
    let Some(font_bytes) = load_system_font() else { return };
    let font = FontRef::try_from_slice(&font_bytes).unwrap();
    // Space has no outline — helper should return None rather than an empty image.
    assert!(rasterize_glyph(&font, ' ', 48.0).is_none());
}

#[test]
fn multiscale_concatenates_prototype_sets() {
    use omniparse::ocr::train::train_multiscale;
    let Some(font_bytes) = load_system_font() else { return };
    let chars = "ABC";
    let sizes = [24.0f32, 48.0, 96.0];
    let protos = train_multiscale(&font_bytes, chars, &sizes).unwrap();
    // Three chars × three sizes = nine prototypes (assuming all outlines valid).
    assert_eq!(protos.len(), chars.chars().count() * sizes.len());
}

#[test]
fn train_then_recognize_same_glyph_returns_training_label() {
    let Some(font_bytes) = load_system_font() else { return };
    let chars = "HELLO";
    let px = 48.0;
    let protos = train_from_ttf_bytes(&font_bytes, chars, px).expect("train");
    assert_eq!(protos.len(), chars.chars().count());

    let font = FontRef::try_from_slice(&font_bytes).unwrap();
    let img = rasterize_glyph(&font, 'E', px).unwrap();
    let region = TextRegion {
        x: 0,
        y: 0,
        width: img.width(),
        height: img.height(),
    };
    let recog = FeatureRecognizer::new(protos);
    let line = recog.recognize(&img, &region).unwrap();
    assert_eq!(line.text, "E");
    assert!(line.confidence > 0.5, "low confidence: {}", line.confidence);
}
