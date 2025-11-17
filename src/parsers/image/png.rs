//! PNG image parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use image::io::Reader as ImageReader;
use std::io::Cursor;

/// Parser for PNG images
pub struct PngParser;

impl Parser for PngParser {
    fn supported_types(&self) -> &[&str] {
        &["image/png"]
    }
    
    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Load image to extract basic information
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| Error::ParseError(format!("Failed to read PNG: {}", e)))?
            .decode()
            .map_err(|e| Error::ParseError(format!("Failed to decode PNG: {}", e)))?;
        
        let mut metadata = Metadata::new();
        
        // Extract dimensions
        let width = img.width();
        let height = img.height();
        metadata.insert("width".to_string(), MetadataValue::Number(width as i64));
        metadata.insert("height".to_string(), MetadataValue::Number(height as i64));
        
        // Extract color type information
        let color_type = img.color();
        metadata.insert("color_type".to_string(), MetadataValue::Text(format!("{:?}", color_type)));
        
        // Extract PNG metadata chunks (tEXt, iTXt, zTXt)
        if let Ok(chunks) = Self::extract_png_chunks(data) {
            for (key, value) in chunks {
                metadata.insert(key, value);
            }
        }
        
        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::None,
            metadata,
            detection_confidence: 0.0,
        })
    }
    
    fn name(&self) -> &str {
        "PngParser"
    }
}

impl PngParser {
    /// Extract PNG metadata chunks (tEXt, iTXt, zTXt)
    fn extract_png_chunks(data: &[u8]) -> Result<Vec<(String, MetadataValue)>> {
        let mut chunks = Vec::new();
        
        // PNG signature is 8 bytes: 137 80 78 71 13 10 26 10
        if data.len() < 8 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
            return Err(Error::ParseError("Invalid PNG signature".to_string()));
        }
        
        let mut pos = 8;
        
        while pos + 12 <= data.len() {
            // Read chunk length (4 bytes, big-endian)
            let length = u32::from_be_bytes([
                data[pos],
                data[pos + 1],
                data[pos + 2],
                data[pos + 3],
            ]) as usize;
            
            // Read chunk type (4 bytes)
            let chunk_type = &data[pos + 4..pos + 8];
            
            // Check if we have enough data for the chunk
            if pos + 12 + length > data.len() {
                break;
            }
            
            // Extract text chunks
            match chunk_type {
                b"tEXt" => {
                    if let Some((key, value)) = Self::parse_text_chunk(&data[pos + 8..pos + 8 + length]) {
                        chunks.push((format!("text_{}", key), MetadataValue::Text(value)));
                    }
                }
                b"iTXt" => {
                    if let Some((key, value)) = Self::parse_itext_chunk(&data[pos + 8..pos + 8 + length]) {
                        chunks.push((format!("itext_{}", key), MetadataValue::Text(value)));
                    }
                }
                b"zTXt" => {
                    if let Some((key, value)) = Self::parse_ztext_chunk(&data[pos + 8..pos + 8 + length]) {
                        chunks.push((format!("ztext_{}", key), MetadataValue::Text(value)));
                    }
                }
                _ => {}
            }
            
            // Move to next chunk (length + type + data + CRC)
            pos += 12 + length;
        }
        
        Ok(chunks)
    }
    
    /// Parse tEXt chunk (uncompressed Latin-1 text)
    fn parse_text_chunk(data: &[u8]) -> Option<(String, String)> {
        // Find null separator between keyword and text
        let null_pos = data.iter().position(|&b| b == 0)?;
        
        let keyword = String::from_utf8_lossy(&data[..null_pos]).to_string();
        let text = String::from_utf8_lossy(&data[null_pos + 1..]).to_string();
        
        Some((keyword, text))
    }
    
    /// Parse iTXt chunk (international text, UTF-8)
    fn parse_itext_chunk(data: &[u8]) -> Option<(String, String)> {
        // Find null separator for keyword
        let null_pos = data.iter().position(|&b| b == 0)?;
        
        let keyword = String::from_utf8_lossy(&data[..null_pos]).to_string();
        
        // Skip compression flag and compression method
        if null_pos + 2 >= data.len() {
            return None;
        }
        
        // Find next null (language tag)
        let lang_start = null_pos + 2;
        let lang_end = data[lang_start..].iter().position(|&b| b == 0)? + lang_start;
        
        // Find next null (translated keyword)
        let trans_start = lang_end + 1;
        let trans_end = data[trans_start..].iter().position(|&b| b == 0).map(|p| p + trans_start);
        
        let text_start = trans_end.map(|p| p + 1).unwrap_or(trans_start);
        if text_start < data.len() {
            let text = String::from_utf8_lossy(&data[text_start..]).to_string();
            Some((keyword, text))
        } else {
            None
        }
    }
    
    /// Parse zTXt chunk (compressed text)
    fn parse_ztext_chunk(data: &[u8]) -> Option<(String, String)> {
        // Find null separator for keyword
        let null_pos = data.iter().position(|&b| b == 0)?;
        
        let keyword = String::from_utf8_lossy(&data[..null_pos]).to_string();
        
        // For simplicity, we'll note that the text is compressed
        // Full implementation would decompress using zlib
        Some((keyword, "[compressed text]".to_string()))
    }
}
