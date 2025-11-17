//! Plain text parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;

/// Parser for plain text files
pub struct PlainTextParser;

impl PlainTextParser {
    /// Detect the encoding of the text data
    fn detect_encoding(data: &[u8]) -> &'static str {
        // Check for UTF-16 BOM
        if data.len() >= 2 {
            if data[0] == 0xFF && data[1] == 0xFE {
                return "UTF-16LE";
            }
            if data[0] == 0xFE && data[1] == 0xFF {
                return "UTF-16BE";
            }
        }
        
        // Check for UTF-8 BOM
        if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
            return "UTF-8";
        }
        
        // Try to validate as UTF-8
        if std::str::from_utf8(data).is_ok() {
            return "UTF-8";
        }
        
        // Check if it's ASCII (all bytes < 128)
        if data.iter().all(|&b| b < 128) {
            return "ASCII";
        }
        
        // Default to UTF-8 (will be handled as potentially invalid)
        "UTF-8"
    }
    
    /// Convert bytes to string based on detected encoding
    fn decode_text(data: &[u8], encoding: &str) -> Result<String> {
        match encoding {
            "UTF-16LE" => {
                // Skip BOM if present
                let start = if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xFE {
                    2
                } else {
                    0
                };
                
                let u16_data: Vec<u16> = data[start..]
                    .chunks_exact(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();
                
                String::from_utf16(&u16_data)
                    .map_err(|e| Error::ParseError(format!("Invalid UTF-16LE: {}", e)))
            }
            "UTF-16BE" => {
                // Skip BOM if present
                let start = if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
                    2
                } else {
                    0
                };
                
                let u16_data: Vec<u16> = data[start..]
                    .chunks_exact(2)
                    .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                    .collect();
                
                String::from_utf16(&u16_data)
                    .map_err(|e| Error::ParseError(format!("Invalid UTF-16BE: {}", e)))
            }
            "UTF-8" | "ASCII" => {
                // Skip UTF-8 BOM if present
                let start = if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
                    3
                } else {
                    0
                };
                
                String::from_utf8(data[start..].to_vec())
                    .map_err(|e| Error::ParseError(format!("Invalid UTF-8: {}", e)))
            }
            _ => Err(Error::ParseError(format!("Unsupported encoding: {}", encoding))),
        }
    }
}

impl Parser for PlainTextParser {
    fn supported_types(&self) -> &[&str] {
        &["text/plain"]
    }
    
    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Detect encoding
        let encoding = Self::detect_encoding(data);
        
        // Decode text
        let text = Self::decode_text(data, encoding)?;
        
        // Calculate metadata
        let character_count = text.chars().count();
        let line_count = text.lines().count();
        
        // Build metadata
        let mut metadata = Metadata::new();
        metadata.insert("character_count".to_string(), MetadataValue::Number(character_count as i64));
        metadata.insert("line_count".to_string(), MetadataValue::Number(line_count as i64));
        metadata.insert("encoding".to_string(), MetadataValue::Text(encoding.to_string()));
        
        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata,
            detection_confidence: 0.0, // Will be set by the extractor
        })
    }
    
    fn name(&self) -> &str {
        "PlainTextParser"
    }
}
