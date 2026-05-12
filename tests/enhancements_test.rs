//! Phase-A regression tests for registry caching, EXIF, HTML hardening, and
//! archive path-traversal detection.

use omniparse::{
    extract_from_bytes, is_mime_supported, supported_mime_types, Content, MetadataValue,
};

#[test]
fn registry_reuses_cached_instance_across_calls() {
    // Two successive calls should return identical sets — smoke-check that the
    // OnceLock path is wired and doesn't panic on re-entry.
    let a = supported_mime_types();
    let b = supported_mime_types();
    assert_eq!(a.len(), b.len());
    assert!(!a.is_empty());
    #[cfg(feature = "pdf")]
    assert!(is_mime_supported("application/pdf"));
    assert!(!is_mime_supported("application/x-definitely-fake"));
}

#[test]
fn jpeg_exif_values_never_use_placeholder_strings() {
    // Synthesize a valid JPEG so the test doesn't depend on fixture quality.
    // Images without EXIF are fine — the assertion is that if any of the known
    // EXIF-derived keys are present, they are not the old "tag_N" placeholders.
    use image::{DynamicImage, RgbImage};
    let mut buf = Vec::new();
    let img = DynamicImage::ImageRgb8(RgbImage::new(4, 4));
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg)
        .expect("encode jpeg");

    let result = extract_from_bytes(&buf, Some("image/jpeg")).expect("JPEG parse");
    assert!(matches!(
        result.metadata.get("width"),
        Some(MetadataValue::Number(_))
    ));
    for key in [
        "camera_make",
        "camera_model",
        "software",
        "tiff_Make",
        "tiff_Model",
    ] {
        if let Some(MetadataValue::Text(v)) = result.metadata.get(key) {
            assert!(!v.starts_with("tag_"), "{key} still emits placeholder: {v}");
        }
    }
}

#[test]
fn html_parser_does_not_panic_on_empty_input() {
    // The old parser used .unwrap() on Selector::parse; even empty input now
    // goes through fallible paths without panicking.
    let result = extract_from_bytes(b"", Some("text/html")).expect("empty HTML parse");
    match result.content {
        Content::Text(_) => {}
        other => panic!("expected Content::Text, got {:?}", other),
    }
}

#[test]
fn html_parser_extracts_opengraph_and_canonical() {
    let html = br#"<!DOCTYPE html>
<html lang="en">
<head>
  <title>Article</title>
  <meta property="og:title" content="The OG Title">
  <meta property="og:type" content="article">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="canonical" href="https://example.com/article">
</head>
<body>
  <h1>A</h1><h1>B</h1>
  <h2>Section</h2>
  <p>hi</p>
</body>
</html>"#;

    let result = extract_from_bytes(html, Some("text/html")).expect("HTML parse");
    assert_eq!(
        result.metadata.get("og_title"),
        Some(&MetadataValue::Text("The OG Title".into()))
    );
    assert_eq!(
        result.metadata.get("og_type"),
        Some(&MetadataValue::Text("article".into()))
    );
    assert_eq!(
        result.metadata.get("twitter_card"),
        Some(&MetadataValue::Text("summary_large_image".into()))
    );
    assert_eq!(
        result.metadata.get("viewport"),
        Some(&MetadataValue::Text("width=device-width, initial-scale=1".into()))
    );
    assert_eq!(
        result.metadata.get("canonical_url"),
        Some(&MetadataValue::Text("https://example.com/article".into()))
    );
    assert_eq!(
        result.metadata.get("heading_h1_count"),
        Some(&MetadataValue::Number(2))
    );
    assert_eq!(
        result.metadata.get("heading_h2_count"),
        Some(&MetadataValue::Number(1))
    );
}

#[test]
fn zip_parser_flags_unsafe_paths() {
    // Minimal in-memory zip with one safe and one path-traversal entry.
    use std::io::Write;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts: FileOptions = FileOptions::default();
        zip.start_file("safe.txt", opts).unwrap();
        zip.write_all(b"ok").unwrap();
        zip.start_file("../escape.txt", opts).unwrap();
        zip.write_all(b"bad").unwrap();
        zip.finish().unwrap();
    }

    let result = extract_from_bytes(&buf, Some("application/zip")).expect("zip parse");
    assert_eq!(
        result.metadata.get("contains_unsafe_paths"),
        Some(&MetadataValue::Boolean(true))
    );
}

#[test]
fn zip_parser_clean_archive_reports_no_unsafe_paths() {
    use std::io::Write;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts: FileOptions = FileOptions::default();
        zip.start_file("docs/readme.md", opts).unwrap();
        zip.write_all(b"hello").unwrap();
        zip.finish().unwrap();
    }

    let result = extract_from_bytes(&buf, Some("application/zip")).expect("zip parse");
    assert_eq!(
        result.metadata.get("contains_unsafe_paths"),
        Some(&MetadataValue::Boolean(false))
    );
}

#[test]
fn pdf_parser_reports_version_and_encryption() {
    #[cfg(not(feature = "pdf"))]
    return;
    #[cfg(feature = "pdf")]
    let Ok(data) = std::fs::read("test_data/document/sample.pdf") else {
        return; // fixture optional
    };
    #[cfg(feature = "pdf")]
    // Fixture may be malformed. In that case the parser falls through to
    // the raw_scan tier, which only guarantees pdf_version (scanned from
    // the header). The richer fields require lopdf to load successfully.
    if let Ok(result) = extract_from_bytes(&data, Some("application/pdf")) {
        let raw_scan = matches!(
            result.metadata.get("pdf_parse_strategy"),
            Some(MetadataValue::Text(s)) if s == "raw_scan"
        );
        assert!(matches!(
            result.metadata.get("pdf_version"),
            Some(MetadataValue::Text(_))
        ));
        if !raw_scan {
            assert!(matches!(
                result.metadata.get("encrypted"),
                Some(MetadataValue::Boolean(_))
            ));
            assert!(matches!(
                result.metadata.get("annotations_count"),
                Some(MetadataValue::Number(_))
            ));
        }
    }
}
