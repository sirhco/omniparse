//! ODP (OpenDocument Presentation) parser implementation

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::utils::security::{validate_file_size, validate_zip_structure, check_xml_bomb, FileSizeLimits};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Parser for OpenDocument Presentation (ODP) files
pub struct OdpParser;

impl Parser for OdpParser {
    fn name(&self) -> &str {
        "OdpParser"
    }

    fn supported_types(&self) -> &[&str] {
        &[
            "application/vnd.oasis.opendocument.presentation",
            "application/odp",
        ]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Validate file size
        validate_file_size(data, FileSizeLimits::ODP, "ODP")?;
        
        // Validate ZIP structure and check for ZIP bombs
        validate_zip_structure(data, Some(&["content.xml", "meta.xml"]))?;
        
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor).map_err(|e| {
            Error::ParseError(format!("Failed to open ODP archive: {}", e))
        })?;

        // Extract text content from all slides
        let (text, slide_count) = extract_slides(&mut archive)?;

        // Extract metadata from meta.xml
        let mut metadata = extract_metadata(&mut archive)?;

        // Add slide count to metadata
        metadata.insert(
            "slide_count".to_string(),
            MetadataValue::Number(slide_count as i64),
        );

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata,
            detection_confidence: 0.95,
        })
    }
}

/// Extract slides from content.xml
fn extract_slides(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<(String, usize)> {
    let mut content_file = archive
        .by_name("content.xml")
        .map_err(|e| Error::ParseError(format!("Failed to find content.xml: {}", e)))?;

    let mut xml_content = String::new();
    content_file
        .read_to_string(&mut xml_content)
        .map_err(|e| Error::ParseError(format!("Failed to read content.xml: {}", e)))?;

    // Check for XML bombs
    check_xml_bomb(&xml_content)?;

    parse_slides(&xml_content)
}

/// Parse slides from content.xml
fn parse_slides(xml_content: &str) -> Result<(String, usize)> {
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut all_text = String::new();
    let mut slide_count = 0;
    let mut buf = Vec::new();

    let mut in_page = false;
    let mut in_text_element = false;
    let mut current_slide_text = String::new();
    let mut slide_name = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = e.name();
                let local_name_bytes = name.local_name();
                let local_name = local_name_bytes.as_ref();

                match local_name {
                    b"page" => {
                        in_page = true;
                        slide_count += 1;
                        current_slide_text.clear();
                        
                        // Extract slide name from draw:name attribute
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                if attr.key.local_name().as_ref() == b"name" {
                                    slide_name = String::from_utf8_lossy(&attr.value).to_string();
                                    break;
                                }
                            }
                        }
                    }
                    b"p" if in_page => {
                        // Paragraph element - we'll collect text from it
                        in_text_element = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if in_text_element => {
                let content = e
                    .unescape()
                    .map_err(|e| Error::ParseError(format!("Failed to unescape text: {}", e)))?;
                let text = content.trim();
                if !text.is_empty() {
                    if !current_slide_text.is_empty() && !current_slide_text.ends_with('\n') {
                        current_slide_text.push(' ');
                    }
                    current_slide_text.push_str(text);
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                let local_name_bytes = name.local_name();
                let local_name = local_name_bytes.as_ref();

                match local_name {
                    b"p" => {
                        in_text_element = false;
                        if !current_slide_text.is_empty() && !current_slide_text.ends_with('\n') {
                            current_slide_text.push('\n');
                        }
                    }
                    b"page" => {
                        in_page = false;
                        
                        // Add slide separator and content
                        if !all_text.is_empty() {
                            all_text.push_str("\n\n");
                        }
                        all_text.push_str(&format!("--- Slide {} ---\n", slide_count));
                        all_text.push_str(&current_slide_text.trim());
                        
                        slide_name.clear();
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

    Ok((all_text, slide_count))
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
        let parser = OdpParser;
        let types = parser.supported_types();
        assert!(types.contains(&"application/vnd.oasis.opendocument.presentation"));
        assert!(types.contains(&"application/odp"));
    }

    #[test]
    fn test_parser_name() {
        let parser = OdpParser;
        assert_eq!(parser.name(), "OdpParser");
    }
}
