//! Generate OCR-testable fixtures under `test_data/ocr/`.
//!
//! Pure-Rust: renders text into a bitmap via `ab_glyph`, writes PNG / JPEG,
//! and wraps a JPEG into a single-page "scanned" PDF using `lopdf`. The
//! resulting files exercise both the image-OCR path and the PDF image-OCR
//! path in the parsers, and are small enough to commit.
//!
//! ```sh
//! cargo run --features ocr-train --example create_ocr_fixtures
//! # or with a custom font / different output dir
//! cargo run --features ocr-train --example create_ocr_fixtures -- \
//!     /System/Library/Fonts/Supplemental/Arial.ttf  test_data/ocr
//! ```

#[cfg(feature = "ocr-train")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ab_glyph::{FontRef, PxScale, ScaleFont};
    use image::{DynamicImage, GrayImage, Luma};
    use lopdf::{dictionary, Document, Object, Stream};
    use std::fs;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    let mut args = std::env::args().skip(1);
    let font_path = args.next().unwrap_or_else(|| {
        "/System/Library/Fonts/Supplemental/Arial.ttf".into()
    });
    let out_dir = args.next().unwrap_or_else(|| "test_data/ocr".into());
    let out_dir: PathBuf = PathBuf::from(out_dir);
    fs::create_dir_all(&out_dir)?;

    let font_bytes = fs::read(&font_path)?;
    let font = FontRef::try_from_slice(&font_bytes)?;

    fn render_lines(font: &impl ab_glyph::Font, lines: &[&str], px: f32) -> GrayImage {
        let scaled = font.as_scaled(PxScale::from(px));
        let line_h = (scaled.ascent().ceil() - scaled.descent().floor() + 8.0) as u32;
        let pad: u32 = 24;
        let widths: Vec<u32> = lines
            .iter()
            .map(|line| {
                line.chars()
                    .map(|c| scaled.h_advance(scaled.scaled_glyph(c).id).ceil() as u32)
                    .sum::<u32>()
            })
            .collect();
        let w = pad * 2 + *widths.iter().max().unwrap_or(&100);
        let h = pad * 2 + line_h * lines.len() as u32;
        let mut img = GrayImage::from_pixel(w, h, Luma([255]));

        for (row, line) in lines.iter().enumerate() {
            let mut pen_x = pad as i32;
            let pen_y = pad as i32
                + (row as u32 * line_h) as i32
                + scaled.ascent().ceil() as i32;
            for ch in line.chars() {
                let glyph = scaled.scaled_glyph(ch);
                let advance = scaled.h_advance(glyph.id);
                if let Some(outlined) = scaled.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    outlined.draw(|gx, gy, coverage| {
                        let ix = pen_x + bounds.min.x as i32 + gx as i32;
                        let iy = pen_y + bounds.min.y as i32 + gy as i32;
                        if ix < 0 || iy < 0 || ix >= w as i32 || iy >= h as i32 {
                            return;
                        }
                        let cur = img.get_pixel(ix as u32, iy as u32)[0];
                        let ink = (coverage.clamp(0.0, 1.0) * 255.0) as u8;
                        img.put_pixel(ix as u32, iy as u32, Luma([cur.saturating_sub(ink)]));
                    });
                }
                pen_x += advance.ceil() as i32;
            }
        }
        img
    }

    fn write_png(img: &GrayImage, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        DynamicImage::ImageLuma8(img.clone()).save(path)?;
        println!("wrote {}", path.display());
        Ok(())
    }

    fn write_jpeg(img: &GrayImage, path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Write a JPEG via the image crate. Use Rgb to keep DCTDecode happy
        // when later embedded in a PDF (DeviceRGB / 8 bpc).
        let dyn_img = DynamicImage::ImageLuma8(img.clone()).to_rgb8();
        let mut bytes: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(&mut bytes);
        DynamicImage::ImageRgb8(dyn_img.clone())
            .write_to(&mut cursor, image::ImageFormat::Jpeg)?;
        fs::write(path, &bytes)?;
        println!("wrote {} ({} KB)", path.display(), bytes.len() / 1024);
        Ok(bytes)
    }

    /// Build a minimal single-page PDF that draws a JPEG (DCTDecode) the
    /// full size of the page. The omniparse PDF parser's OCR path keys off
    /// DCTDecode images, so this exercises the scanned-PDF code.
    fn build_image_pdf(
        jpeg_bytes: &[u8],
        w: u32,
        h: u32,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = Document::with_version("1.5");

        let image_id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => Object::Integer(w as i64),
                "Height" => Object::Integer(h as i64),
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => Object::Integer(8),
                "Filter" => "DCTDecode",
            },
            jpeg_bytes.to_vec(),
        ));

        let content = format!("q\n{w} 0 0 {h} 0 0 cm\n/Im0 Do\nQ");
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            content.into_bytes(),
        ));

        let resources_id = doc.add_object(dictionary! {
            "XObject" => dictionary! {
                "Im0" => Object::Reference(image_id),
            },
        });

        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => Object::Array(vec![
                0.into(),
                0.into(),
                Object::Integer(w as i64),
                Object::Integer(h as i64),
            ]),
            "Resources" => Object::Reference(resources_id),
            "Contents" => Object::Reference(content_id),
        });

        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => Object::Array(vec![Object::Reference(page_id)]),
                "Count" => Object::Integer(1),
            }),
        );

        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        });

        doc.trailer
            .set("Root", Object::Reference(catalog_id));

        doc.save(path)?;
        println!("wrote {}", path.display());
        Ok(())
    }

    // 1. Single line — easiest case.
    let img = render_lines(&font, &["HELLO WORLD"], 64.0);
    write_png(&img, &out_dir.join("hello_world.png"))?;
    let _ = write_jpeg(&img, &out_dir.join("hello_world.jpg"))?;

    // 2. Multi-line + mixed case — exercises layout grouping.
    let img = render_lines(
        &font,
        &[
            "Omniparse OCR test fixture",
            "The quick brown fox jumps over the lazy dog.",
            "1234567890",
        ],
        48.0,
    );
    write_png(&img, &out_dir.join("multi_line.png"))?;

    // 3. Scanned-style PDF: wrap a JPEG of "HELLO WORLD" into a one-page PDF
    //    with no text layer. PDF parser's OCR path runs against DCTDecode
    //    images.
    let img = render_lines(&font, &["HELLO WORLD"], 96.0);
    let (w, h) = (img.width(), img.height());
    let jpeg = write_jpeg(&img, &out_dir.join("scanned_source.jpg"))?;
    build_image_pdf(&jpeg, w, h, &out_dir.join("scanned.pdf"))?;
    // The intermediate JPEG isn't needed for the PDF test; remove it.
    fs::remove_file(out_dir.join("scanned_source.jpg")).ok();

    println!("\nFixtures written to {}", out_dir.display());
    println!(
        "Try them:  OMNIPARSE_OCR=ml omniparse {}/hello_world.png",
        out_dir.display()
    );

    Ok(())
}

#[cfg(not(feature = "ocr-train"))]
fn main() {
    eprintln!(
        "create_ocr_fixtures requires the `ocr-train` feature.\n\
         Run: cargo run --features ocr-train --example create_ocr_fixtures"
    );
    std::process::exit(2);
}
