//! Train OCR glyph prototypes from a TrueType/OpenType font file.
//!
//! Requires the `ocr-train` Cargo feature.
//!
//! ```sh
//! cargo run --features ocr-train --example train_prototypes -- \
//!     /path/to/font.ttf \
//!     prototypes.json \
//!     48
//! ```
//!
//! The third argument is the pixel size at which glyphs are rasterized. Pick a
//! value close to the rendered glyph height in your real images (e.g. 48 for
//! body copy at 150 DPI).
//!
//! A fourth optional argument overrides the default character set.
//!
//! Use the result at runtime with:
//!
//! ```sh
//! OMNIPARSE_OCR=classical OMNIPARSE_OCR_PROTOTYPES=/path/to/prototypes.json \
//!     cargo run --features ocr --release -- some-image.jpg
//! ```

#[cfg(feature = "ocr-train")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use omniparse::ocr::prototypes::save_prototypes_json;
    use omniparse::ocr::train::{train_multifont_multiscale_from_paths, DEFAULT_CHAR_SET};

    let mut args = std::env::args().skip(1);
    let fonts_arg = args.next().ok_or(
        "usage: train_prototypes <font1.ttf[:font2.ttf:...]> <out.json> <px-sizes> [chars]\n\
         - Multiple fonts separated by ':' train a merged prototype set\n\
         - <px-sizes> is a comma-separated list, e.g. 24,48,96",
    )?;
    let out_path = args.next().ok_or("missing output JSON path")?;
    let px_sizes_arg = args.next().ok_or("missing px-sizes list")?;
    let chars: String = args.next().unwrap_or_else(|| DEFAULT_CHAR_SET.to_string());

    let font_paths: Vec<&str> = fonts_arg.split(':').filter(|s| !s.is_empty()).collect();
    let px_sizes: Vec<f32> = px_sizes_arg
        .split(',')
        .map(|s| s.trim().parse::<f32>())
        .collect::<Result<_, _>>()?;

    println!(
        "training {} glyphs from {} font(s) at sizes {:?}",
        chars.chars().count(),
        font_paths.len(),
        px_sizes
    );
    for path in &font_paths {
        println!("  - {}", path);
    }
    let prototypes = train_multifont_multiscale_from_paths(&font_paths, &chars, &px_sizes)?;
    println!("produced {} prototypes", prototypes.len());

    save_prototypes_json(&prototypes, &out_path)?;
    println!("wrote {}", out_path);
    println!();
    println!("to use at runtime:");
    println!("  OMNIPARSE_OCR=classical OMNIPARSE_OCR_PROTOTYPES={out_path} \\");
    println!("      cargo run --features ocr --release -- image.jpg");
    Ok(())
}

#[cfg(not(feature = "ocr-train"))]
fn main() {
    eprintln!("rebuild with --features ocr-train to run this example");
    std::process::exit(1);
}
