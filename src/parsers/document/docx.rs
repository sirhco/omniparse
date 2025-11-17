//! DOCX (Microsoft Word) document parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Parser for DOCX documents
pub struct DocxParser;

impl Parser for DocxParser {
    fn supported_types(&self) -> &[&str] {
        &[
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/docx",
        ]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor).map_err(|e| {
            Error::ParseError(format!("Failed to open DOCX archive: {}", e))
        })?;

        // Extract text content from document.xml
        let text = extract_text(&mut archive)?;

        // Extract metadata from core.xml
        let metadata = extract_metadata(&mut archive)?;

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata,
            detection_confidence: 1.0,
        })
    }

    fn name(&self) -> &str {
        "DocxParser"
    }
}

/// Extract text content from document.xml
fn extract_text(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<String> {
    // Try to find and read document.xml
    let mut document_file = archive
        .by_name("word/document.xml")
        .map_err(|e| Error::ParseError(format!("Failed to find document.xml: {}", e)))?;

    let mut xml_content = String::new();
    document_file
        .read_to_string(&mut xml_content)
        .map_err(|e| Error::ParseError(format!("Failed to read document.xml: {}", e)))?;

    // Parse XML and extract text from <w:t> elements
    let mut reader = Reader::from_str(&xml_content);
    reader.trim_text(true);

    let mut text = String::new();
    let mut buf = Vec::new();
    let mut in_text_element = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                // Check if this is a text element (w:t)
                let name = e.name();
                if name.as_ref() == b"w:t" {
                    in_text_element = true;
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    let content = e
                        .unescape()
                        .map_err(|e| Error::ParseError(format!("Failed to unescape text: {}", e)))?;
                    text.push_str(&content);
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                if name.as_ref() == b"w:t" {
                    in_text_element = false;
                } else if name.as_ref() == b"w:p" {
                    // End of paragraph, add newline
                    text.push('\n');
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(Error::ParseError(format!(
                    "Error parsing document.xml: {}",
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(text.trim().to_string())
}

/// Extract metadata from core.xml
fn extract_metadata(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<Metadata> {
    let mut metadata = Metadata::new();

    // Try to read core.xml (docProps/core.xml)
    let core_result = archive.by_name("docProps/core.xml");

    if let Ok(mut core_file) = core_result {
        let mut xml_content = String::new();
        if core_file.read_to_string(&mut xml_content).is_ok() {
            parse_core_properties(&xml_content, &mut metadata)?;
        }
    }

    Ok(metadata)
}

/// Parse core properties XML
fn parse_core_properties(xml_content: &str, metadata: &mut Metadata) -> Result<()> {
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
                            "creator" => {
                                metadata.insert("author".to_string(), MetadataValue::Text(text));
                            }
                            "subject" => {
                                metadata.insert("subject".to_string(), MetadataValue::Text(text));
                            }
                            "description" => {
                                metadata
                                    .insert("description".to_string(), MetadataValue::Text(text));
                            }
                            "created" => {
                                metadata
                                    .insert("creation_date".to_string(), MetadataValue::Text(text));
                            }
                            "modified" => {
                                metadata
                                    .insert("modified_date".to_string(), MetadataValue::Text(text));
                            }
                            "lastModifiedBy" => {
                                metadata.insert(
                                    "last_modified_by".to_string(),
                                    MetadataValue::Text(text),
                                );
                            }
                            "revision" => {
                                if let Ok(rev) = text.parse::<i64>() {
                                    metadata
                                        .insert("revision".to_string(), MetadataValue::Number(rev));
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
                return Err(Error::ParseError(format!("Error parsing core.xml: {}", e)))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}
