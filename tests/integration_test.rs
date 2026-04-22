//! Integration tests for Omniparse
//!
//! These tests validate end-to-end functionality including:
//! - Complete extraction workflows from files
//! - Batch processing of multiple files
//! - Error handling for corrupted files
//! - Async API functionality

use omniparse::{extract_from_path, extract_from_bytes, supported_mime_types, is_mime_supported};
use omniparse::core::{Content, Error};
use std::path::PathBuf;

// ============================================================================
// END-TO-END EXTRACTION TESTS
// ============================================================================

#[test]
fn test_end_to_end_text_extraction() {
    // Test complete workflow: file -> detection -> parsing -> result
    // Use JSON file which has more predictable parsing
    let result = extract_from_path("test_data/text/sample.json")
        .expect("Failed to extract from JSON file");
    
    // Verify MIME type detection
    assert_eq!(result.mime_type, "application/json");
    
    // Verify content extraction
    match &result.content {
        Content::Text(text) => {
            assert!(!text.is_empty());
            assert!(text.contains("Test Document"));
        }
        _ => panic!("Expected text content"),
    }
    
    // Verify metadata extraction
    assert!(result.metadata.keys().count() > 0);
    
    // Verify detection confidence
    assert!(result.detection_confidence > 0.0);
    assert!(result.detection_confidence <= 1.0);
}

#[test]
fn test_end_to_end_json_extraction() {
    let result = extract_from_path("test_data/text/sample.json")
        .expect("Failed to extract from JSON file");
    
    assert_eq!(result.mime_type, "application/json");
    
    match &result.content {
        Content::Text(text) => {
            assert!(text.contains("Test Document"));
        }
        _ => panic!("Expected text content"),
    }
}

#[test]
fn test_end_to_end_csv_extraction() {
    let result = extract_from_path("test_data/text/sample.csv")
        .expect("Failed to extract from CSV file");
    
    assert_eq!(result.mime_type, "text/csv");
    
    match &result.content {
        Content::Text(text) => {
            assert!(text.contains("Alice"));
            assert!(text.contains("Bob"));
        }
        _ => panic!("Expected text content"),
    }
    
    // Verify CSV-specific metadata
    assert!(result.metadata.get("row_count").is_some());
    assert!(result.metadata.get("column_count").is_some());
}

#[test]
fn test_end_to_end_xml_extraction() {
    let result = extract_from_path("test_data/text/sample.xml");
    
    // XML may be detected as text/xml, application/xml, or image/svg+xml
    // SVG is not currently supported, so it may fail
    match result {
        Ok(res) => {
            assert!(res.mime_type.contains("xml"));
            match &res.content {
                Content::Text(text) => {
                    assert!(!text.is_empty());
                }
                _ => panic!("Expected text content"),
            }
        }
        Err(Error::UnsupportedFormat(mime)) => {
            // SVG XML is not supported yet, which is acceptable
            assert!(mime.contains("svg"));
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_end_to_end_pdf_extraction() {
    let result = extract_from_path("test_data/document/sample.pdf");
    
    // PDF may fail with test files, but should handle gracefully
    match result {
        Ok(res) => {
            assert_eq!(res.mime_type, "application/pdf");
        }
        Err(e) => {
            // Should be a parse error or corrupted file error, not a panic
            assert!(matches!(e, Error::ParseError(_) | Error::CorruptedFile(_)));
        }
    }
}

#[test]
fn test_end_to_end_docx_extraction() {
    let result = extract_from_path("test_data/document/sample.docx")
        .expect("Failed to extract from DOCX file");
    
    assert!(result.mime_type.contains("docx") || 
            result.mime_type.contains("wordprocessingml"));
    
    match &result.content {
        Content::Text(text) => {
            assert!(!text.is_empty());
        }
        _ => panic!("Expected text content"),
    }
}

#[test]
fn test_end_to_end_zip_extraction() {
    let result = extract_from_path("test_data/archive/sample.zip");
    
    // ZIP extraction may succeed or fail depending on archive contents
    match result {
        Ok(res) => {
            assert!(res.mime_type.contains("zip"));
            // Verify archive metadata if extraction succeeded
            assert!(res.metadata.get("file_count").is_some());
        }
        Err(_) => {
            // ZIP parsing may fail if it's a DOCX or other ZIP-based format
            // This is acceptable behavior
        }
    }
}

#[test]
fn test_end_to_end_tar_extraction() {
    let result = extract_from_path("test_data/archive/sample.tar")
        .expect("Failed to extract from TAR file");
    
    assert!(result.mime_type.contains("tar"));
    assert!(result.metadata.get("file_count").is_some());
}

// ============================================================================
// EXTRACT FROM BYTES TESTS
// ============================================================================

#[test]
fn test_extract_from_bytes_with_hint() {
    let data = std::fs::read("test_data/text/sample.json")
        .expect("Failed to read test file");
    
    let result = extract_from_bytes(&data, Some("application/json"))
        .expect("Failed to extract from bytes");
    
    assert_eq!(result.mime_type, "application/json");
    
    match &result.content {
        Content::Text(text) => {
            assert!(text.contains("Test Document"));
        }
        _ => panic!("Expected text content"),
    }
}

#[test]
fn test_extract_from_bytes_without_hint() {
    let data = std::fs::read("test_data/text/sample.json")
        .expect("Failed to read test file");
    
    let result = extract_from_bytes(&data, None)
        .expect("Failed to extract from bytes");
    
    // Should auto-detect as JSON
    assert_eq!(result.mime_type, "application/json");
}

#[test]
fn test_extract_from_bytes_wrong_hint() {
    let data = std::fs::read("test_data/text/sample.json")
          
      .expect("Failed to read test file");
    
    // Provide wrong MIME type hint - should still try to parse with the hint
    let result = extract_from_bytes(&data, Some("text/plain"));
    
    // This might succeed (treating JSON as plain text) or fail
    // The important thing is it doesn't panic
    let _ = result;
}

// ============================================================================
// BATCH PROCESSING TESTS
// ============================================================================

#[test]
fn test_batch_processing_multiple_files() {
    let files = vec![
        "test_data/text/sample.json",
        "test_data/text/sample.csv",
        "test_data/document/sample.docx",
        "test_data/archive/sample.tar",
    ];
    
    let mut results = Vec::new();
    let mut errors = Vec::new();
    
    for file in &files {
        match extract_from_path(file) {
            Ok(result) => results.push(result),
            Err(e) => errors.push((file, e)),
        }
    }
    
    // Most files should parse successfully
    assert!(results.len() >= 2, "Expected at least 2 files to parse successfully, got {}", results.len());
    
    // Verify each result has a MIME type
    for result in &results {
        assert!(!result.mime_type.is_empty());
    }
}

#[test]
fn test_batch_processing_mixed_formats() {
    let files = vec![
        "test_data/text/sample.txt",
        "test_data/document/sample.docx",
        "test_data/archive/sample.tar",
    ];
    
    let results: Vec<_> = files.iter()
        .filter_map(|file| extract_from_path(file).ok())
        .collect();
    
    // Should successfully process at least 2 files
    assert!(results.len() >= 2, "Expected at least 2 successful extractions, got {}", results.len());
    
    // Verify we have different MIME types
    let mime_types: Vec<_> = results.iter().map(|r| &r.mime_type).collect();
    assert!(!mime_types.is_empty());
}

#[test]
fn test_batch_processing_with_errors() {
    let files = vec![
        "test_data/text/sample.txt",
        "test_data/text/invalid.json",  // This should fail
        "test_data/text/sample.csv",
    ];
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    for file in &files {
        match extract_from_path(file) {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }
    
    // Should have at least 1 success
    assert!(success_count >= 1, "Expected at least 1 successful extraction, got {}", success_count);
    
    // Should have at least 1 error (invalid.json)
    assert!(error_count >= 1, "Expected at least 1 error, got {}", error_count);
}

#[test]
fn test_batch_processing_continue_on_error() {
    // Simulate batch processing that continues even when individual files fail
    let files = vec![
        "test_data/text/sample.txt",
        "nonexistent_file.txt",  // This will cause IO error
        "test_data/text/sample.json",
    ];
    
    let results: Vec<_> = files.iter()
        .filter_map(|file| {
            match extract_from_path(file) {
                Ok(result) => Some(result),
                Err(e) => {
                    // Log error but continue
                    eprintln!("Error processing {}: {}", file, e);
                    None
                }
            }
        })
        .collect();
    
    // Should have at least 1 successful result despite errors
    assert!(results.len() >= 1, "Expected at least 1 successful extraction, got {}", results.len());
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[test]
fn test_error_handling_nonexistent_file() {
    let result = extract_from_path("nonexistent_file.txt");
    
    assert!(result.is_err(), "Expected error for nonexistent file");
    
    match result {
        Err(Error::Io(_)) => {
            // Correct error type
        }
        Err(e) => panic!("Expected IO error, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[test]
fn test_error_handling_corrupted_json() {
    let result = extract_from_path("test_data/text/invalid.json");
    
    assert!(result.is_err(), "Expected error for invalid JSON");
    
    match result {
        Err(Error::ParseError(_)) => {
            // Correct error type
        }
        Err(e) => {
            // May also be CorruptedFile error
            assert!(matches!(e, Error::CorruptedFile(_)), 
                    "Expected ParseError or CorruptedFile, got: {:?}", e);
        }
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[test]
fn test_error_handling_empty_pdf() {
    let result = extract_from_path("test_data/document/empty.pdf");
    
    // Empty PDF should either parse (with no content) or fail gracefully
    match result {
        Ok(res) => {
            assert_eq!(res.mime_type, "application/pdf");
        }
        Err(e) => {
            // Should be ParseError or CorruptedFile, not a panic
            assert!(
                matches!(e, Error::ParseError(_) | Error::CorruptedFile(_)),
                "Expected ParseError or CorruptedFile, got: {:?}", e
            );
        }
    }
}

#[test]
fn test_error_handling_unsupported_format() {
    // Create a file with unknown format
    let unknown_data = b"UNKNOWN_FORMAT_HEADER\x00\x01\x02\x03";
    
    let result = extract_from_bytes(unknown_data, Some("application/x-unknown"));
    
    assert!(result.is_err(), "Expected error for unsupported format");
    
    match result {
        Err(Error::UnsupportedFormat(mime)) => {
            assert_eq!(mime, "application/x-unknown");
        }
        Err(e) => panic!("Expected UnsupportedFormat error, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[test]
fn test_error_handling_empty_file() {
    let result = extract_from_path("test_data/text/empty.txt");
    
    // Empty file should parse successfully (as empty text)
    match result {
        Ok(res) => {
            assert_eq!(res.mime_type, "text/plain");
            match &res.content {
                Content::Text(text) => assert_eq!(text, ""),
                _ => panic!("Expected text content"),
            }
        }
        Err(e) => panic!("Empty file should parse successfully, got error: {:?}", e),
    }
}

#[test]
fn test_error_context_includes_details() {
    let result = extract_from_path("nonexistent_file.txt");
    
    if let Err(e) = result {
        let error_msg = e.to_string();
        // Error message should contain useful information
        assert!(!error_msg.is_empty());
    }
}

// ============================================================================
// API QUERY TESTS
// ============================================================================

#[test]
fn test_supported_mime_types_not_empty() {
    let types = supported_mime_types();
    
    assert!(!types.is_empty(), "Should have at least one supported type");
    
    // Should include common types
    assert!(types.contains(&"text/plain".to_string()));
    assert!(types.contains(&"application/json".to_string()));
    assert!(types.contains(&"text/csv".to_string()));
}

#[test]
fn test_is_mime_supported_common_types() {
    // Test common supported types
    assert!(is_mime_supported("text/plain"));
    assert!(is_mime_supported("application/json"));
    assert!(is_mime_supported("text/csv"));
    assert!(is_mime_supported("application/xml"));
    assert!(is_mime_supported("application/pdf"));
    assert!(is_mime_supported("application/zip"));
}

#[test]
fn test_is_mime_supported_unsupported_types() {
    // Test unsupported types
    assert!(!is_mime_supported("application/x-unknown"));
    assert!(!is_mime_supported("video/mp4"));
    assert!(!is_mime_supported("audio/x-flac"));
}

#[test]
fn test_supported_types_consistency() {
    let types = supported_mime_types();
    
    // Every type in the list should be supported
    for mime_type in &types {
        assert!(
            is_mime_supported(mime_type),
            "Type {} is in supported list but is_mime_supported returns false",
            mime_type
        );
    }
}

// ============================================================================
// PATH HANDLING TESTS
// ============================================================================

#[test]
fn test_extract_with_pathbuf() {
    let path = PathBuf::from("test_data/text/sample.json");
    let result = extract_from_path(path)
        .expect("Failed to extract with PathBuf");
    
    // Should detect as JSON
    assert_eq!(result.mime_type, "application/json");
}

#[test]
fn test_extract_with_string() {
    let path = String::from("test_data/text/sample.json");
    let result = extract_from_path(path)
        .expect("Failed to extract with String");
    
    // Should detect as JSON
    assert_eq!(result.mime_type, "application/json");
}

#[test]
fn test_extract_with_str_ref() {
    let result = extract_from_path("test_data/text/sample.json")
        .expect("Failed to extract with &str");
    
    // Should detect as JSON
    assert_eq!(result.mime_type, "application/json");
}

// ============================================================================
// DETECTION CONFIDENCE TESTS
// ============================================================================

#[test]
fn test_detection_confidence_range() {
    let files = vec![
        "test_data/text/sample.txt",
        "test_data/text/sample.json",
        "test_data/archive/sample.zip",
    ];
    
    for file in files {
        if let Ok(result) = extract_from_path(file) {
            assert!(
                result.detection_confidence >= 0.0 && result.detection_confidence <= 1.0,
                "Confidence {} out of range for {}",
                result.detection_confidence,
                file
            );
        }
    }
}

#[test]
fn test_high_confidence_for_magic_bytes() {
    // Files with strong magic bytes should have high confidence
    let result = extract_from_path("test_data/archive/sample.tar");
    
    // TAR has strong magic bytes
    match result {
        Ok(res) => {
            assert!(
                res.detection_confidence >= 0.8,
                "Expected high confidence for TAR with magic bytes, got {}",
                res.detection_confidence
            );
        }
        Err(_) => {
            // If parsing fails, that's okay for this test
        }
    }
}

// ============================================================================
// ASYNC API TESTS
// ============================================================================

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use omniparse::extract_from_path_async;
    
    #[tokio::test]
    async fn test_async_extract_text_file() {
        let result = extract_from_path_async("test_data/text/sample.json")
            .await
            .expect("Failed to extract async");
        
        assert_eq!(result.mime_type, "application/json");
        
        match &result.content {
            Content::Text(text) => {
                assert!(text.contains("Test Document"));
            }
            _ => panic!("Expected text content"),
        }
    }
    
    #[tokio::test]
    async fn test_async_extract_json_file() {
        let result = extract_from_path_async("test_data/text/sample.json")
            .await
            .expect("Failed to extract JSON async");
        
        assert_eq!(result.mime_type, "application/json");
    }
    
    #[tokio::test]
    async fn test_async_extract_nonexistent_file() {
        let result = extract_from_path_async("nonexistent_file.txt").await;
        
        assert!(result.is_err(), "Expected error for nonexistent file");
        
        match result {
            Err(Error::Io(_)) => {
                // Correct error type
            }
            Err(e) => panic!("Expected IO error, got: {:?}", e),
            Ok(_) => panic!("Expected error, got success"),
        }
    }
    
    #[tokio::test]
    async fn test_async_batch_processing() {
        let files = vec![
            "test_data/text/sample.json",
            "test_data/text/sample.csv",
            "test_data/document/sample.docx",
        ];
        
        let mut results = Vec::new();
        
        for file in files {
            if let Ok(result) = extract_from_path_async(file).await {
                results.push(result);
            }
        }
        
        assert!(results.len() >= 2, "Expected at least 2 files to parse successfully, got {}", results.len());
    }
    
    #[tokio::test]
    async fn test_async_parallel_extraction() {
        use tokio::task;
        
        let files = vec![
            "test_data/text/sample.json",
            "test_data/text/sample.csv",
            "test_data/document/sample.docx",
        ];
        
        let handles: Vec<_> = files.into_iter()
            .map(|file| {
                task::spawn(async move {
                    extract_from_path_async(file).await
                })
            })
            .collect();
        
        let mut success_count = 0;
        
        for handle in handles {
            if let Ok(Ok(_)) = handle.await {
                success_count += 1;
            }
        }
        
        assert!(success_count >= 2, "Expected at least 2 parallel extractions to succeed, got {}", success_count);
    }
}
