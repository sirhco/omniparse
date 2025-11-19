//! XLS (Legacy Excel) parser implementation

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::utils::security::{validate_file_size, FileSizeLimits};
use calamine::{open_workbook_auto_from_rs, Data, Reader};
use std::io::Cursor;
use std::panic;

/// Parser for Microsoft Excel XLS files (legacy binary format)
pub struct XlsParser;

impl XlsParser {
    /// Extract all sheets from the workbook and format as text
    fn extract_sheets(workbook: &mut calamine::Sheets<Cursor<Vec<u8>>>) -> Result<String> {
        let mut content = String::new();
        let sheet_names = workbook.sheet_names().to_vec();

        for sheet_name in &sheet_names {
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&format!("=== Sheet: {} ===\n", sheet_name));

            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                for row in range.rows() {
                    let row_values: Vec<String> = row
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
                        .collect();

                    content.push_str(&row_values.join(","));
                    content.push('\n');
                }
            }
        }

        Ok(content)
    }

    /// Extract metadata from the workbook
    fn extract_metadata(workbook: &mut calamine::Sheets<Cursor<Vec<u8>>>) -> Metadata {
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

        // Note: calamine's XLS reader has limited support for document properties
        // We extract what's available from the workbook structure

        metadata
    }
}

impl Parser for XlsParser {
    fn name(&self) -> &str {
        "XlsParser"
    }

    fn supported_types(&self) -> &[&str] {
        &[
            "application/vnd.ms-excel",
            "application/xls",
        ]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Validate file size
        validate_file_size(data, FileSizeLimits::XLS, "XLS")?;
        
        // Check if file is large enough to be a valid XLS file
        if data.len() < 1024 {
            return Err(Error::CorruptedFile(
                "File too small to be a valid XLS file".to_string()
            ));
        }

        // Create a cursor from the data
        let cursor = Cursor::new(data.to_vec());

        // Try to open the workbook - wrap in catch_unwind to handle panics from calamine
        let workbook_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            open_workbook_auto_from_rs(cursor)
        }));

        let mut workbook = match workbook_result {
            Ok(Ok(wb)) => wb,
            Ok(Err(e)) => {
                let error_msg = e.to_string();
                return Err(if error_msg.contains("password") || error_msg.contains("encrypted") {
                    Error::ParseError("File is password-protected or encrypted".to_string())
                } else if error_msg.contains("corrupt") || error_msg.contains("invalid") {
                    Error::CorruptedFile(format!("Corrupted XLS file: {}", error_msg))
                } else {
                    Error::ParseError(format!("Failed to parse XLS: {}", error_msg))
                });
            }
            Err(_) => {
                return Err(Error::CorruptedFile(
                    "Corrupted or incomplete XLS file structure (panic during parsing)".to_string()
                ));
            }
        };

        // Extract content and metadata - also wrap in catch_unwind
        let content_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            Self::extract_sheets(&mut workbook)
        }));

        let content_text = match content_result {
            Ok(Ok(text)) => text,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(Error::CorruptedFile(
                    "Corrupted XLS file structure (panic during content extraction)".to_string()
                ));
            }
        };

        let metadata = Self::extract_metadata(&mut workbook);

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(content_text),
            metadata,
            detection_confidence: 0.90,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_types() {
        let parser = XlsParser;
        let types = parser.supported_types();
        assert!(types.contains(&"application/vnd.ms-excel"));
        assert!(types.contains(&"application/xls"));
    }

    #[test]
    fn test_parser_name() {
        let parser = XlsParser;
        assert_eq!(parser.name(), "XlsParser");
    }
}
