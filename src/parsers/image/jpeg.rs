//! JPEG image parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use image::io::Reader as ImageReader;
use std::io::Cursor;

/// Parser for JPEG images
pub struct JpegParser;

impl Parser for JpegParser {
    fn supported_types(&self) -> &[&str] {
        &["image/jpeg", "image/jpg"]
    }
    
    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Load image to extract basic information
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| Error::ParseError(format!("Failed to read JPEG: {}", e)))?
            .decode()
            .map_err(|e| Error::ParseError(format!("Failed to decode JPEG: {}", e)))?;
        
        let mut metadata = Metadata::new();
        
        // Extract dimensions
        let width = img.width();
        let height = img.height();
        metadata.insert("width".to_string(), MetadataValue::Number(width as i64));
        metadata.insert("height".to_string(), MetadataValue::Number(height as i64));
        
        // Extract color space information
        let color_type = img.color();
        metadata.insert("color_type".to_string(), MetadataValue::Text(format!("{:?}", color_type)));
        
        // Try to extract EXIF metadata
        if let Ok(exif_data) = Self::extract_exif(data) {
            for (key, value) in exif_data {
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
        "JpegParser"
    }
}

impl JpegParser {
    /// Extract EXIF metadata from JPEG data
    fn extract_exif(data: &[u8]) -> Result<Vec<(String, MetadataValue)>> {
        let mut exif_metadata = Vec::new();
        
        // Use exif crate if available, otherwise return empty
        // For now, we'll implement basic EXIF extraction
        // This is a simplified implementation - full EXIF parsing would require the exif crate
        
        // Look for EXIF marker (0xFFE1)
        if let Some(_exif_start) = Self::find_exif_marker(data) {
            // Basic EXIF data found
            exif_metadata.push((
                "exif_present".to_string(),
                MetadataValue::Boolean(true),
            ));
        }
        
        Ok(exif_metadata)
    }
    
    /// Find EXIF marker in JPEG data
    fn find_exif_marker(data: &[u8]) -> Option<usize> {
        for i in 0..data.len().saturating_sub(4) {
            if data[i] == 0xFF && data[i + 1] == 0xE1 {
                // Check for "Exif\0\0" identifier
                if i + 10 < data.len() 
                    && &data[i + 4..i + 10] == b"Exif\0\0" {
                    return Some(i);
                }
            }
        }
        None
    }
}
