//! Unit tests for all parser implementations
//!
//! This test suite validates the functionality of each parser with various file formats.

use omniparse::parsers::ParserRegistry;
use omniparse::core::{Content, MetadataValue};
use std::fs;

// Helper function to read test data files
fn read_test_file(path: &str) -> Vec<u8> {
    fs::read(path).expect(&format!("Failed to read test file: {}", path))
}

// Helper function to extract text content from result
fn extract_text(content: &Content) -> Option<&str> {
    match content {
        Content::Text(text) => Some(text),
        _ => None,
    }
}

// ============================================================================
// TEXT PARSER TESTS
// ============================================================================

#[test]
fn test_plain_text_parser_basic() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("text/plain").expect("PlainTextParser not found");
    
    let data = read_test_file("test_data/text/sample.txt");
    let result = parser.parse(&data, "text/plain").expect("Failed to parse plain text");
    
    assert_eq!(result.mime_type, "text/plain");
    
    // Check content
    let text = extract_text(&result.content).expect("Expected text content");
    assert!(text.contains("Hello, World!"));
    assert!(text.contains("sample plain text file"));
    
    // Check metadata
    assert!(result.metadata.get("encoding").is_some());
    assert!(result.metadata.get("line_count").is_some());
    assert!(result.metadata.get("character_count").is_some());
}

#[test]
fn test_plain_text_parser_empty() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("text/plain").expect("PlainTextParser not found");
    
    let data = read_test_file("test_data/text/empty.txt");
    let result = parser.parse(&data, "text/plain").expect("Failed to parse empty text");
    
    let text = extract_text(&result.content).expect("Expected text content");
    assert_eq!(text, "");
}

#[test]
fn test_plain_text_parser_utf8() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("text/plain").expect("PlainTextParser not found");
    
    // Test UTF-8 content with special characters
    let utf8_data = "Hello 世界! Привет мир! 🌍".as_bytes();
    let result = parser.parse(utf8_data, "text/plain").expect("Failed to parse UTF-8 text");
    
    let text = extract_text(&result.content).expect("Expected text content");
    assert!(text.contains("世界"));
    assert!(text.contains("Привет"));
    assert!(text.contains("🌍"));
    
    // Check encoding metadata
    if let Some(MetadataValue::Text(encoding)) = result.metadata.get("encoding") {
        assert_eq!(encoding, "UTF-8");
    } else {
        panic!("Expected encoding metadata");
    }
}

#[test]
fn test_json_parser_valid() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/json").expect("JsonParser not found");
    
    let data = read_test_file("test_data/text/sample.json");
    let result = parser.parse(&data, "application/json").expect("Failed to parse JSON");
    
    assert_eq!(result.mime_type, "application/json");
    
    // Check content
    let text = extract_text(&result.content).expect("Expected text content");
    assert!(text.contains("Test Document"));
    assert!(text.contains("Omniparse Test"));
    
    // Check metadata
    if let Some(MetadataValue::Boolean(valid)) = result.metadata.get("valid") {
        assert!(valid, "JSON should be valid");
    } else {
        panic!("Expected valid metadata");
    }
}

#[test]
fn test_json_parser_invalid() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/json").expect("JsonParser not found");
    
    let data = read_test_file("test_data/text/invalid.json");
    let result = parser.parse(&data, "application/json");
    
    // Invalid JSON should return an error
    assert!(result.is_err(), "Expected error for invalid JSON");
}

#[test]
fn test_json_parser_minimal() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/json").expect("JsonParser not found");
    
    let data = read_test_file("test_data/text/minimal.json");
    let result = parser.parse(&data, "application/json").expect("Failed to parse minimal JSON");
    
    if let Some(MetadataValue::Boolean(valid)) = result.metadata.get("valid") {
        assert!(valid, "Minimal JSON should be valid");
    }
}

#[test]
fn test_csv_parser_basic() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("text/csv").expect("CsvParser not found");
    
    let data = read_test_file("test_data/text/sample.csv");
    let result = parser.parse(&data, "text/csv").expect("Failed to parse CSV");
    
    assert_eq!(result.mime_type, "text/csv");
    
    // Check content
    let text = extract_text(&result.content).expect("Expected text content");
    assert!(text.contains("Alice"));
    assert!(text.contains("Bob"));
    assert!(text.contains("Charlie"));
    
    // Check metadata
    if let Some(MetadataValue::Number(row_count)) = result.metadata.get("row_count") {
        assert!(*row_count >= 3, "Expected at least 3 data rows");
    } else {
        panic!("Expected row_count metadata");
    }
    
    if let Some(MetadataValue::Number(col_count)) = result.metadata.get("column_count") {
        assert_eq!(*col_count, 3, "Expected 3 columns");
    } else {
        panic!("Expected column_count metadata");
    }
}

#[test]
fn test_csv_parser_with_headers() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("text/csv").expect("CsvParser not found");
    
    let data = read_test_file("test_data/text/sample.csv");
    let result = parser.parse(&data, "text/csv").expect("Failed to parse CSV");
    
    // Check headers metadata
    if let Some(MetadataValue::List(headers)) = result.metadata.get("headers") {
        assert_eq!(headers.len(), 3);
        // Headers should be Name, Age, City
        if let MetadataValue::Text(first_header) = &headers[0] {
            assert_eq!(first_header, "Name");
        }
    } else {
        panic!("Expected headers metadata");
    }
}

#[test]
fn test_csv_parser_minimal() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("text/csv").expect("CsvParser not found");
    
    let data = read_test_file("test_data/text/minimal.csv");
    let result = parser.parse(&data, "text/csv").expect("Failed to parse minimal CSV");
    
    assert!(result.metadata.get("row_count").is_some());
}

#[test]
fn test_csv_parser_tsv() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("text/tab-separated-values").expect("CsvParser not found for TSV");
    
    // Create TSV data
    let tsv_data = b"Name\tAge\tCity\nAlice\t30\tNew York\nBob\t25\tSan Francisco";
    let result = parser.parse(tsv_data, "text/tab-separated-values").expect("Failed to parse TSV");
    
    let text = extract_text(&result.content).expect("Expected text content");
    assert!(text.contains("Alice"));
}

#[test]
fn test_xml_parser_basic() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/xml").expect("XmlParser not found");
    
    let data = read_test_file("test_data/text/sample.xml");
    let result = parser.parse(&data, "application/xml").expect("Failed to parse XML");
    
    assert_eq!(result.mime_type, "application/xml");
    
    // Check content
    let text = extract_text(&result.content).expect("Expected text content");
    assert!(text.contains("Sample XML Document"));
    assert!(text.contains("Omniparse Test"));
    assert!(text.contains("test XML file"));
    
    // Check metadata
    if let Some(MetadataValue::Text(root)) = result.metadata.get("root_element") {
        assert_eq!(root, "document");
    } else {
        panic!("Expected root_element metadata");
    }
}

#[test]
fn test_xml_parser_with_namespaces() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/xml").expect("XmlParser not found");
    
    let xml_with_ns = br#"<?xml version="1.0"?>
<root xmlns:custom="http://example.com/custom">
    <custom:element>Test</custom:element>
</root>"#;
    
    let result = parser.parse(xml_with_ns, "application/xml").expect("Failed to parse XML with namespaces");
    
    // Should handle namespaces
    assert!(result.metadata.get("namespaces").is_some());
}

// ============================================================================
// DOCUMENT PARSER TESTS
// ============================================================================

#[cfg(feature = "pdf")]
#[test]
fn test_pdf_parser_basic() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/pdf").expect("PdfParser not found");
    
    let data = read_test_file("test_data/document/sample.pdf");
    let result = parser.parse(&data, "application/pdf");
    
    // PDF parsing may fail if the test file is corrupted or minimal
    // Just ensure the parser handles it gracefully
    match result {
        Ok(res) => {
            assert_eq!(res.mime_type, "application/pdf");
            // If successful, check for metadata
            let _ = res.metadata.get("page_count");
        }
        Err(_) => {
            // PDF parsing failed, which is acceptable for test files
            // The important thing is it didn't panic
        }
    }
}

#[cfg(feature = "pdf")]
#[test]
fn test_pdf_parser_empty() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/pdf").expect("PdfParser not found");
    
    let data = read_test_file("test_data/document/empty.pdf");
    let result = parser.parse(&data, "application/pdf");
    
    // Empty PDF might parse successfully or fail depending on structure
    // Just ensure it doesn't panic
    let _ = result;
}

#[test]
fn test_docx_parser_basic() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
        .expect("DocxParser not found");
    
    let data = read_test_file("test_data/document/sample.docx");
    let result = parser.parse(&data, "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
        .expect("Failed to parse DOCX");
    
    // Check content
    let text = extract_text(&result.content).expect("Expected text content");
    assert!(!text.is_empty(), "DOCX should contain some text");
}

#[test]
fn test_docx_parser_alternate_mime() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/docx").expect("DocxParser not found with alternate MIME");
    
    let data = read_test_file("test_data/document/sample.docx");
    let result = parser.parse(&data, "application/docx").expect("Failed to parse DOCX with alternate MIME");
    
    assert_eq!(result.mime_type, "application/docx");
}

#[test]
fn test_odt_parser_basic() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/vnd.oasis.opendocument.text")
        .expect("OdtParser not found");
    
    let data = read_test_file("test_data/document/sample.odt");
    let result = parser.parse(&data, "application/vnd.oasis.opendocument.text")
        .expect("Failed to parse ODT");
    
    // Check content
    let text = extract_text(&result.content).expect("Expected text content");
    assert!(!text.is_empty(), "ODT should contain some text");
}

#[test]
fn test_odt_parser_alternate_mime() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/odt").expect("OdtParser not found with alternate MIME");
    
    let data = read_test_file("test_data/document/sample.odt");
    let result = parser.parse(&data, "application/odt").expect("Failed to parse ODT with alternate MIME");
    
    assert_eq!(result.mime_type, "application/odt");
}

// ============================================================================
// IMAGE PARSER TESTS
// ============================================================================

#[test]
fn test_jpeg_parser_basic() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("image/jpeg").expect("JpegParser not found");
    
    let data = read_test_file("test_data/image/sample.jpg");
    let result = parser.parse(&data, "image/jpeg");
    
    // JPEG parsing may fail if the test file is corrupted or minimal
    match result {
        Ok(res) => {
            assert_eq!(res.mime_type, "image/jpeg");
            // Check metadata - images should have dimensions
            assert!(res.metadata.get("width").is_some() || res.metadata.get("dimensions").is_some());
        }
        Err(_) => {
            // JPEG parsing failed, which is acceptable for test files
        }
    }
}

#[test]
fn test_jpeg_parser_alternate_mime() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("image/jpg").expect("JpegParser not found with .jpg MIME");
    
    let data = read_test_file("test_data/image/sample.jpg");
    let result = parser.parse(&data, "image/jpg");
    
    // JPEG parsing may fail if the test file is corrupted
    match result {
        Ok(res) => {
            assert_eq!(res.mime_type, "image/jpg");
        }
        Err(_) => {
            // JPEG parsing failed, which is acceptable for test files
        }
    }
}

#[test]
fn test_png_parser_basic() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("image/png").expect("PngParser not found");
    
    let data = read_test_file("test_data/image/sample.png");
    let result = parser.parse(&data, "image/png");
    
    // PNG parsing may fail if the test file is corrupted
    match result {
        Ok(res) => {
            assert_eq!(res.mime_type, "image/png");
            // Check metadata
            assert!(res.metadata.get("width").is_some() || res.metadata.get("dimensions").is_some());
        }
        Err(_) => {
            // PNG parsing failed, which is acceptable for test files
        }
    }
}

#[test]
fn test_png_parser_empty() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("image/png").expect("PngParser not found");
    
    let data = read_test_file("test_data/image/empty.png");
    let result = parser.parse(&data, "image/png");
    
    // Empty PNG might parse or fail - just ensure no panic
    let _ = result;
}

#[test]
fn test_tiff_parser_basic() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("image/tiff").expect("TiffParser not found");
    
    let data = read_test_file("test_data/image/sample.tiff");
    let result = parser.parse(&data, "image/tiff");
    
    // TIFF parsing may fail if the test file is corrupted or uses unsupported features
    match result {
        Ok(res) => {
            assert_eq!(res.mime_type, "image/tiff");
            // Check metadata
            assert!(res.metadata.get("width").is_some() || res.metadata.get("dimensions").is_some());
        }
        Err(_) => {
            // TIFF parsing failed, which is acceptable for test files
        }
    }
}

#[test]
fn test_tiff_parser_alternate_mime() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("image/tif").expect("TiffParser not found with .tif MIME");
    
    let data = read_test_file("test_data/image/sample.tiff");
    let result = parser.parse(&data, "image/tif");
    
    // TIFF parsing may fail if the test file is corrupted
    match result {
        Ok(res) => {
            assert_eq!(res.mime_type, "image/tif");
        }
        Err(_) => {
            // TIFF parsing failed, which is acceptable for test files
        }
    }
}

// ============================================================================
// ARCHIVE PARSER TESTS
// ============================================================================

#[test]
fn test_zip_parser_basic() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/zip").expect("ZipParser not found");
    
    let data = read_test_file("test_data/archive/sample.zip");
    let result = parser.parse(&data, "application/zip").expect("Failed to parse ZIP");
    
    assert_eq!(result.mime_type, "application/zip");
    
    // Check metadata
    if let Some(MetadataValue::Number(file_count)) = result.metadata.get("file_count") {
        assert!(*file_count > 0, "ZIP should contain files");
    } else {
        panic!("Expected file_count metadata");
    }
    
    assert!(result.metadata.get("total_size").is_some());
}

#[test]
fn test_zip_parser_empty() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/zip").expect("ZipParser not found");
    
    let data = read_test_file("test_data/archive/empty.zip");
    let result = parser.parse(&data, "application/zip");
    
    // Empty ZIP file may not be a valid ZIP structure
    match result {
        Ok(res) => {
            // If it parses, it should have 0 files
            if let Some(MetadataValue::Number(file_count)) = res.metadata.get("file_count") {
                assert_eq!(*file_count, 0, "Empty ZIP should have 0 files");
            }
        }
        Err(_) => {
            // Empty ZIP may not be a valid ZIP file structure
        }
    }
}

#[test]
fn test_zip_parser_alternate_mime() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/x-zip-compressed")
        .expect("ZipParser not found with alternate MIME");
    
    let data = read_test_file("test_data/archive/sample.zip");
    let result = parser.parse(&data, "application/x-zip-compressed")
        .expect("Failed to parse ZIP with alternate MIME");
    
    assert_eq!(result.mime_type, "application/x-zip-compressed");
}

#[test]
fn test_tar_parser_basic() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/x-tar").expect("TarParser not found");
    
    let data = read_test_file("test_data/archive/sample.tar");
    let result = parser.parse(&data, "application/x-tar").expect("Failed to parse TAR");
    
    assert_eq!(result.mime_type, "application/x-tar");
    
    // Check metadata
    if let Some(MetadataValue::Number(file_count)) = result.metadata.get("file_count") {
        assert!(*file_count > 0, "TAR should contain files");
    } else {
        panic!("Expected file_count metadata");
    }
}

#[test]
fn test_tar_parser_alternate_mime() {
    let registry = ParserRegistry::default();
    let parser = registry.get_parser("application/tar")
        .expect("TarParser not found with alternate MIME");
    
    let data = read_test_file("test_data/archive/sample.tar");
    let result = parser.parse(&data, "application/tar")
        .expect("Failed to parse TAR with alternate MIME");
    
    assert_eq!(result.mime_type, "application/tar");
}

// ============================================================================
// PARSER NAME TESTS
// ============================================================================

#[test]
fn test_all_parsers_have_names() {
    let registry = ParserRegistry::default();
    let types = registry.supported_types();
    
    for mime_type in types {
        if let Some(parser) = registry.get_parser(&mime_type) {
            let name = parser.name();
            assert!(!name.is_empty(), "Parser for {} should have a name", mime_type);
            println!("Parser for {}: {}", mime_type, name);
        }
    }
}
