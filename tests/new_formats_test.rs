//! Round-trip tests for the v0.3.0 format parsers (Markdown, SVG, WebP,
//! EPUB, MP3). Each test is feature-gated so users can disable individual
//! parsers without breaking the suite.

use omniparse::{extract_from_bytes, Content, MetadataValue};

#[cfg(feature = "markdown")]
#[test]
fn markdown_extracts_title_and_heading_counts() {
    let md = b"# Hello\n\nSome intro paragraph.\n\n## Sub\n\n- item\n\n```rust\nfn main() {}\n```\n\n[link](https://example.com)";
    let result = extract_from_bytes(md, Some("text/markdown")).unwrap();
    match &result.content {
        Content::Text(t) => {
            assert!(t.contains("Hello"));
            assert!(t.contains("intro"));
        }
        other => panic!("expected text, got {other:?}"),
    }
    assert_eq!(result.metadata.title(), Some("Hello"));
    assert_eq!(
        result.metadata.get("heading_h1_count"),
        Some(&MetadataValue::Number(1))
    );
    assert_eq!(
        result.metadata.get("heading_h2_count"),
        Some(&MetadataValue::Number(1))
    );
    assert_eq!(
        result.metadata.get("code_block_count"),
        Some(&MetadataValue::Number(1))
    );
    assert_eq!(
        result.metadata.get("link_count"),
        Some(&MetadataValue::Number(1))
    );
}

#[cfg(feature = "svg")]
#[test]
fn svg_extracts_title_desc_and_element_counts() {
    let svg = br#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">
  <title>My Diagram</title>
  <desc>A quick sketch.</desc>
  <rect x="0" y="0" width="50" height="50"/>
  <rect x="50" y="50" width="50" height="50"/>
  <circle cx="25" cy="75" r="10"/>
  <text x="10" y="20">hello world</text>
</svg>"#;
    let result = extract_from_bytes(svg, Some("image/svg+xml")).unwrap();
    assert_eq!(result.metadata.title(), Some("My Diagram"));
    assert_eq!(
        result.metadata.get("description"),
        Some(&MetadataValue::Text("A quick sketch.".into()))
    );
    assert_eq!(
        result.metadata.get("viewbox"),
        Some(&MetadataValue::Text("0 0 100 100".into()))
    );
    assert_eq!(
        result.metadata.get("element_rect_count"),
        Some(&MetadataValue::Number(2))
    );
    assert_eq!(
        result.metadata.get("element_circle_count"),
        Some(&MetadataValue::Number(1))
    );
    if let Content::Text(t) = &result.content {
        assert!(t.contains("hello world"));
    } else {
        panic!("expected text content");
    }
}

#[cfg(feature = "webp")]
#[test]
fn webp_extracts_basic_dimensions() {
    use image::{DynamicImage, RgbaImage};
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 8, image::Rgba([255, 0, 0, 255])));
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::WebP)
        .unwrap();
    let result = extract_from_bytes(&buf, Some("image/webp")).unwrap();
    assert_eq!(result.metadata.get("width"), Some(&MetadataValue::Number(8)));
    assert_eq!(
        result.metadata.get("height"),
        Some(&MetadataValue::Number(8))
    );
}

#[cfg(feature = "epub")]
#[test]
fn epub_parser_accepts_minimal_fixture_when_available() {
    let Ok(bytes) = std::fs::read("test_data/document/sample.epub") else {
        return; // fixture optional
    };
    // If a fixture exists, we only assert that parsing succeeds and returns
    // spine/resource counts — content quality depends on the specific file.
    let result = extract_from_bytes(&bytes, Some("application/epub+zip")).unwrap();
    assert!(matches!(
        result.metadata.get("spine_count"),
        Some(MetadataValue::Number(_))
    ));
    assert!(matches!(
        result.metadata.get("resource_count"),
        Some(MetadataValue::Number(_))
    ));
}

#[cfg(feature = "mp3")]
#[test]
fn mp3_parser_reads_id3_fixture_when_available() {
    let Ok(bytes) = std::fs::read("test_data/audio/sample.mp3") else {
        return; // fixture optional
    };
    // Parse should succeed whether or not the fixture has ID3 tags.
    let result = extract_from_bytes(&bytes, Some("audio/mpeg"));
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
}
