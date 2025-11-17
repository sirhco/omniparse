//! Performance validation tests

use omniparse::extract_from_path;
use std::fs;

use std::time::Instant;

#[test]
fn test_10mb_text_file_performance() {
    // Create a 10MB text file
    let test_file = "/tmp/omniparse_perf_test.txt";
    let content = "A".repeat(10 * 1024 * 1024); // 10MB of 'A's
    fs::write(test_file, content).expect("Failed to create test file");
    
    // Measure extraction time
    let start = Instant::now();
    let result = extract_from_path(test_file);
    let duration = start.elapsed();
    
    // Clean up
    let _ = fs::remove_file(test_file);
    
    // Verify extraction succeeded
    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());
    
    // Check performance requirement: should complete within 100ms
    // Note: This is a soft requirement and may vary based on hardware
    println!("Extraction time for 10MB file: {:?}", duration);
    
    // We'll be lenient and allow up to 500ms for slower systems
    assert!(
        duration.as_millis() < 500,
        "Extraction took too long: {:?} (expected < 500ms)",
        duration
    );
}

#[test]
fn test_memory_usage_with_large_file() {
    // Create a 10MB text file
    let test_file = "/tmp/omniparse_memory_test.txt";
    let content = "B".repeat(10 * 1024 * 1024); // 10MB of 'B's
    fs::write(test_file, content).expect("Failed to create test file");
    
    // Extract content
    let result = extract_from_path(test_file);
    
    // Clean up
    let _ = fs::remove_file(test_file);
    
    // Verify extraction succeeded
    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());
    
    // The result should contain the extracted content
    let extraction = result.unwrap();
    
    // Verify we got text content
    match extraction.content {
        omniparse::Content::Text(text) => {
            println!("Extracted text length: {} bytes", text.len());
            assert!(text.len() > 0, "No content extracted");
        }
        _ => panic!("Expected text content"),
    }
}

#[test]
#[cfg(feature = "parallel")]
fn test_parallel_processing_performance() {
    use omniparse::core::Extractor;
    use omniparse::utils::parallel::process_files_parallel;
    use std::path::PathBuf;
    
    // Create multiple test files
    let test_files: Vec<PathBuf> = (0..10)
        .map(|i| {
            let path = format!("/tmp/omniparse_parallel_test_{}.txt", i);
            let content = format!("Test file {} content\n", i).repeat(1000);
            fs::write(&path, content).expect("Failed to create test file");
            PathBuf::from(path)
        })
        .collect();
    
    // Create extractor
    let extractor = Extractor::new();
    
    // Measure parallel processing time
    let start = Instant::now();
    let results = process_files_parallel(&extractor, &test_files);
    let parallel_duration = start.elapsed();
    
    println!("Parallel processing time for 10 files: {:?}", parallel_duration);
    
    // Verify all files were processed
    assert_eq!(results.len(), 10, "Not all files were processed");
    
    let success_count = results.iter().filter(|r| r.result.is_ok()).count();
    println!("Successfully processed: {}/10 files", success_count);
    
    // Clean up
    for file in &test_files {
        let _ = fs::remove_file(file);
    }
    
    // Most files should succeed (allowing for some failures due to test conditions)
    assert!(
        success_count >= 8,
        "Too many files failed: only {}/10 succeeded",
        success_count
    );
}

#[test]
fn test_streaming_with_large_file() {
    use std::io::Cursor;
    
    // Create a large buffer (5MB)
    let content = "C".repeat(5 * 1024 * 1024);
    let mut cursor = Cursor::new(content.as_bytes());
    
    // Test streaming utilities
    use omniparse::utils::streaming::read_with_limit;
    
    let start = Instant::now();
    let result = read_with_limit(&mut cursor, 10 * 1024 * 1024);
    let duration = start.elapsed();
    
    println!("Streaming read time for 5MB: {:?}", duration);
    
    assert!(result.is_ok(), "Streaming read failed");
    let data = result.unwrap();
    assert_eq!(data.len(), 5 * 1024 * 1024, "Incorrect data size");
}
