//! Test to verify all parsers are registered correctly.
//!
//! Assertions check for required MIME types rather than an exact count — new
//! parsers are added regularly and the count is a moving target.

use omniparse::{is_mime_supported, supported_mime_types};

const REQUIRED_MIME_TYPES: &[&str] = &[
    // Text
    "text/plain",
    "application/json",
    "text/json",
    "text/csv",
    "text/tab-separated-values",
    "application/xml",
    "text/xml",
    "text/html",
    "text/css",
    "application/rtf",
    // Documents
    "application/pdf",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/docx",
    "application/vnd.oasis.opendocument.text",
    "application/odt",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/vnd.oasis.opendocument.spreadsheet",
    "application/vnd.oasis.opendocument.presentation",
    "application/vnd.ms-excel",
    "application/msword",
    "application/vnd.ms-powerpoint",
    // Images
    "image/jpeg",
    "image/jpg",
    "image/png",
    "image/tiff",
    "image/tif",
    // Archives
    "application/zip",
    "application/x-zip-compressed",
    "application/x-tar",
    "application/tar",
];

#[test]
fn test_all_required_parsers_registered() {
    let supported = supported_mime_types();
    for required in REQUIRED_MIME_TYPES {
        assert!(
            supported.contains(&required.to_string()),
            "required MIME type '{}' missing from registry",
            required
        );
    }
}

#[test]
fn test_is_mime_supported_for_known_formats() {
    for mime in REQUIRED_MIME_TYPES {
        assert!(is_mime_supported(mime), "expected {} to be supported", mime);
    }
    for mime in ["application/x-custom", "video/mp4", "audio/flac"] {
        assert!(
            !is_mime_supported(mime),
            "did not expect {} to be supported",
            mime
        );
    }
}

#[test]
fn test_registry_is_non_empty() {
    assert!(supported_mime_types().len() >= REQUIRED_MIME_TYPES.len());
}
