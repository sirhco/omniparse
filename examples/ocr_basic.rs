//! End-to-end OCR example using the bundled classical pipeline.
//!
//! Run with:
//!
//! ```sh
//! cargo run --features ocr --example ocr_basic -- path/to/image.png
//! ```
//!
//! Or without a path argument, the example synthesizes a small bitmap from
//! one of the bundled glyph prototypes and recognizes it as a smoke test.

#[cfg(feature = "ocr")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use image::{DynamicImage, GrayImage, Luma};
    use omniparse::ocr::{prototypes::BUNDLED_GLYPHS, OcrEngine};

    let path = std::env::args().nth(1);

    let image = match path.as_deref() {
        Some(p) => image::open(p)?,
        None => {
            println!("no path supplied; synthesizing an 'H' bitmap as a smoke test");
            let (_, art) = BUNDLED_GLYPHS.iter().find(|(c, _)| *c == 'H').unwrap();
            let rows: Vec<&str> = art
                .lines()
                .map(str::trim_end)
                .filter(|r| !r.is_empty())
                .collect();
            let scale = 3u32;
            let margin = 8u32;
            let gw = rows[0].len() as u32 * scale + 2 * margin;
            let gh = rows.len() as u32 * scale + 2 * margin;
            let mut img = GrayImage::from_pixel(gw, gh, Luma([255]));
            for (y, row) in rows.iter().enumerate() {
                for (x, ch) in row.chars().enumerate() {
                    if ch == '#' {
                        for dy in 0..scale {
                            for dx in 0..scale {
                                img.put_pixel(
                                    margin + x as u32 * scale + dx,
                                    margin + y as u32 * scale + dy,
                                    Luma([0]),
                                );
                            }
                        }
                    }
                }
            }
            DynamicImage::ImageLuma8(img)
        }
    };

    let engine = OcrEngine::new();
    let out = engine.recognize(image)?;

    println!("recognized text: {}", out.text);
    println!("mean confidence: {:.2}", out.mean_confidence);
    println!("line count:      {}", out.lines.len());
    for line in &out.lines {
        println!(
            "  '{}' conf={:.2} @ ({},{}) {}x{}",
            line.text,
            line.confidence,
            line.region.x,
            line.region.y,
            line.region.width,
            line.region.height,
        );
    }
    Ok(())
}

#[cfg(not(feature = "ocr"))]
fn main() {
    eprintln!("rebuild with --features ocr to run this example");
    std::process::exit(1);
}
