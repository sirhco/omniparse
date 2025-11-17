//! Test to verify all parsers are registered correctly

use omniparse::{supported_mime_types, is_mime_supported};

#[test]
fn test_all_parsers_registered() {
    let supported = supported_mime_types();
    
    // Expected MIME types from all parsers
    let expected_types = vec![
        // Text parsers
        "text/plain",
        "application/json",
        "text/json",
        "text/csv",
        "text/tab-separated-values",
        "application/xml",
        "text/xml",
        // Document parsers
        "application/pdf",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/docx",
        "application/vnd.oasis.opendocument.text",
        "application/odt",
        // Image parsers
        "image/jpeg",
        "image/jpg",
        "image/png",
        "image/tiff",
        "image/tif",
        // Archive parsers
        "application/zip",
        "application/x-zip-compressed",
        "application/x-tar",
        "application/tar",
    ];
    
    println!("Supported MIME types ({}):", supported.len());
    for mime_type in &supported {
        println!("  - {}", mime_type);
    }
    
    // Verify all expected types are present
    for expected in &expected_types {
        assert!(
            supported.contains(&expected.to_string()),
            "MIME type '{}' is not in supported list",
            expected
        );
    }
    
    // Verify the count matches
    assert_eq!(
        supported.len(),
        expected_types.len(),
        "Expected {} MIME types, but found {}",
        expected_types.len(),
        supported.len()
    );
}

#[test]
fn test_is_mime_supported_for_all_formats() {
    // Test all implemented formats
    let formats = vec![
        // Text formats
        ("text/plain", true),
        ("application/json", true),
        ("text/json", true),
        ("text/csv", true),
        ("text/tab-separated-values", true),
        ("application/xml", true),
        ("text/xml", true),
        // Document formats
        ("application/pdf", true),
        ("application/vnd.openxmlformats-officedocument.wordprocessingml.document", true),
        ("application/docx", true),
        ("application/vnd.oasis.opendocument.text", true),
        ("application/odt", true),
        // Image formats
        ("image/jpeg", true),
        ("image/jpg", true),
        ("image/png", true),
        ("image/tiff", true),
        ("image/tif", true),
        // Archive formats
        ("application/zip", true),
        ("application/x-zip-compressed", true),
        ("application/x-tar", true),
        ("application/tar", true),
        // Unsupported formats
        ("application/x-custom", false),
        ("text/html", false),
        ("video/mp4", false),
    ];
    
    for (mime_type, expected) in formats {
        assert_eq!(
            is_mime_supported(mime_type),
            expected,
            "is_mime_supported('{}') returned {}, expected {}",
            mime_type,
            !expected,
            expected
        );
    }
}

#[test]
fn test_parser_count() {
    // We have 12 parsers total:
    // - 4 text parsers (PlainText, Json, Csv, Xml)
    // - 3 document parsers (Pdf, Docx, Odt)
    // - 3 image parsers (Jpeg, Png, Tiff)
    // - 2 archive parsers (Zip, Tar)
    
    let supported = supported_mime_types();
    
    // Total unique MIME types: 21
    assert_eq!(
        supported.len(),
        21,
        "Expected 21 MIME types, found {}",
        supported.len()
    );
}
