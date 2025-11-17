//! ODT (OpenDocument Text) document parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Parser for ODT documents
pub struct OdtParser;

impl Parser for OdtParser {
    fn supported_types(&self) -> &[&str] {
        &[
            "application/vnd.oasis.opendocument.text",
            "application/odt",
        ]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor).map_err(|e| {
            Error::ParseError(format!("Failed to open ODT archive: {}", e))
        })?;

        // Extract text content from content.xml
        let text = extract_text(&mut archive)?;

        // Extract metadata from meta.xml
        let metadata = extract_metadata(&mut archive)?;

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata,
            detection_confidence: 1.0,
        })
    }

    fn name(&self) -> &str {
        "OdtParser"
    }
}

/// Extract text content from content.xml
fn extract_text(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<String> {
    // Try to find and read content.xml
    let mut content_file = archive
        .by_name("content.xml")
        .map_err(|e| Error::ParseError(format!("Failed to find content.xml: {}", e)))?;

    let mut xml_content = String::new();
    content_file
        .read_to_string(&mut xml_content)
        .map_err(|e| Error::ParseError(format!("Failed to read content.xml: {}", e)))?;

    // Parse XML and extract text
    let mut reader = Reader::from_str(&xml_content);
    reader.trim_text(true);

    let mut text = String::new();
    let mut buf = Vec::new();
    let mut in_body = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                let local_name_bytes = name.local_name();
                let local_name = local_name_bytes.as_ref();
                
                // Check if we're entering the body
                if local_name == b"body" {
                    in_body = true;
                }
                
                // Add newline for paragraph starts
                if in_body && local_name == b"p" {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_body {
                    let content = e
                        .unescape()
                        .map_err(|e| Error::ParseError(format!("Failed to unescape text: {}", e)))?;
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        if !text.is_empty() && !text.ends_with(' ') && !text.ends_with('\n') {
                            text.push(' ');
                        }
                        text.push_str(trimmed);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                let local_name_bytes = name.local_name();
                let local_name = local_name_bytes.as_ref();
                
                if local_name == b"body" {
                    in_body = false;
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

    Ok(text.trim().to_string())
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
                            "generator" => {
                                metadata
                                    .insert("generator".to_string(), MetadataValue::Text(text));
                            }
                            "editing-cycles" => {
                                if let Ok(cycles) = text.parse::<i64>() {
                                    metadata.insert(
                                        "editing_cycles".to_string(),
                                        MetadataValue::Number(cycles),
                                    );
                                }
                            }
                            "page-count" => {
                                if let Ok(count) = text.parse::<i64>() {
                                    metadata
                                        .insert("page_count".to_string(), MetadataValue::Number(count));
                                }
                            }
                            "word-count" => {
                                if let Ok(count) = text.parse::<i64>() {
                                    metadata
                                        .insert("word_count".to_string(), MetadataValue::Number(count));
                                }
                            }
                            "character-count" => {
                                if let Ok(count) = text.parse::<i64>() {
                                    metadata.insert(
                                        "character_count".to_string(),
                                        MetadataValue::Number(count),
                                    );
                                }
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
