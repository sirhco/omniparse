//! CSV parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use csv::ReaderBuilder;
use std::io::Cursor;

/// Parser for CSV files
pub struct CsvParser;

impl CsvParser {
    /// Detect the delimiter used in the CSV
    fn detect_delimiter(data: &[u8]) -> u8 {
        // Try common delimiters and see which one produces the most consistent columns
        let delimiters = [b',', b';', b'\t', b'|'];
        let sample_size = data.len().min(1024);
        let sample = &data[..sample_size];
        
        let mut best_delimiter = b',';
        let mut best_score = 0;
        
        for &delimiter in &delimiters {
            if let Ok(text) = std::str::from_utf8(sample) {
                let lines: Vec<&str> = text.lines().take(10).collect();
                if lines.is_empty() {
                    continue;
                }
                
                // Count delimiters per line
                let counts: Vec<usize> = lines.iter()
                    .map(|line| line.bytes().filter(|&b| b == delimiter).count())
                    .collect();
                
                // Check consistency (all lines should have similar counts)
                if let Some(&first_count) = counts.first() {
                    if first_count > 0 {
                        let consistent = counts.iter().filter(|&&c| c == first_count).count();
                        let score = consistent * first_count;
                        
                        if score > best_score {
                            best_score = score;
                            best_delimiter = delimiter;
                        }
                    }
                }
            }
        }
        
        best_delimiter
    }
}

impl Parser for CsvParser {
    fn supported_types(&self) -> &[&str] {
        &["text/csv", "text/tab-separated-values"]
    }
    
    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Detect delimiter
        let delimiter = Self::detect_delimiter(data);
        
        // Build CSV reader
        let cursor = Cursor::new(data);
        let mut reader = ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(true)
            .from_reader(cursor);
        
        // Extract headers
        let headers = reader.headers()
            .map_err(|e| Error::ParseError(format!("Failed to read CSV headers: {}", e)))?
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        
        let column_count = headers.len();
        
        // Read all records and convert to text
        let mut text_lines = vec![headers.join(", ")];
        let mut row_count = 0;
        
        for result in reader.records() {
            match result {
                Ok(record) => {
                    let line = record.iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    text_lines.push(line);
                    row_count += 1;
                }
                Err(e) => {
                    return Err(Error::ParseError(format!("Failed to read CSV record: {}", e)));
                }
            }
        }
        
        let text = text_lines.join("\n");
        
        // Build metadata
        let mut metadata = Metadata::new();
        metadata.insert("column_count".to_string(), MetadataValue::Number(column_count as i64));
        metadata.insert("row_count".to_string(), MetadataValue::Number(row_count));
        metadata.insert("headers".to_string(), MetadataValue::List(
            headers.into_iter().map(MetadataValue::Text).collect()
        ));
        metadata.insert("delimiter".to_string(), MetadataValue::Text(
            match delimiter {
                b',' => "comma".to_string(),
                b';' => "semicolon".to_string(),
                b'\t' => "tab".to_string(),
                b'|' => "pipe".to_string(),
                _ => format!("0x{:02x}", delimiter),
            }
        ));
        
        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata,
            detection_confidence: 0.0, // Will be set by the extractor
        })
    }
    
    fn name(&self) -> &str {
        "CsvParser"
    }
}
