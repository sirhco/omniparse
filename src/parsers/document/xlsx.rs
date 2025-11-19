//! XLSX (Excel) parser implementation

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::utils::security::{validate_file_size, validate_zip_structure, FileSizeLimits};
use calamine::{open_workbook_auto_from_rs, Data, Reader, Sheets};
use std::io::Cursor;

/// Parser for Microsoft Excel XLSX files
pub struct XlsxParser;

impl XlsxParser {
    /// Extract all sheets from the workbook and format as text
    fn extract_sheets(workbook: &mut Sheets<Cursor<Vec<u8>>>) -> Result<String> {
        // Pre-allocate with estimated capacity for better performance
        let mut content = String::with_capacity(8192);
        let sheet_names = workbook.sheet_names().to_vec();

        for sheet_name in &sheet_names {
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&format!("=== Sheet: {} ===\n", sheet_name));

            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                for row in range.rows() {
                    // Use iterator chain for more efficient string building
                    let row_str = row
                        .iter()
                        .map(|cell| match cell {
                            Data::Empty => String::new(),
                            Data::String(s) => s.clone(),
                            Data::Float(f) => f.to_string(),
                            Data::Int(i) => i.to_string(),
                            Data::Bool(b) => b.to_string(),
                            Data::Error(e) => format!("#ERROR: {:?}", e),
                            Data::DateTime(dt) => dt.to_string(),
                            Data::DateTimeIso(dt) => dt.clone(),
                            Data::DurationIso(d) => d.clone(),
                        })
                        .collect::<Vec<_>>()
                        .join(",");

                    content.push_str(&row_str);
                    content.push('\n');
                }
            }
        }

        Ok(content)
    }

    /// Extract metadata from the workbook
    fn extract_metadata(workbook: &mut Sheets<Cursor<Vec<u8>>>) -> Metadata {
        let mut metadata = Metadata::new();
        let sheet_names = workbook.sheet_names().to_vec();

        metadata.insert(
            "sheet_count".to_string(),
            MetadataValue::Number(sheet_names.len() as i64),
        );
        metadata.insert(
            "sheet_names".to_string(),
            MetadataValue::Text(sheet_names.join(", ")),
        );

        let mut total_rows = 0;
        let mut max_columns = 0;

        for sheet_name in &sheet_names {
            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                let (rows, cols) = range.get_size();

                total_rows += rows;
                max_columns = max_columns.max(cols);

                metadata.insert(
                    format!("sheet_{}_rows", sheet_name),
                    MetadataValue::Number(rows as i64),
                );
                metadata.insert(
                    format!("sheet_{}_columns", sheet_name),
                    MetadataValue::Number(cols as i64),
                );
            }
        }

        metadata.insert(
            "total_rows".to_string(),
            MetadataValue::Number(total_rows as i64),
        );
        metadata.insert(
            "max_columns".to_string(),
            MetadataValue::Number(max_columns as i64),
        );

        // Note: calamine's Metadata struct doesn't expose document properties
        // in a straightforward way. For now, we'll just include sheet information.

        metadata
    }
}

impl Parser for XlsxParser {
    fn name(&self) -> &str {
        "XlsxParser"
    }

    fn supported_types(&self) -> &[&str] {
        &[
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "application/xlsx",
        ]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Validate file size
        validate_file_size(data, FileSizeLimits::XLSX, "XLSX")?;
        
        // Validate ZIP structure and check for ZIP bombs
        validate_zip_structure(data, Some(&["[Content_Types].xml"]))?;
        
        // Create a cursor from the data
        let cursor = Cursor::new(data.to_vec());

        // Try to open the workbook
        let mut workbook = open_workbook_auto_from_rs(cursor).map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("password") || error_msg.contains("encrypted") {
                Error::ParseError("File is password-protected or encrypted".to_string())
            } else if error_msg.contains("corrupt") || error_msg.contains("invalid") {
                Error::CorruptedFile(format!("Corrupted XLSX file: {}", error_msg))
            } else {
                Error::ParseError(format!("Failed to parse XLSX: {}", error_msg))
            }
        })?;

        // Extract content and metadata
        let content_text = Self::extract_sheets(&mut workbook)?;
        let metadata = Self::extract_metadata(&mut workbook);

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(content_text),
            metadata,
            detection_confidence: 0.95,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_types() {
        let parser = XlsxParser;
        let types = parser.supported_types();
        assert!(types.contains(&"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
        assert!(types.contains(&"application/xlsx"));
    }

    #[test]
    fn test_parser_name() {
        let parser = XlsxParser;
        assert_eq!(parser.name(), "XlsxParser");
    }

    #[test]
    fn test_error_handling_invalid_data() {
        let parser = XlsxParser;
        let invalid_data = b"This is not an XLSX file";
        let result = parser.parse(invalid_data, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet");
        
        // Should return an error for invalid data
        assert!(result.is_err(), "Expected error for invalid XLSX data");
    }

    #[test]
    fn test_error_handling_empty_data() {
        let parser = XlsxParser;
        let empty_data = b"";
        let result = parser.parse(empty_data, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet");
        
        // Should return an error for empty data
        assert!(result.is_err(), "Expected error for empty XLSX data");
    }
}
