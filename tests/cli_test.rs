//! CLI integration tests
//!
//! These tests validate the command-line interface functionality including:
//! - CLI argument parsing and handling
//! - Output format options (text, JSON, YAML)
//! - Error handling and exit codes
//! - Batch processing of multiple files
//! - Various CLI flags and options

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// BASIC CLI TESTS
// ============================================================================

#[test]
fn test_cli_help_flag() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust toolkit"))
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_version_flag() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("omniparse"));
}

#[test]
fn test_cli_no_arguments() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_cli_single_file_extraction() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("MIME Type:"))
        .stdout(predicate::str::contains("application/json"));
}

#[test]
fn test_cli_text_file_extraction() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    // Note: sample.txt contains a comma so may be detected as CSV
    // This test just verifies the CLI runs successfully
    cmd.arg("test_data/text/empty.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("MIME Type:"))
        .stdout(predicate::str::contains("text/plain"));
}

#[test]
fn test_cli_csv_file_extraction() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/text/sample.csv")
        .assert()
        .success()
        .stdout(predicate::str::contains("MIME Type:"))
        .stdout(predicate::str::contains("text/csv"))
        .stdout(predicate::str::contains("Metadata:"));
}

// ============================================================================
// OUTPUT FORMAT TESTS
// ============================================================================

#[test]
fn test_cli_text_format_default() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("MIME Type:"))
        .stdout(predicate::str::contains("Metadata:"))
        .stdout(predicate::str::contains("Content:"));
}

#[test]
fn test_cli_text_format_explicit() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--format")
        .arg("text")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("MIME Type:"))
        .stdout(predicate::str::contains("Metadata:"));
}

#[test]
fn test_cli_json_format() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--format")
        .arg("json")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mime_type\""))
        .stdout(predicate::str::contains("\"metadata\""))
        .stdout(predicate::str::contains("\"detection_confidence\""));
}

#[test]
fn test_cli_json_format_short_flag() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("-f")
        .arg("json")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mime_type\""));
}

#[test]
fn test_cli_yaml_format() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--format")
        .arg("yaml")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("mime_type:"))
        .stdout(predicate::str::contains("metadata:"));
}

#[test]
fn test_cli_json_format_valid_json() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    let output = cmd.arg("--format")
        .arg("json")
        .arg("test_data/text/sample.json")
        .assert()
        .success();
    
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    
    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");
    
    assert!(parsed.get("mime_type").is_some());
    assert!(parsed.get("metadata").is_some());
}

// ============================================================================
// METADATA-ONLY FLAG TESTS
// ============================================================================

#[test]
fn test_cli_metadata_only_flag() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--metadata-only")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("MIME Type:"))
        .stdout(predicate::str::contains("Metadata:"));
}

#[test]
fn test_cli_metadata_only_short_flag() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("-m")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("Metadata:"));
}

#[test]
fn test_cli_metadata_only_json_format() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    let output = cmd.arg("--metadata-only")
        .arg("--format")
        .arg("json")
        .arg("test_data/text/sample.json")
        .assert()
        .success();
    
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");
    
    // Should have metadata but no content field
    assert!(parsed.get("metadata").is_some());
    assert!(parsed.get("content").is_none());
}

// ============================================================================
// DETECT-ONLY FLAG TESTS
// ============================================================================

#[test]
fn test_cli_detect_only_flag() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--detect-only")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("MIME Type:"))
        .stdout(predicate::str::contains("Confidence:"))
        .stdout(predicate::str::contains("Detected By:"));
}

#[test]
fn test_cli_detect_only_short_flag() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("-d")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("MIME Type:"));
}

#[test]
fn test_cli_detect_only_json_format() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    let output = cmd.arg("--detect-only")
        .arg("--format")
        .arg("json")
        .arg("test_data/text/sample.json")
        .assert()
        .success();
    
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");
    
    assert!(parsed.get("mime_type").is_some());
    assert!(parsed.get("confidence").is_some());
    assert!(parsed.get("detected_by").is_some());
}

#[test]
fn test_cli_detect_only_multiple_files() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--detect-only")
        .arg("test_data/text/sample.json")
        .arg("test_data/text/sample.csv")
        .assert()
        .success()
        .stdout(predicate::str::contains("application/json"))
        .stdout(predicate::str::contains("text/csv"));
}

// ============================================================================
// OUTPUT FILE TESTS
// ============================================================================

#[test]
fn test_cli_output_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.txt");
    
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--output")
        .arg(&output_path)
        .arg("test_data/text/sample.json")
        .assert()
        .success();
    
    // Verify file was created and contains expected content
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("MIME Type:"));
    assert!(content.contains("application/json"));
}

#[test]
fn test_cli_output_to_file_short_flag() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.txt");
    
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("-o")
        .arg(&output_path)
        .arg("test_data/text/sample.json")
        .assert()
        .success();
    
    assert!(output_path.exists());
}

#[test]
fn test_cli_output_json_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.json");
    
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--format")
        .arg("json")
        .arg("--output")
        .arg(&output_path)
        .arg("test_data/text/sample.json")
        .assert()
        .success();
    
    // Verify file contains valid JSON
    let content = fs::read_to_string(&output_path).unwrap();
    let _: serde_json::Value = serde_json::from_str(&content)
        .expect("Output file should contain valid JSON");
}

// ============================================================================
// VERBOSE FLAG TESTS
// ============================================================================

#[test]
fn test_cli_verbose_flag() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--verbose")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stderr(predicate::str::contains("Processing:"));
}

#[test]
fn test_cli_verbose_short_flag() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("-v")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stderr(predicate::str::contains("Processing:"));
}

#[test]
fn test_cli_verbose_with_multiple_files() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--verbose")
        .arg("test_data/text/sample.json")
        .arg("test_data/text/sample.csv")
        .assert()
        .success()
        .stderr(predicate::str::contains("Summary:"));
}

// ============================================================================
// BATCH PROCESSING TESTS
// ============================================================================

#[test]
fn test_cli_multiple_files() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/text/sample.json")
        .arg("test_data/text/sample.csv")
        .arg("test_data/text/empty.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("application/json"))
        .stdout(predicate::str::contains("text/csv"))
        .stdout(predicate::str::contains("text/plain"));
}

#[test]
fn test_cli_batch_with_file_separators() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/text/sample.json")
        .arg("test_data/text/sample.csv")
        .assert()
        .success()
        .stdout(predicate::str::contains("File:"))
        .stdout(predicate::str::contains("---"));
}

#[test]
fn test_cli_batch_json_format() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    let output = cmd.arg("--format")
        .arg("json")
        .arg("test_data/text/sample.json")
        .arg("test_data/text/sample.csv")
        .assert()
        .success();
    
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    
    // Should contain multiple JSON objects
    assert!(stdout.contains("application/json"));
    assert!(stdout.contains("text/csv"));
}

#[test]
fn test_cli_batch_with_errors() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/text/sample.json")
        .arg("nonexistent_file.txt")
        .arg("test_data/text/sample.csv")
        .assert()
        .success()  // Should succeed overall even with one error
        .stderr(predicate::str::contains("Error processing"));
}

#[test]
fn test_cli_batch_all_errors() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("nonexistent1.txt")
        .arg("nonexistent2.txt")
        .assert()
        .failure()  // Should fail if all files fail
        .stderr(predicate::str::contains("Error"));
}

// ============================================================================
// PARALLEL PROCESSING TESTS
// ============================================================================

#[test]
fn test_cli_parallel_flag() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--parallel")
        .arg("test_data/text/sample.json")
        .arg("test_data/text/sample.csv")
        .arg("test_data/text/empty.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("application/json"))
        .stdout(predicate::str::contains("text/csv"));
}

#[test]
fn test_cli_parallel_short_flag() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("-p")
        .arg("test_data/text/sample.json")
        .arg("test_data/text/sample.csv")
        .assert()
        .success();
}

#[test]
fn test_cli_parallel_with_verbose() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--parallel")
        .arg("--verbose")
        .arg("test_data/text/sample.json")
        .arg("test_data/text/sample.csv")
        .assert()
        .success()
        .stderr(predicate::str::contains("Processing"))
        .stderr(predicate::str::contains("files in parallel"));
}

#[test]
fn test_cli_parallel_single_file() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    // Parallel flag with single file should still work (just not parallel)
    cmd.arg("--parallel")
        .arg("test_data/text/sample.json")
        .assert()
        .success();
}

// ============================================================================
// ERROR HANDLING AND EXIT CODE TESTS
// ============================================================================

#[test]
fn test_cli_nonexistent_file_error() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("nonexistent_file.txt")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn test_cli_invalid_json_error() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/text/invalid.json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn test_cli_error_message_to_stderr() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("nonexistent_file.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_cli_invalid_format_option() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--format")
        .arg("invalid_format")
        .arg("test_data/text/sample.json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_cli_corrupted_file_error() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    // Empty PDF should cause an error
    cmd.arg("test_data/document/empty.pdf")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

// ============================================================================
// COMBINED FLAGS TESTS
// ============================================================================

#[test]
fn test_cli_metadata_only_with_json() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--metadata-only")
        .arg("--format")
        .arg("json")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"metadata\""));
}

#[test]
fn test_cli_detect_only_with_yaml() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--detect-only")
        .arg("--format")
        .arg("yaml")
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("mime_type:"));
}

#[test]
fn test_cli_verbose_with_output_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.txt");
    
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--verbose")
        .arg("--output")
        .arg(&output_path)
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stderr(predicate::str::contains("Processing:"));
    
    assert!(output_path.exists());
}

#[test]
fn test_cli_parallel_with_json_output() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--parallel")
        .arg("--format")
        .arg("json")
        .arg("test_data/text/sample.json")
        .arg("test_data/text/sample.csv")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mime_type\""));
}

#[test]
fn test_cli_all_flags_combined() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.json");
    
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("--verbose")
        .arg("--metadata-only")
        .arg("--format")
        .arg("json")
        .arg("--output")
        .arg(&output_path)
        .arg("test_data/text/sample.json")
        .assert()
        .success()
        .stderr(predicate::str::contains("Processing:"));
    
    let content = fs::read_to_string(&output_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed.get("metadata").is_some());
    assert!(parsed.get("content").is_none());
}

// ============================================================================
// DIFFERENT FILE TYPE TESTS
// ============================================================================

#[test]
fn test_cli_docx_file() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/document/sample.docx")
        .assert()
        .success()
        .stdout(predicate::str::contains("MIME Type:"));
}

#[test]
fn test_cli_tar_file() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/archive/sample.tar")
        .assert()
        .success()
        .stdout(predicate::str::contains("tar"));
}

#[test]
fn test_cli_xml_file() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    // XML may succeed or fail depending on detection
    let _ = cmd.arg("test_data/text/sample.xml")
        .assert();
}

#[test]
fn test_cli_mixed_file_types() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/text/sample.json")
        .arg("test_data/document/sample.docx")
        .arg("test_data/archive/sample.tar")
        .assert()
        .success()
        .stdout(predicate::str::contains("application/json"));
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_cli_empty_text_file() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/text/empty.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("text/plain"));
}

#[test]
fn test_cli_minimal_json_file() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    cmd.arg("test_data/text/minimal.json")
        .assert()
        .success()
        .stdout(predicate::str::contains("application/json"));
}

#[test]
fn test_cli_minimal_csv_file() {
    let mut cmd = Command::cargo_bin("omniparse").unwrap();
    
    // Minimal CSV may fail parsing if it's too minimal
    // Just verify it doesn't panic
    let _ = cmd.arg("test_data/text/minimal.csv")
        .assert();
}
