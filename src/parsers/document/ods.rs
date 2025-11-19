//! ODS (OpenDocument Spreadsheet) parser implementation

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::utils::security::{validate_file_size, validate_zip_structure, check_xml_bomb, FileSizeLimits};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Parser for OpenDocument Spreadsheet (ODS) files
pub struct OdsParser;

impl Parser for OdsParser {
    fn name(&self) -> &str {
        "OdsParser"
    }

    fn supported_types(&self) -> &[&str] {
        &[
            "application/vnd.oasis.opendocument.spreadsheet",
            "application/ods",
        ]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Validate file size
        validate_file_size(data, FileSizeLimits::ODS, "ODS")?;
        
        // Validate ZIP structure and check for ZIP bombs
        validate_zip_structure(data, Some(&["content.xml", "meta.xml"]))?;
        
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor).map_err(|e| {
            Error::ParseError(format!("Failed to open ODS archive: {}", e))
        })?;

        // Extract text content from content.xml
        let (text, table_info) = extract_tables(&mut archive)?;

        // Extract metadata from meta.xml
        let mut metadata = extract_metadata(&mut archive)?;

        // Add table information to metadata
        metadata.insert(
            "table_count".to_string(),
            MetadataValue::Number(table_info.len() as i64),
        );

        let table_names: Vec<String> = table_info.iter().map(|t| t.name.clone()).collect();
        metadata.insert(
            "table_names".to_string(),
            MetadataValue::Text(table_names.join(", ")),
        );

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata,
            detection_confidence: 0.95,
        })
    }
}

/// Information about a table in the spreadsheet
struct TableInfo {
    name: String,
    row_count: usize,
    column_count: usize,
}

/// Extract tables from content.xml
fn extract_tables(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<(String, Vec<TableInfo>)> {
    let mut content_file = archive
        .by_name("content.xml")
        .map_err(|e| Error::ParseError(format!("Failed to find content.xml: {}", e)))?;

    let mut xml_content = String::new();
    content_file
        .read_to_string(&mut xml_content)
        .map_err(|e| Error::ParseError(format!("Failed to read content.xml: {}", e)))?;

    // Check for XML bombs
    check_xml_bomb(&xml_content)?;

    parse_tables(&xml_content)
}

/// Parse tables from content.xml
fn parse_tables(xml_content: &str) -> Result<(String, Vec<TableInfo>)> {
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut all_text = String::new();
    let mut table_info = Vec::new();
    let mut buf = Vec::new();

    let mut in_table = false;
    let mut in_row = false;
    let mut in_cell = false;
    let mut current_table_name = String::new();
    let mut current_row = Vec::new();
    let mut all_rows = Vec::new();
    let mut max_columns = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = e.name();
                let local_name_bytes = name.local_name();
                let local_name = local_name_bytes.as_ref();

                match local_name {
                    b"table" => {
                        in_table = true;
                        // Extract table name from table:name attribute
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                if attr.key.local_name().as_ref() == b"name" {
                                    current_table_name = String::from_utf8_lossy(&attr.value).to_string();
                                    break;
                                }
                            }
                        }
                    }
                    b"table-row" if in_table => {
                        in_row = true;
                        current_row.clear();
                    }
                    b"table-cell" if in_row => {
                        in_cell = true;
                        // Check for repeated cells attribute
                        let mut repeat_count = 1;
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                if attr.key.local_name().as_ref() == b"number-columns-repeated" {
                                    if let Ok(count_str) = String::from_utf8(attr.value.to_vec()) {
                                        repeat_count = count_str.parse().unwrap_or(1);
                                    }
                                }
                            }
                        }
                        // Will be filled with actual text or empty strings
                        for _ in 0..repeat_count {
                            current_row.push(String::new());
                        }
                    }
                    b"p" if in_cell => {
                        // Paragraph inside cell - we'll collect text
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if in_cell => {
                let content = e
                    .unescape()
                    .map_err(|e| Error::ParseError(format!("Failed to unescape text: {}", e)))?;
                let text = content.trim();
                if !text.is_empty() && !current_row.is_empty() {
                    // Add to the last cell in current_row
                    if let Some(last_cell) = current_row.last_mut() {
                        if !last_cell.is_empty() {
                            last_cell.push(' ');
                        }
                        last_cell.push_str(text);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                let local_name_bytes = name.local_name();
                let local_name = local_name_bytes.as_ref();

                match local_name {
                    b"table-cell" => {
                        in_cell = false;
                    }
                    b"table-row" => {
                        in_row = false;
                        if !current_row.is_empty() {
                            max_columns = max_columns.max(current_row.len());
                            all_rows.push(current_row.clone());
                        }
                    }
                    b"table" => {
                        in_table = false;
                        
                        // Format table content
                        if !all_text.is_empty() {
                            all_text.push_str("\n\n");
                        }
                        all_text.push_str(&format!("=== Table: {} ===\n", current_table_name));
                        
                        for row in &all_rows {
                            all_text.push_str(&row.join(","));
                            all_text.push('\n');
                        }

                        // Save table info
                        table_info.push(TableInfo {
                            name: current_table_name.clone(),
                            row_count: all_rows.len(),
                            column_count: max_columns,
                        });

                        // Reset for next table
                        all_rows.clear();
                        max_columns = 0;
                        current_table_name.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(Error::ParseError(format!(
                    "Error parsing content.xml: {}",
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok((all_text, table_info))
}

/// Extract metadata from meta.xml
fn extract_metadata(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<Metadata> {
    let mut metadata = Metadata::new();

    // Try to read meta.xml
    let meta_result = archive.by_name("meta.xml");

    if let Ok(mut meta_file) = meta_result {
        let mut xml_content = String::new();
        if meta_file.read_to_string(&mut xml_content).is_ok() {
            parse_meta_properties(&xml_content, &mut metadata)?;
        }
    }

    Ok(metadata)
}

/// Parse meta properties XML
fn parse_meta_properties(xml_content: &str, metadata: &mut Metadata) -> Result<()> {
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut current_element = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                let local_name_bytes = name.local_name();
                let local_name = String::from_utf8_lossy(local_name_bytes.as_ref());
                current_element = local_name.to_string();
            }
            Ok(Event::Text(e)) => {
                if !current_element.is_empty() {
                    let content = e
                        .unescape()
                        .map_err(|e| Error::ParseError(format!("Failed to unescape text: {}", e)))?;
                    let text = content.trim().to_string();

                    if !text.is_empty() {
                        match current_element.as_str() {
                            "title" => {
                                metadata.insert("title".to_string(), MetadataValue::Text(text));
                            }
                            "initial-creator" | "creator" => {
                                metadata.insert("author".to_string(), MetadataValue::Text(text));
                            }
                            "subject" => {
                                metadata.insert("subject".to_string(), MetadataValue::Text(text));
                            }
                            "description" => {
                                metadata
                                    .insert("description".to_string(), MetadataValue::Text(text));
                            }
                            "creation-date" => {
                                metadata
                                    .insert("creation_date".to_string(), MetadataValue::Text(text));
                            }
                            "date" => {
                                metadata
                                    .insert("modified_date".to_string(), MetadataValue::Text(text));
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::End(_)) => {
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(Error::ParseError(format!("Error parsing meta.xml: {}", e)))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_types() {
        let parser = OdsParser;
        let types = parser.supported_types();
        assert!(types.contains(&"application/vnd.oasis.opendocument.spreadsheet"));
        assert!(types.contains(&"application/ods"));
    }

    #[test]
    fn test_parser_name() {
        let parser = OdsParser;
        assert_eq!(parser.name(), "OdsParser");
    }
}
