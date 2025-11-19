//! DOC (Legacy Microsoft Word) parser implementation
//!
//! This parser provides basic support for legacy Microsoft Word .doc files (OLE2 format).
//! Note: Full DOC parsing is complex due to the proprietary binary format. This implementation
//! provides basic text extraction and metadata where possible.

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::utils::security::{validate_file_size, FileSizeLimits};

/// Parser for Microsoft Word DOC files (legacy OLE2 format)
///
/// This parser provides basic support for extracting text and metadata from
/// legacy .doc files. Due to the complexity of the OLE2/Word binary format,
/// extraction capabilities may be limited compared to modern formats.
pub struct DocParser;

impl Parser for DocParser {
    fn name(&self) -> &str {
        "DocParser"
    }

    fn supported_types(&self) -> &[&str] {
        &[
            "application/msword",
            "application/doc",
        ]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Validate file size
        validate_file_size(data, FileSizeLimits::DOC, "DOC")?;
        
        // Validate OLE2 header
        if !is_ole2_file(data) {
            return Err(Error::ParseError(
                "Invalid DOC file: missing OLE2 header".to_string(),
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

/// Extract text content from DOC file
///
/// This is a basic implementation that attempts to extract readable text
/// from the OLE2 structure. Full DOC parsing would require a complete
/// implementation of the Word Binary File Format specification.
fn extract_text(data: &[u8]) -> Result<String> {
    // For now, we'll implement a basic text extraction strategy
    // A full implementation would parse the OLE2 structure and extract
    // the WordDocument stream, then parse the FIB (File Information Block)
    // and piece table to reconstruct the text.
    
    // Basic approach: scan for readable text sequences
    let text = extract_readable_text(data);
    
    if text.trim().is_empty() {
        return Err(Error::ParseError(
            "Unable to extract text from DOC file. Full DOC parsing requires complex OLE2 structure analysis.".to_string(),
        ));
    }
    
    Ok(text)
}

/// Extract readable ASCII/UTF-8 text from binary data
///
/// This is a fallback method that scans for readable text sequences.
/// It's not perfect but can extract some content from DOC files.
fn extract_readable_text(data: &[u8]) -> String {
    let mut text = String::new();
    let mut current_word = Vec::new();
    
    for &byte in data {
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
    
    // Count alphabetic characters
    let alpha_count = s.chars().filter(|c| c.is_alphabetic()).count();
    let total_chars = s.chars().count();
    
    // At least 50% should be alphabetic for it to be considered text
    alpha_count as f32 / total_chars as f32 >= 0.5
}

/// Extract metadata from DOC file
///
/// Attempts to extract basic metadata from the OLE2 structure.
/// This includes document properties if available.
fn extract_metadata(data: &[u8]) -> Result<Metadata> {
    let mut metadata = Metadata::new();
    
    // Add a note about limited support
    metadata.insert(
        "parser_note".to_string(),
        MetadataValue::Text(
            "Basic DOC support - full parsing requires OLE2 structure analysis".to_string(),
        ),
    );
    
    // Try to extract basic properties from OLE2 structure
    if let Ok(ole_metadata) = extract_ole2_metadata(data) {
        // Merge OLE2 metadata into our metadata
        for (key, value) in ole_metadata {
            metadata.insert(key, value);
        }
    }
    
    Ok(metadata)
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
    // This is a heuristic approach that works for many DOC files
    
    // Try to find title (often appears near the beginning after certain markers)
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
        let parser = DocParser;
        let types = parser.supported_types();
        assert!(types.contains(&"application/msword"));
        assert!(types.contains(&"application/doc"));
    }

    #[test]
    fn test_parser_name() {
        let parser = DocParser;
        assert_eq!(parser.name(), "DocParser");
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
        assert!(is_likely_text("Document"));
        assert!(!is_likely_text("123"));
        assert!(!is_likely_text("!!!"));
        assert!(!is_likely_text("ab")); // Too short
    }
}
