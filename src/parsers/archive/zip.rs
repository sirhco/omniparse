//! ZIP archive parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::utils::security::is_safe_archive_path;
use std::io::Cursor;
use zip::ZipArchive;

/// Parser for ZIP archives
pub struct ZipParser;

struct ZipSummary {
    files: Vec<String>,
    total_size: u64,
    compressed_size: u64,
    contains_unsafe_paths: bool,
}

impl ZipParser {
    /// Walk the archive once, collecting name, size, and safety info.
    fn summarize(archive: &mut ZipArchive<Cursor<&[u8]>>) -> ZipSummary {
        let mut summary = ZipSummary {
            files: Vec::with_capacity(archive.len()),
            total_size: 0,
            compressed_size: 0,
            contains_unsafe_paths: false,
        };
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().to_string();
                if !is_safe_archive_path(&name) {
                    summary.contains_unsafe_paths = true;
                }
                summary.files.push(name);
                summary.total_size += file.size();
                summary.compressed_size += file.compressed_size();
            }
        }
        summary
    }
}

impl Parser for ZipParser {
    fn supported_types(&self) -> &[&str] {
        &["application/zip", "application/x-zip-compressed"]
    }
    
    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| Error::ParseError(format!("Failed to open ZIP archive: {}", e)))?;

        let summary = Self::summarize(&mut archive);
        let file_count = summary.files.len();

        let compression_ratio = if summary.total_size > 0 {
            (summary.compressed_size as f64) / (summary.total_size as f64)
        } else {
            0.0
        };

        let content_text = if summary.files.is_empty() {
            "Empty ZIP archive".to_string()
        } else {
            format!(
                "ZIP Archive Contents ({} files):\n{}",
                file_count,
                summary.files.join("\n")
            )
        };

        let mut metadata = Metadata::new();
        metadata.insert("file_count".to_string(), MetadataValue::Number(file_count as i64));
        metadata.insert("total_size".to_string(), MetadataValue::Number(summary.total_size as i64));
        metadata.insert("compressed_size".to_string(), MetadataValue::Number(summary.compressed_size as i64));
        metadata.insert("compression_ratio".to_string(), MetadataValue::Float(compression_ratio));
        metadata.insert(
            "contains_unsafe_paths".to_string(),
            MetadataValue::Boolean(summary.contains_unsafe_paths),
        );
        metadata.insert(
            "files".to_string(),
            MetadataValue::List(summary.files.into_iter().map(MetadataValue::Text).collect()),
        );

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(content_text),
            metadata,
            detection_confidence: 0.0,
        })
    }
    
    fn name(&self) -> &str {
        "ZipParser"
    }
}
