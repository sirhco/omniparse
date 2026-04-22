//! End-to-end pipeline validator.
//!
//! Renders a known reference string using the user-supplied font, runs the
//! full OCR pipeline against the rendered bitmap, and reports per-character
//! accuracy. Use this to confirm the classical pipeline is functioning on a
//! controlled input that exactly matches your trained prototypes.
//!
//! Run with:
//!
//! ```sh
//! cargo run --features ocr-train --example ocr_validate -- \
//!     /System/Library/Fonts/Supplemental/Arial.ttf \
//!     "HELLO WORLD" \
//!     48
//! ```
//!
//! With no arguments, defaults to Arial + "ABC123" at 48px.

#[cfg(feature = "ocr-train")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
    use image::{DynamicImage, GrayImage, Luma};
    use omniparse::ocr::{
        recognize::FeatureRecognizer,
        train::{train_from_ttf_bytes, DEFAULT_CHAR_SET},
        OcrEngine, OcrEngineBuilder,
    };

    let mut args = std::env::args().skip(1);
    let font_path = args
        .next()
        .unwrap_or_else(|| "/System/Library/Fonts/Supplemental/Arial.ttf".into());
    let target_text = args.next().unwrap_or_else(|| "ABC123".into());
    let px_size: f32 = args
        .next()
        .and_then(|v| v.parse().ok())
        .unwrap_or(48.0);

    let font_bytes = std::fs::read(&font_path)?;
    let font = FontRef::try_from_slice(&font_bytes)?;

    println!(
        "validator: font={} text={:?} px_size={}",
        font_path, target_text, px_size
    );

    // 1. Render target_text into a single-line bitmap.
    let scaled = font.as_scaled(PxScale::from(px_size));
    let mut pen_x = 20i32;
    let pen_y = (scaled.ascent().ceil() as i32) + 20;
    let canvas_w: u32 = 20 + target_text
        .chars()
        .map(|c| scaled.h_advance(scaled.scaled_glyph(c).id).ceil() as u32)
        .sum::<u32>()
        + 20;
    let canvas_h: u32 = (scaled.ascent().ceil() as i32 - scaled.descent().floor() as i32) as u32 + 40;
    let mut img = GrayImage::from_pixel(canvas_w, canvas_h, Luma([255]));
    for ch in target_text.chars() {
        let glyph = scaled.scaled_glyph(ch);
        let advance = scaled.h_advance(glyph.id);
        if let Some(outlined) = scaled.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                let ix = pen_x + bounds.min.x as i32 + gx as i32;
                let iy = pen_y + bounds.min.y as i32 + gy as i32;
                if ix < 0 || iy < 0 || ix >= canvas_w as i32 || iy >= canvas_h as i32 {
                    return;
                }
                let cur = img.get_pixel(ix as u32, iy as u32)[0];
                let ink = (coverage.clamp(0.0, 1.0) * 255.0) as u8;
                img.put_pixel(ix as u32, iy as u32, Luma([cur.saturating_sub(ink)]));
            });
        }
        pen_x += advance.ceil() as i32;
    }

    // 2. Train prototypes from the SAME font — guarantees feature-space
    //    alignment; a failure here indicates a pipeline bug, not a font
    //    mismatch.
    let prototypes = train_from_ttf_bytes(&font_bytes, DEFAULT_CHAR_SET, px_size)?;
    println!("trained {} prototypes", prototypes.len());

    // 3. Build engine with default preprocessor, custom recognizer.
    let engine: OcrEngine = OcrEngineBuilder::default()
        .recognizer(FeatureRecognizer::new(prototypes).with_k(3).build_kdtree())
        .build();

    // 4. Recognize.
    let out = engine.recognize(DynamicImage::ImageLuma8(img.clone()))?;
    println!("recognized: {:?}", out.text);
    println!("mean_confidence: {:.3}", out.mean_confidence);
    println!("lines: {}", out.lines.len());

    // 5. Per-character accuracy against the target.
    let expected: Vec<char> = target_text.chars().filter(|c| !c.is_whitespace()).collect();
    let actual: Vec<char> = out.text.chars().filter(|c| !c.is_whitespace()).collect();
    let matched = expected
        .iter()
        .zip(actual.iter())
        .filter(|(a, b)| a == b)
        .count();
    let total = expected.len();
    println!(
        "accuracy: {}/{} ({:.1}%)",
        matched,
        total,
        100.0 * matched as f32 / total.max(1) as f32
    );

    // 6. Also dump the rendered canvas for visual inspection.
    if let Ok(dir) = std::env::var("OMNIPARSE_OCR_DEBUG_DIR") {
        let _ = std::fs::create_dir_all(&dir);
        let _ = img.save(std::path::Path::new(&dir).join("validate_input.png"));
        println!("wrote {}/validate_input.png", dir);
    }

    Ok(())
}

#[cfg(not(feature = "ocr-train"))]
fn main() {
    eprintln!("rebuild with --features ocr-train");
    std::process::exit(1);
}
