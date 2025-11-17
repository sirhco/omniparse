//! ZIP archive parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use std::io::Cursor;
use zip::ZipArchive;

/// Parser for ZIP archives
pub struct ZipParser;

impl ZipParser {
    /// Extract file list from ZIP archive
    fn extract_file_list(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Vec<String> {
        let mut files = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                files.push(file.name().to_string());
            }
        }
        files
    }
    
    /// Calculate total uncompressed size
    fn calculate_total_size(archive: &mut ZipArchive<Cursor<&[u8]>>) -> u64 {
        let mut total = 0u64;
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                total += file.size();
            }
        }
        total
    }
    
    /// Calculate total compressed size
    fn calculate_compressed_size(archive: &mut ZipArchive<Cursor<&[u8]>>) -> u64 {
        let mut total = 0u64;
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                total += file.compressed_size();
            }
        }
        total
    }
}

impl Parser for ZipParser {
    fn supported_types(&self) -> &[&str] {
        &["application/zip", "application/x-zip-compressed"]
    }
    
    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Create a cursor for the data
        let cursor = Cursor::new(data);
        
        // Open ZIP archive
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| Error::ParseError(format!("Failed to open ZIP archive: {}", e)))?;
        
        // Extract file list
        let file_list = Self::extract_file_list(&mut archive);
        let file_count = file_list.len();
        
        // Calculate sizes
        let total_size = Self::calculate_total_size(&mut archive);
        let compressed_size = Self::calculate_compressed_size(&mut archive);
        
        // Calculate compression ratio
        let compression_ratio = if total_size > 0 {
            (compressed_size as f64) / (total_size as f64)
        } else {
            0.0
        };
        
        // Create text content with file listing
        let content_text = if file_list.is_empty() {
            "Empty ZIP archive".to_string()
        } else {
            format!(
                "ZIP Archive Contents ({} files):\n{}",
                file_count,
                file_list.join("\n")
            )
        };
        
        // Build metadata
        let mut metadata = Metadata::new();
        metadata.insert("file_count".to_string(), MetadataValue::Number(file_count as i64));
        metadata.insert("total_size".to_string(), MetadataValue::Number(total_size as i64));
        metadata.insert("compressed_size".to_string(), MetadataValue::Number(compressed_size as i64));
        metadata.insert("compression_ratio".to_string(), MetadataValue::Float(compression_ratio));
        metadata.insert(
            "files".to_string(),
            MetadataValue::List(
                file_list.into_iter()
                    .map(MetadataValue::Text)
                    .collect()
            )
        );
        
        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(content_text),
            metadata,
            detection_confidence: 0.0, // Will be set by the extractor
        })
    }
    
    fn name(&self) -> &str {
        "ZipParser"
    }
}
