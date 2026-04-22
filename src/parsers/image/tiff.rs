//! TIFF image parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::parsers::image::exif::extract_exif_fields;
use image::io::Reader as ImageReader;
use std::io::Cursor;

/// Parser for TIFF images
pub struct TiffParser;

impl Parser for TiffParser {
    fn supported_types(&self) -> &[&str] {
        &["image/tiff", "image/tif"]
    }
    
    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Load image to extract basic information
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| Error::ParseError(format!("Failed to read TIFF: {}", e)))?
            .decode()
            .map_err(|e| Error::ParseError(format!("Failed to decode TIFF: {}", e)))?;
        
        let mut metadata = Metadata::new();
        
        // Extract dimensions
        let width = img.width();
        let height = img.height();
        metadata.insert("width".to_string(), MetadataValue::Number(width as i64));
        metadata.insert("height".to_string(), MetadataValue::Number(height as i64));
        
        // Extract color type information
        let color_type = img.color();
        metadata.insert("color_type".to_string(), MetadataValue::Text(format!("{:?}", color_type)));
        
        // Extract TIFF-specific metadata
        if let Ok(tiff_metadata) = Self::extract_tiff_tags(data) {
            for (key, value) in tiff_metadata {
                metadata.insert(key, value);
            }
        }

        // TIFF IFD layout is identical to EXIF; reuse the shared helper for real values.
        for (key, value) in extract_exif_fields(data) {
            metadata.insert(key, value);
        }
        
        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::None,
            metadata,
            detection_confidence: 0.0,
        })
    }
    
    fn name(&self) -> &str {
        "TiffParser"
    }
}

impl TiffParser {
    /// Extract TIFF tags as metadata
    fn extract_tiff_tags(data: &[u8]) -> Result<Vec<(String, MetadataValue)>> {
        let mut tags = Vec::new();
        
        // Check TIFF header
        if data.len() < 8 {
            return Err(Error::ParseError("File too small to be TIFF".to_string()));
        }
        
        // Determine byte order (II = little-endian, MM = big-endian)
        let is_little_endian = match &data[0..2] {
            b"II" => true,
            b"MM" => false,
            _ => return Err(Error::ParseError("Invalid TIFF byte order marker".to_string())),
        };
        
        tags.push((
            "byte_order".to_string(),
            MetadataValue::Text(if is_little_endian { "little-endian" } else { "big-endian" }.to_string()),
        ));
        
        // Verify magic number (42)
        let magic = if is_little_endian {
            u16::from_le_bytes([data[2], data[3]])
        } else {
            u16::from_be_bytes([data[2], data[3]])
        };
        
        if magic != 42 {
            return Err(Error::ParseError(format!("Invalid TIFF magic number: {}", magic)));
        }
        
        // Read IFD offset
        let ifd_offset = if is_little_endian {
            u32::from_le_bytes([data[4], data[5], data[6], data[7]])
        } else {
            u32::from_be_bytes([data[4], data[5], data[6], data[7]])
        } as usize;
        
        // Count IFDs (Image File Directories) for multi-page detection
        let page_count = Self::count_ifds(data, ifd_offset, is_little_endian);
        tags.push((
            "page_count".to_string(),
            MetadataValue::Number(page_count as i64),
        ));
        
        if page_count > 1 {
            tags.push((
                "multi_page".to_string(),
                MetadataValue::Boolean(true),
            ));
        }
        
        let _ = ifd_offset; // structural-only walk; real IFD tag values come from extract_exif_fields

        Ok(tags)
    }
    
    /// Count the number of IFDs (pages) in the TIFF file
    fn count_ifds(data: &[u8], mut ifd_offset: usize, is_little_endian: bool) -> usize {
        let mut count = 0;
        
        while ifd_offset > 0 && ifd_offset + 2 <= data.len() {
            count += 1;
            
            // Read number of entries
            let num_entries = if is_little_endian {
                u16::from_le_bytes([data[ifd_offset], data[ifd_offset + 1]])
            } else {
                u16::from_be_bytes([data[ifd_offset], data[ifd_offset + 1]])
            } as usize;
            
            // Calculate next IFD offset position
            let next_ifd_pos = ifd_offset + 2 + (num_entries * 12);
            
            if next_ifd_pos + 4 > data.len() {
                break;
            }
            
            // Read next IFD offset
            ifd_offset = if is_little_endian {
                u32::from_le_bytes([
                    data[next_ifd_pos],
                    data[next_ifd_pos + 1],
                    data[next_ifd_pos + 2],
                    data[next_ifd_pos + 3],
                ])
            } else {
                u32::from_be_bytes([
                    data[next_ifd_pos],
                    data[next_ifd_pos + 1],
                    data[next_ifd_pos + 2],
                    data[next_ifd_pos + 3],
                ])
            } as usize;
        }
        
        count
    }
    
}
