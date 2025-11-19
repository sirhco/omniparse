//! PPT (Legacy Microsoft PowerPoint) parser implementation
//!
//! This parser provides basic support for legacy Microsoft PowerPoint .ppt files (OLE2 format).
//! Note: Full PPT parsing is complex due to the proprietary binary format. This implementation
//! provides basic text extraction and metadata where possible.

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::utils::security::{validate_file_size, FileSizeLimits};

/// Parser for Microsoft PowerPoint PPT files (legacy OLE2 format)
///
/// This parser provides basic support for extracting text and metadata from
/// legacy .ppt files. Due to the complexity of the OLE2/PowerPoint binary format,
/// extraction capabilities may be limited compared to modern formats.
pub struct PptParser;

impl Parser for PptParser {
    fn name(&self) -> &str {
        "PptParser"
    }

    fn supported_types(&self) -> &[&str] {
        &[
            "application/vnd.ms-powerpoint",
            "application/ppt",
        ]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Validate file size
        validate_file_size(data, FileSizeLimits::PPT, "PPT")?;
        
        // Validate OLE2 header
        if !is_ole2_file(data) {
            return Err(Error::ParseError(
                "Invalid PPT file: missing OLE2 header".to_string(),
            ));
        }

        // Extract text content
        let content_text = extract_text(data)?;

        // Extract metadata
        let metadata = extract_metadata(data)?;

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(content_text),
            metadata,
            detection_confidence: 0.90,
        })
    }
}

/// Check if the data starts with OLE2 magic bytes
fn is_ole2_file(data: &[u8]) -> bool {
    data.len() >= 8 && data[0..8] == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
}

/// Extract text content from PPT file
///
/// This is a basic implementation that attempts to extract readable text
/// from the OLE2 structure. Full PPT parsing would require a complete
/// implementation of the PowerPoint Binary File Format specification.
fn extract_text(data: &[u8]) -> Result<String> {
    // For now, we'll implement a basic text extraction strategy
    // A full implementation would parse the OLE2 structure and extract
    // the PowerPoint Document stream, then parse the various record types
    // to reconstruct slide text.
    
    // Basic approach: scan for readable text sequences
    let text = extract_readable_text(data);
    
    if text.trim().is_empty() {
        return Err(Error::ParseError(
            "Unable to extract text from PPT file. Full PPT parsing requires complex OLE2 structure analysis.".to_string(),
        ));
    }
    
    Ok(text)
}

/// Extract readable ASCII/UTF-8 text from binary data
///
/// This is a fallback method that scans for readable text sequences.
/// It's not perfect but can extract some content from PPT files.
fn extract_readable_text(data: &[u8]) -> String {
    let mut text = String::new();
    let mut current_word = Vec::new();
    let mut slide_number = 0;
    let mut last_was_slide_marker = false;
    
    for i in 0..data.len() {
        let byte = data[i];
        
        // Check if byte is printable ASCII or common whitespace
        if (byte >= 32 && byte <= 126) || byte == b'\n' || byte == b'\r' || byte == b'\t' {
            current_word.push(byte);
        } else if byte == 0 && !current_word.is_empty() {
            // Null byte might indicate end of text sequence
            if current_word.len() >= 3 {
                // Only keep sequences of 3+ characters
                if let Ok(s) = String::from_utf8(current_word.clone()) {
                    // Filter out sequences that look like binary data
                    if is_likely_text(&s) {
                        // Check if this might be a new slide (heuristic)
                        if should_add_slide_separator(&s, &text, last_was_slide_marker) {
                            slide_number += 1;
                            if slide_number > 1 {
                                text.push_str("\n\n--- Slide ");
                                text.push_str(&slide_number.to_string());
                                text.push_str(" ---\n");
                            } else {
                                text.push_str("--- Slide 1 ---\n");
                            }
                            last_was_slide_marker = true;
                        } else {
                            last_was_slide_marker = false;
                        }
                        
                        text.push_str(&s);
                        text.push(' ');
                    }
                }
            }
            current_word.clear();
        } else if !current_word.is_empty() {
            // Non-text byte encountered
            if current_word.len() >= 3 {
                if let Ok(s) = String::from_utf8(current_word.clone()) {
                    if is_likely_text(&s) {
                        text.push_str(&s);
                        text.push(' ');
                    }
                }
            }
            current_word.clear();
        }
    }
    
    // Handle any remaining text
    if current_word.len() >= 3 {
        if let Ok(s) = String::from_utf8(current_word) {
            if is_likely_text(&s) {
                text.push_str(&s);
            }
        }
    }
    
    // Clean up the text
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Check if a string is likely to be actual text content
fn is_likely_text(s: &str) -> bool {
    if s.len() < 3 {
        return false;
    }
    
    // Filter out common binary patterns
    if s.contains("Microsoft") && s.len() < 20 {
        return false; // Likely metadata, not content
    }
    
    // Count alphabetic characters
    let alpha_count = s.chars().filter(|c| c.is_alphabetic()).count();
    let total_chars = s.chars().count();
    
    // At least 50% should be alphabetic for it to be considered text
    alpha_count as f32 / total_chars as f32 >= 0.5
}

/// Heuristic to determine if we should add a slide separator
fn should_add_slide_separator(s: &str, current_text: &str, last_was_marker: bool) -> bool {
    // Don't add consecutive markers
    if last_was_marker {
        return false;
    }
    
    // If we have substantial text already and find a title-like string
    if current_text.len() > 100 && s.len() > 10 && s.len() < 100 {
        // Check if it looks like a title (starts with capital, no lowercase at start)
        let first_char = s.chars().next();
        if let Some(c) = first_char {
            if c.is_uppercase() && !s.contains("  ") {
                return true;
            }
        }
    }
    
    false
}

/// Extract metadata from PPT file
///
/// Attempts to extract basic metadata from the OLE2 structure.
/// This includes document properties if available.
fn extract_metadata(data: &[u8]) -> Result<Metadata> {
    let mut metadata = Metadata::new();
    
    // Add a note about limited support
    metadata.insert(
        "parser_note".to_string(),
        MetadataValue::Text(
            "Basic PPT support - full parsing requires OLE2 structure analysis".to_string(),
        ),
    );
    
    // Try to extract basic properties from OLE2 structure
    if let Ok(ole_metadata) = extract_ole2_metadata(data) {
        // Merge OLE2 metadata into our metadata
        for (key, value) in ole_metadata {
            metadata.insert(key, value);
        }
    }
    
    // Try to estimate slide count from the data
    if let Some(slide_count) = estimate_slide_count(data) {
        metadata.insert(
            "slide_count".to_string(),
            MetadataValue::Number(slide_count as i64),
        );
    }
    
    Ok(metadata)
}

/// Estimate the number of slides in the presentation
///
/// This is a heuristic approach that looks for slide-related markers
/// in the binary data.
fn estimate_slide_count(data: &[u8]) -> Option<usize> {
    // Look for PowerPoint slide markers in the binary data
    // This is a simplified heuristic approach
    
    let mut slide_markers = 0;
    
    // Search for common slide-related byte patterns
    // PowerPoint files often have specific record types for slides
    for i in 0..data.len().saturating_sub(4) {
        // Look for slide record markers (this is a simplified check)
        // Real implementation would parse the OLE2 structure properly
        if data[i] == 0x0F && data[i + 1] == 0x00 {
            // Potential slide marker
            slide_markers += 1;
        }
    }
    
    // If we found markers, estimate slides (with some filtering)
    if slide_markers > 0 {
        // Typically there are multiple markers per slide, so divide by a factor
        let estimated = (slide_markers / 10).max(1);
        Some(estimated.min(1000)) // Cap at reasonable maximum
    } else {
        None
    }
}

/// Extract metadata from OLE2 SummaryInformation stream
///
/// This attempts to parse the OLE2 directory structure and extract
/// properties from the SummaryInformation stream if present.
fn extract_ole2_metadata(data: &[u8]) -> Result<Vec<(String, MetadataValue)>> {
    let mut metadata = Vec::new();
    
    // Basic OLE2 structure parsing
    // The OLE2 format is complex, so this is a simplified approach
    // that looks for common property patterns
    
    // Look for common metadata strings in the binary data
    // This is a heuristic approach that works for many PPT files
    
    // Try to find title
    if let Some(title) = extract_property_string(data, b"Title") {
        if !title.is_empty() && title.len() < 200 {
            metadata.push(("title".to_string(), MetadataValue::Text(title)));
        }
    }
    
    // Try to find author
    if let Some(author) = extract_property_string(data, b"Author") {
        if !author.is_empty() && author.len() < 200 {
            metadata.push(("author".to_string(), MetadataValue::Text(author)));
        }
    }
    
    // Try to find subject
    if let Some(subject) = extract_property_string(data, b"Subject") {
        if !subject.is_empty() && subject.len() < 200 {
            metadata.push(("subject".to_string(), MetadataValue::Text(subject)));
        }
    }
    
    Ok(metadata)
}

/// Extract a property string from OLE2 data
///
/// This is a heuristic approach that searches for property names
/// and extracts the following string data.
fn extract_property_string(data: &[u8], property_name: &[u8]) -> Option<String> {
    // Search for the property name in the data
    for i in 0..data.len().saturating_sub(property_name.len() + 100) {
        if data[i..i + property_name.len()] == *property_name {
            // Found the property name, try to extract the value
            // Skip ahead past the property name and some bytes
            let start = i + property_name.len();
            
            // Look for a string in the next 100 bytes
            for offset in 0..100 {
                let pos = start + offset;
                if pos >= data.len() {
                    break;
                }
                
                // Try to extract a null-terminated string
                if let Some(value) = extract_null_terminated_string(&data[pos..], 100) {
                    if value.len() >= 2 && is_likely_text(&value) {
                        return Some(value);
                    }
                }
            }
        }
    }
    
    None
}

/// Extract a null-terminated string from binary data
fn extract_null_terminated_string(data: &[u8], max_len: usize) -> Option<String> {
    let mut bytes = Vec::new();
    
    for &byte in data.iter().take(max_len) {
        if byte == 0 {
            break;
        }
        if byte >= 32 && byte <= 126 {
            bytes.push(byte);
        } else if !bytes.is_empty() {
            // Non-printable character after we've started collecting
            break;
        }
    }
    
    if bytes.len() >= 2 {
        String::from_utf8(bytes).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_types() {
        let parser = PptParser;
        let types = parser.supported_types();
        assert!(types.contains(&"application/vnd.ms-powerpoint"));
        assert!(types.contains(&"application/ppt"));
    }

    #[test]
    fn test_parser_name() {
        let parser = PptParser;
        assert_eq!(parser.name(), "PptParser");
    }

    #[test]
    fn test_ole2_detection() {
        let ole2_header = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        assert!(is_ole2_file(&ole2_header));
        
        let invalid_header = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert!(!is_ole2_file(&invalid_header));
    }

    #[test]
    fn test_is_likely_text() {
        assert!(is_likely_text("Hello World"));
        assert!(is_likely_text("Presentation Title"));
        assert!(!is_likely_text("123"));
        assert!(!is_likely_text("!!!"));
        assert!(!is_likely_text("ab")); // Too short
    }
}
