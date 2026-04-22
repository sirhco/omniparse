//! TAR archive parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::utils::security::is_safe_archive_path;
use chrono::{DateTime, TimeZone, Utc};
use std::io::Cursor;
use tar::Archive;

/// Parser for TAR archives
pub struct TarParser;

impl TarParser {
    /// Extract file information from TAR archive
    fn extract_file_info(data: &[u8]) -> Result<Vec<FileInfo>> {
        let cursor = Cursor::new(data);
        let mut archive = Archive::new(cursor);
        
        let mut files = Vec::new();
        
        for entry_result in archive.entries()
            .map_err(|e| Error::ParseError(format!("Failed to read TAR entries: {}", e)))? 
        {
            let entry = entry_result
                .map_err(|e| Error::ParseError(format!("Failed to read TAR entry: {}", e)))?;
            
            let header = entry.header();
            let path = entry.path()
                .map_err(|e| Error::ParseError(format!("Invalid path in TAR: {}", e)))?
                .to_string_lossy()
                .to_string();
            
            let size = header.size()
                .map_err(|e| Error::ParseError(format!("Invalid size in TAR: {}", e)))?;
            
            let mtime = header.mtime()
                .map_err(|e| Error::ParseError(format!("Invalid mtime in TAR: {}", e)))?;
            
            let modified = Utc.timestamp_opt(mtime as i64, 0).single();
            
            files.push(FileInfo {
                path,
                size,
                modified,
            });
        }
        
        Ok(files)
    }
}

struct FileInfo {
    path: String,
    size: u64,
    modified: Option<DateTime<Utc>>,
}

impl Parser for TarParser {
    fn supported_types(&self) -> &[&str] {
        &["application/x-tar", "application/tar"]
    }
    
    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        let file_infos = Self::extract_file_info(data)?;
        let file_count = file_infos.len();

        let total_size: u64 = file_infos.iter().map(|f| f.size).sum();
        let contains_unsafe_paths = file_infos.iter().any(|f| !is_safe_archive_path(&f.path));

        let file_list: Vec<String> = file_infos.iter()
            .map(|f| format!("{} ({} bytes)", f.path, f.size))
            .collect();

        let content_text = if file_list.is_empty() {
            "Empty TAR archive".to_string()
        } else {
            format!(
                "TAR Archive Contents ({} files):\n{}",
                file_count,
                file_list.join("\n")
            )
        };

        let mut metadata = Metadata::new();
        metadata.insert("file_count".to_string(), MetadataValue::Number(file_count as i64));
        metadata.insert("total_size".to_string(), MetadataValue::Number(total_size as i64));
        metadata.insert(
            "contains_unsafe_paths".to_string(),
            MetadataValue::Boolean(contains_unsafe_paths),
        );
        
        // Add file list with details
        let file_details: Vec<MetadataValue> = file_infos.iter()
            .map(|f| {
                MetadataValue::Text(format!(
                    "{} (size: {}, modified: {})",
                    f.path,
                    f.size,
                    f.modified
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_else(|| "unknown".to_string())
                ))
            })
            .collect();
        
        metadata.insert("files".to_string(), MetadataValue::List(file_details));
        
        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(content_text),
            metadata,
            detection_confidence: 0.0, // Will be set by the extractor
        })
    }
    
    fn name(&self) -> &str {
        "TarParser"
    }
}
