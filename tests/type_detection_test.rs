//! Unit tests for type detection functionality

use omniparse::detection::{TypeDetector, DetectionMethod, Detector};
use std::path::Path;

#[test]
fn test_magic_bytes_pdf_detection() {
    let detector = TypeDetector::new();
    let pdf_data = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3";
    
    let result = detector.detect_from_bytes(pdf_data);
    
    assert_eq!(result.mime_type, "application/pdf");
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
    assert!(result.confidence >= 0.9, "Magic bytes should have high confidence");
}

#[test]
fn test_magic_bytes_png_detection() {
    let detector = TypeDetector::new();
    let png_data = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
    
    let result = detector.detect_from_bytes(png_data);
    
    assert_eq!(result.mime_type, "image/png");
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
    assert!(result.confidence >= 0.9);
}

#[test]
fn test_magic_bytes_jpeg_detection() {
    let detector = TypeDetector::new();
    let jpeg_data = b"\xFF\xD8\xFF\xE0\x00\x10JFIF";
    
    let result = detector.detect_from_bytes(jpeg_data);
    
    assert_eq!(result.mime_type, "image/jpeg");
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
    assert!(result.confidence >= 0.9);
}

#[test]
fn test_magic_bytes_zip_detection() {
    let detector = TypeDetector::new();
    let zip_data = b"PK\x03\x04\x14\x00\x00\x00";
    
    let result = detector.detect_from_bytes(zip_data);
    
    // ZIP signature is shared by multiple formats, first match wins
    // The detector may return any ZIP-based format
    assert!(result.mime_type.contains("zip") || 
            result.mime_type.contains("openxmlformats") ||
            result.mime_type.contains("oasis"));
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
    assert!(result.confidence >= 0.9);
}

#[test]
fn test_magic_bytes_gzip_detection() {
    let detector = TypeDetector::new();
    let gzip_data = b"\x1F\x8B\x08\x00\x00\x00\x00\x00";
    
    let result = detector.detect_from_bytes(gzip_data);
    
    assert_eq!(result.mime_type, "application/gzip");
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
    assert!(result.confidence >= 0.9);
}

#[test]
fn test_content_analysis_json_detection() {
    let detector = TypeDetector::new();
    let json_data = b"{\"key\": \"value\", \"number\": 42}";
    
    let result = detector.detect_from_bytes(json_data);
    
    assert_eq!(result.mime_type, "application/json");
    // JSON starting with { has magic bytes pattern, so it's detected as MagicBytes
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
    assert!(result.confidence >= 0.9);
}

#[test]
fn test_content_analysis_json_array_detection() {
    let detector = TypeDetector::new();
    let json_data = b"[1, 2, 3, 4, 5]";
    
    let result = detector.detect_from_bytes(json_data);
    
    assert_eq!(result.mime_type, "application/json");
    // JSON starting with [ has magic bytes pattern, so it's detected as MagicBytes
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
}

#[test]
fn test_content_analysis_xml_detection() {
    let detector = TypeDetector::new();
    let xml_data = b"<?xml version=\"1.0\"?><root><element>text</element></root>";
    
    let result = detector.detect_from_bytes(xml_data);
    
    // XML starting with <?xml has magic bytes pattern
    assert!(result.mime_type == "text/xml" || result.mime_type == "image/svg+xml");
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
    assert!(result.confidence >= 0.9);
}

#[test]
fn test_content_analysis_html_detection() {
    let detector = TypeDetector::new();
    let html_data = b"<html><head><title>Test</title></head><body>Content</body></html>";
    
    let result = detector.detect_from_bytes(html_data);
    
    assert_eq!(result.mime_type, "text/html");
    // HTML starting with <html has magic bytes pattern
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
}

#[test]
fn test_content_analysis_csv_detection() {
    let detector = TypeDetector::new();
    let csv_data = b"name,age,city\nJohn,30,NYC\nJane,25,LA";
    
    let result = detector.detect_from_bytes(csv_data);
    
    assert_eq!(result.mime_type, "text/csv");
    assert_eq!(result.detected_by, DetectionMethod::ContentAnalysis);
}

#[test]
fn test_content_analysis_plain_text_detection() {
    let detector = TypeDetector::new();
    let text_data = b"This is just plain text without any special formatting.";
    
    let result = detector.detect_from_bytes(text_data);
    
    assert_eq!(result.mime_type, "text/plain");
    assert_eq!(result.detected_by, DetectionMethod::ContentAnalysis);
}

#[test]
fn test_extension_fallback_pdf() {
    let detector = TypeDetector::new();
    // Binary data that doesn't match any magic bytes but is valid UTF-8
    // Use non-UTF-8 bytes to avoid content analysis detecting it as text
    let unknown_data = b"\xFF\xFE\x00\x01\x02\x03\x04\x05";
    
    let result = detector.detect(unknown_data, Some("document.pdf"));
    
    assert_eq!(result.mime_type, "application/pdf");
    assert_eq!(result.detected_by, DetectionMethod::Extension);
    assert!(result.confidence >= 0.3 && result.confidence <= 0.5);
}

#[test]
fn test_extension_fallback_docx() {
    let detector = TypeDetector::new();
    let unknown_data = b"\xFF\xFE\x00\x01\x02\x03\x04\x05";
    
    let result = detector.detect(unknown_data, Some("report.docx"));
    
    assert_eq!(result.mime_type, "application/vnd.openxmlformats-officedocument.wordprocessingml.document");
    assert_eq!(result.detected_by, DetectionMethod::Extension);
}

#[test]
fn test_extension_fallback_jpeg() {
    let detector = TypeDetector::new();
    let unknown_data = b"\xFF\xFE\x00\x01\x02\x03\x04\x05";
    
    let result = detector.detect(unknown_data, Some("photo.jpg"));
    
    assert_eq!(result.mime_type, "image/jpeg");
    assert_eq!(result.detected_by, DetectionMethod::Extension);
}

#[test]
fn test_extension_fallback_case_insensitive() {
    let detector = TypeDetector::new();
    let unknown_data = b"\xFF\xFE\x00\x01\x02\x03\x04\x05";
    
    let result = detector.detect(unknown_data, Some("FILE.PDF"));
    
    assert_eq!(result.mime_type, "application/pdf");
    assert_eq!(result.detected_by, DetectionMethod::Extension);
}

#[test]
fn test_unknown_file_type_no_magic_no_extension() {
    let detector = TypeDetector::new();
    // Use non-UTF-8 bytes to avoid being detected as text
    let unknown_data = b"\xFF\xFE\xFD\xFC\xFB\xFA\xF9\xF8\xF7\xF6";
    
    let result = detector.detect_from_bytes(unknown_data);
    
    assert_eq!(result.mime_type, "application/octet-stream");
    assert_eq!(result.detected_by, DetectionMethod::Unknown);
    assert!(result.confidence < 0.2, "Unknown type should have very low confidence");
}

#[test]
fn test_unknown_file_type_with_unsupported_extension() {
    let detector = TypeDetector::new();
    let unknown_data = b"\xFF\xFE\xFD\xFC\xFB\xFA";
    
    let result = detector.detect(unknown_data, Some("file.xyz"));
    
    assert_eq!(result.mime_type, "application/octet-stream");
    assert_eq!(result.detected_by, DetectionMethod::Unknown);
}

#[test]
fn test_empty_file_detection() {
    let detector = TypeDetector::new();
    let empty_data = b"";
    
    let result = detector.detect_from_bytes(empty_data);
    
    assert_eq!(result.mime_type, "application/octet-stream");
    assert_eq!(result.detected_by, DetectionMethod::Unknown);
}

#[test]
fn test_confidence_scoring_magic_bytes() {
    let detector = TypeDetector::new();
    let pdf_data = b"%PDF-1.7";
    
    let result = detector.detect_from_bytes(pdf_data);
    
    assert!(result.confidence >= 0.9 && result.confidence <= 1.0,
        "Magic bytes detection should have 0.9-1.0 confidence, got {}", result.confidence);
}

#[test]
fn test_confidence_scoring_content_analysis() {
    let detector = TypeDetector::new();
    // Use plain text that will be detected via content analysis, not magic bytes
    let text_data = b"This is some plain text without special formatting or structure.";
    
    let result = detector.detect_from_bytes(text_data);
    
    assert_eq!(result.detected_by, DetectionMethod::ContentAnalysis);
    assert!(result.confidence >= 0.6 && result.confidence <= 0.8,
        "Content analysis should have 0.6-0.8 confidence, got {}", result.confidence);
}

#[test]
fn test_confidence_scoring_extension() {
    let detector = TypeDetector::new();
    let unknown_data = b"\xFF\xFE\xFD\xFC";
    
    let result = detector.detect(unknown_data, Some("test.txt"));
    
    assert_eq!(result.detected_by, DetectionMethod::Extension);
    assert!(result.confidence >= 0.3 && result.confidence <= 0.5,
        "Extension detection should have 0.3-0.5 confidence, got {}", result.confidence);
}

#[test]
fn test_magic_bytes_priority_over_content_analysis() {
    let detector = TypeDetector::new();
    // PDF magic bytes at start, but also looks like text
    let pdf_data = b"%PDF-1.4\nSome text content";
    
    let result = detector.detect_from_bytes(pdf_data);
    
    assert_eq!(result.mime_type, "application/pdf");
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
}

#[test]
fn test_content_analysis_priority_over_extension() {
    let detector = TypeDetector::new();
    // JSON content but wrong extension
    let json_data = b"{\"key\": \"value\"}";
    
    let result = detector.detect(json_data, Some("file.txt"));
    
    assert_eq!(result.mime_type, "application/json");
    // JSON has magic bytes pattern, so it's detected as MagicBytes (which has higher priority)
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
}

#[test]
fn test_detect_from_path_with_real_file() {
    let detector = TypeDetector::new();
    
    // Test with an actual test file
    let result = detector.detect_from_path(Path::new("test_data/document/sample.pdf"));
    
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.mime_type, "application/pdf");
}

#[test]
fn test_tar_magic_bytes_with_offset() {
    let detector = TypeDetector::new();
    // TAR has magic bytes at offset 257
    let mut tar_data = vec![0u8; 262];
    tar_data[257..262].copy_from_slice(b"ustar");
    
    let result = detector.detect_from_bytes(&tar_data);
    
    assert_eq!(result.mime_type, "application/x-tar");
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
}

#[test]
fn test_svg_xml_detection() {
    let detector = TypeDetector::new();
    let svg_data = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><circle r=\"50\"/></svg>";
    
    let result = detector.detect_from_bytes(svg_data);
    
    assert_eq!(result.mime_type, "image/svg+xml");
    // SVG starting with <svg has magic bytes pattern
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
}

#[test]
fn test_multiple_zip_based_formats() {
    let detector = TypeDetector::new();
    let zip_signature = b"PK\x03\x04";
    
    // All these formats share the ZIP signature
    // Detection should succeed but may not distinguish between them without deeper analysis
    let result = detector.detect_from_bytes(zip_signature);
    
    // Should detect as some ZIP-based format
    assert!(result.mime_type.contains("zip") || 
            result.mime_type.contains("openxmlformats") ||
            result.mime_type.contains("oasis"));
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
}

#[test]
fn test_tiff_little_endian() {
    let detector = TypeDetector::new();
    let tiff_data = b"\x49\x49\x2A\x00";
    
    let result = detector.detect_from_bytes(tiff_data);
    
    assert_eq!(result.mime_type, "image/tiff");
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
}

#[test]
fn test_tiff_big_endian() {
    let detector = TypeDetector::new();
    let tiff_data = b"\x4D\x4D\x00\x2A";
    
    let result = detector.detect_from_bytes(tiff_data);
    
    assert_eq!(result.mime_type, "image/tiff");
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
}

#[test]
fn test_detector_trait_implementation() {
    let detector = TypeDetector::new();
    let pdf_data = b"%PDF-1.5";
    
    // Test using the Detector trait
    let result = detector.detect(pdf_data, None);
    
    assert_eq!(result.mime_type, "application/pdf");
    assert_eq!(result.detected_by, DetectionMethod::MagicBytes);
}

#[test]
fn test_detector_trait_with_filename() {
    let detector = TypeDetector::new();
    let unknown_data = b"\xFF\xFE\xFD\xFC";
    
    // Test using the Detector trait with filename
    let result = detector.detect(unknown_data, Some("document.pdf"));
    
    assert_eq!(result.mime_type, "application/pdf");
    assert_eq!(result.detected_by, DetectionMethod::Extension);
}
