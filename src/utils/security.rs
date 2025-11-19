//! Security utilities for safe file parsing
//!
//! This module provides security checks to protect against various file-based attacks
//! including ZIP bombs, XML bombs, and excessively large files.

use crate::core::{Error, Result};
use std::io::Cursor;
use zip::ZipArchive;

/// Maximum allowed decompression ratio for ZIP-based formats
/// If uncompressed size / compressed size exceeds this, it's likely a ZIP bomb
const MAX_DECOMPRESSION_RATIO: f64 = 100.0;

/// Maximum allowed uncompressed size for ZIP-based formats (500 MB)
const MAX_UNCOMPRESSED_SIZE: u64 = 500 * 1024 * 1024;

/// Maximum allowed file size for individual formats (in bytes)
pub struct FileSizeLimits;

impl FileSizeLimits {
    /// Maximum size for HTML files (10 MB)
    pub const HTML: usize = 10 * 1024 * 1024;
    
    /// Maximum size for CSS files (5 MB)
    pub const CSS: usize = 5 * 1024 * 1024;
    
    /// Maximum size for RTF files (20 MB)
    pub const RTF: usize = 20 * 1024 * 1024;
    
    /// Maximum size for XLSX files (100 MB)
    pub const XLSX: usize = 100 * 1024 * 1024;
    
    /// Maximum size for PPTX files (100 MB)
    pub const PPTX: usize = 100 * 1024 * 1024;
    
    /// Maximum size for ODS files (100 MB)
    pub const ODS: usize = 100 * 1024 * 1024;
    
    /// Maximum size for ODP files (100 MB)
    pub const ODP: usize = 100 * 1024 * 1024;
    
    /// Maximum size for XLS files (50 MB)
    pub const XLS: usize = 50 * 1024 * 1024;
    
    /// Maximum size for DOC files (50 MB)
    pub const DOC: usize = 50 * 1024 * 1024;
    
    /// Maximum size for PPT files (50 MB)
    pub const PPT: usize = 50 * 1024 * 1024;
}

/// Validate file size against a maximum limit
///
/// # Arguments
/// * `data` - The file data to check
/// * `max_size` - Maximum allowed size in bytes
/// * `format_name` - Name of the format for error messages
///
/// # Returns
/// * `Ok(())` if the file size is within limits
/// * `Err(Error::ParseError)` if the file exceeds the limit
pub fn validate_file_size(data: &[u8], max_size: usize, format_name: &str) -> Result<()> {
    if data.len() > max_size {
        return Err(Error::ParseError(format!(
            "{} file size ({} bytes) exceeds maximum allowed size ({} bytes)",
            format_name,
            data.len(),
            max_size
        )));
    }
    Ok(())
}

/// Check for ZIP bomb attacks in ZIP-based formats
///
/// This function validates that:
/// 1. The total uncompressed size is reasonable
/// 2. The decompression ratio is not suspiciously high
///
/// # Arguments
/// * `archive` - The ZIP archive to check
///
/// # Returns
/// * `Ok(())` if the archive appears safe
/// * `Err(Error::ParseError)` if a ZIP bomb is detected
pub fn check_zip_bomb(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<()> {
    let mut total_uncompressed: u64 = 0;
    let mut total_compressed: u64 = 0;
    
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            total_uncompressed += file.size();
            total_compressed += file.compressed_size();
        }
    }
    
    // Check if total uncompressed size exceeds limit
    if total_uncompressed > MAX_UNCOMPRESSED_SIZE {
        return Err(Error::ParseError(format!(
            "Potential ZIP bomb detected: uncompressed size ({} bytes) exceeds maximum allowed ({} bytes)",
            total_uncompressed,
            MAX_UNCOMPRESSED_SIZE
        )));
    }
    
    // Check decompression ratio (avoid division by zero)
    if total_compressed > 0 {
        let ratio = total_uncompressed as f64 / total_compressed as f64;
        if ratio > MAX_DECOMPRESSION_RATIO {
            return Err(Error::ParseError(format!(
                "Potential ZIP bomb detected: decompression ratio ({:.2}) exceeds maximum allowed ({:.2})",
                ratio,
                MAX_DECOMPRESSION_RATIO
            )));
        }
    }
    
    Ok(())
}

/// Maximum allowed XML entity expansion depth
const MAX_ENTITY_DEPTH: usize = 10;

/// Maximum allowed XML entity expansion count
const MAX_ENTITY_COUNT: usize = 10000;

/// Check for XML bomb attacks (billion laughs attack)
///
/// This function performs basic validation of XML content to detect
/// potential entity expansion attacks.
///
/// # Arguments
/// * `xml_content` - The XML content to check
///
/// # Returns
/// * `Ok(())` if the XML appears safe
/// * `Err(Error::ParseError)` if an XML bomb is detected
pub fn check_xml_bomb(xml_content: &str) -> Result<()> {
    // Count entity declarations
    let entity_count = xml_content.matches("<!ENTITY").count();
    
    if entity_count > MAX_ENTITY_COUNT {
        return Err(Error::ParseError(format!(
            "Potential XML bomb detected: entity count ({}) exceeds maximum allowed ({})",
            entity_count,
            MAX_ENTITY_COUNT
        )));
    }
    
    // Check for nested entity references (simplified check)
    // Look for patterns like &entity1; inside entity definitions
    if entity_count > 0 {
        let mut depth = 0;
        let lines: Vec<&str> = xml_content.lines().collect();
        
        for line in lines {
            if line.contains("<!ENTITY") {
                // Count entity references in this entity definition
                let ref_count = line.matches('&').count();
                if ref_count > MAX_ENTITY_DEPTH {
                    return Err(Error::ParseError(format!(
                        "Potential XML bomb detected: entity reference depth ({}) exceeds maximum allowed ({})",
                        ref_count,
                        MAX_ENTITY_DEPTH
                    )));
                }
                depth = depth.max(ref_count);
            }
        }
    }
    
    Ok(())
}

/// Validate ZIP-based file structure before full parsing
///
/// This performs basic structural validation to ensure the file
/// is a valid ZIP archive and contains expected files.
///
/// # Arguments
/// * `data` - The file data to validate
/// * `expected_files` - Optional list of files that should exist in the archive
///
/// # Returns
/// * `Ok(())` if the structure is valid
/// * `Err(Error::CorruptedFile)` if the structure is invalid
pub fn validate_zip_structure(data: &[u8], expected_files: Option<&[&str]>) -> Result<()> {
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor).map_err(|e| {
        Error::CorruptedFile(format!("Invalid ZIP structure: {}", e))
    })?;
    
    // Check for ZIP bomb before proceeding
    check_zip_bomb(&mut archive)?;
    
    // If expected files are specified, verify they exist
    if let Some(files) = expected_files {
        for expected_file in files {
            if archive.by_name(expected_file).is_err() {
                return Err(Error::CorruptedFile(format!(
                    "Missing expected file in archive: {}",
                    expected_file
                )));
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_file_size_within_limit() {
        let data = vec![0u8; 1000];
        let result = validate_file_size(&data, 2000, "Test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_size_exceeds_limit() {
        let data = vec![0u8; 3000];
        let result = validate_file_size(&data, 2000, "Test");
        assert!(result.is_err());
        if let Err(Error::ParseError(msg)) = result {
            assert!(msg.contains("exceeds maximum allowed size"));
        }
    }

    #[test]
    fn test_check_xml_bomb_safe_xml() {
        let xml = r#"<?xml version="1.0"?>
<root>
    <element>Content</element>
</root>"#;
        let result = check_xml_bomb(xml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_xml_bomb_excessive_entities() {
        let mut xml = String::from("<?xml version=\"1.0\"?>\n<!DOCTYPE root [\n");
        // Create excessive entity declarations
        for i in 0..11000 {
            xml.push_str(&format!("<!ENTITY entity{} \"value\">\n", i));
        }
        xml.push_str("]>\n<root></root>");
        
        let result = check_xml_bomb(&xml);
        assert!(result.is_err());
        if let Err(Error::ParseError(msg)) = result {
            assert!(msg.contains("entity count"));
        }
    }

    #[test]
    fn test_check_xml_bomb_nested_entities() {
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE root [
<!ENTITY a "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;">
]>
<root></root>"#;
        let result = check_xml_bomb(xml);
        assert!(result.is_err());
        if let Err(Error::ParseError(msg)) = result {
            assert!(msg.contains("entity reference depth"));
        }
    }
}
