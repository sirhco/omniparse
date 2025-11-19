//! PPTX (PowerPoint) parser implementation

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::utils::security::{validate_file_size, validate_zip_structure, check_xml_bomb, FileSizeLimits};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Parser for Microsoft PowerPoint PPTX files
pub struct PptxParser;

impl Parser for PptxParser {
    fn name(&self) -> &str {
        "PptxParser"
    }

    fn supported_types(&self) -> &[&str] {
        &[
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "application/pptx",
        ]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Validate file size
        validate_file_size(data, FileSizeLimits::PPTX, "PPTX")?;
        
        // Validate ZIP structure and check for ZIP bombs
        validate_zip_structure(data, Some(&["[Content_Types].xml"]))?;
        
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor).map_err(|e| {
            Error::ParseError(format!("Failed to open PPTX archive: {}", e))
        })?;

        // Check if speaker notes exist
        let has_notes = check_for_notes(&mut archive);

        // Extract text content from all slides
        let (text, slide_count) = extract_slides(&mut archive)?;

        // Extract metadata from core.xml
        let mut metadata = extract_metadata(&mut archive)?;
        
        // Add slide count to metadata
        metadata.insert("slide_count".to_string(), MetadataValue::Number(slide_count as i64));
        
        // Add has_notes flag to metadata
        metadata.insert("has_notes".to_string(), MetadataValue::Boolean(has_notes));

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata,
            detection_confidence: 1.0,
        })
    }
}

/// Check if the presentation has any speaker notes
fn check_for_notes(archive: &mut ZipArchive<Cursor<&[u8]>>) -> bool {
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name();
            if name.starts_with("ppt/notesSlides/notesSlide") && name.ends_with(".xml") {
                return true;
            }
        }
    }
    false
}

/// Extract text content from all slides
fn extract_slides(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<(String, usize)> {
    // Pre-allocate string with estimated capacity for better performance
    let mut all_text = String::with_capacity(4096);
    let mut slide_count = 0;

    // Collect slide names first to avoid multiple archive iterations
    let mut slide_names = Vec::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| {
            Error::ParseError(format!("Failed to access archive entry: {}", e))
        })?;
        
        let name = file.name();
        
        // Check if this is a slide file
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") && !name.contains("_rels") {
            slide_names.push(name.to_string());
        }
    }
    
    // Sort slide names to ensure correct order
    slide_names.sort();
    
    // Extract text from each slide
    for name in slide_names {
        slide_count += 1;
        
        let slide_text = extract_slide_text(archive, &name)?;
        
        // Try to extract speaker notes for this slide
        let notes_path = format!("ppt/notesSlides/notesSlide{}.xml", slide_count);
        let notes_text = extract_notes_text(archive, &notes_path).unwrap_or_default();
        
        if !all_text.is_empty() {
            all_text.push_str("\n\n");
        }
        all_text.push_str(&format!("--- Slide {} ---\n", slide_count));
        all_text.push_str(&slide_text);
        
        if !notes_text.is_empty() {
            all_text.push_str("\n\nSpeaker Notes:\n");
            all_text.push_str(&notes_text);
        }
    }

    Ok((all_text, slide_count))
}

/// Extract speaker notes from a notes slide XML file
fn extract_notes_text(archive: &mut ZipArchive<Cursor<&[u8]>>, notes_path: &str) -> Result<String> {
    // Try to open the notes file - it may not exist
    let mut notes_file = match archive.by_name(notes_path) {
        Ok(file) => file,
        Err(_) => return Ok(String::new()), // No notes for this slide
    };

    let mut xml_content = String::new();
    notes_file
        .read_to_string(&mut xml_content)
        .map_err(|e| Error::ParseError(format!("Failed to read {}: {}", notes_path, e)))?;

    // Parse XML and extract text from <a:t> elements
    let mut reader = Reader::from_str(&xml_content);
    reader.trim_text(true);

    let mut text = String::new();
    let mut buf = Vec::new();
    let mut in_text_element = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = e.name();
                if name.local_name().as_ref() == b"t" {
                    in_text_element = true;
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    let content = e
                        .unescape()
                        .map_err(|e| Error::ParseError(format!("Failed to unescape text: {}", e)))?;
                    text.push_str(&content);
                    text.push(' ');
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                if name.local_name().as_ref() == b"t" {
                    in_text_element = false;
                } else if name.local_name().as_ref() == b"p" {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(Error::ParseError(format!(
                    "Error parsing {}: {}",
                    notes_path, e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(text.trim().to_string())
}

/// Extract text from a single slide XML file
fn extract_slide_text(archive: &mut ZipArchive<Cursor<&[u8]>>, slide_path: &str) -> Result<String> {
    let mut slide_file = archive
        .by_name(slide_path)
        .map_err(|e| Error::ParseError(format!("Failed to find {}: {}", slide_path, e)))?;

    let mut xml_content = String::new();
    slide_file
        .read_to_string(&mut xml_content)
        .map_err(|e| Error::ParseError(format!("Failed to read {}: {}", slide_path, e)))?;

    // Check for XML bombs
    check_xml_bomb(&xml_content)?;

    // Parse XML and extract text from <a:t> elements
    let mut reader = Reader::from_str(&xml_content);
    reader.trim_text(true);

    let mut text = String::new();
    let mut buf = Vec::new();
    let mut in_text_element = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = e.name();
                // Check for <a:t> elements (text runs in DrawingML)
                if name.local_name().as_ref() == b"t" {
                    in_text_element = true;
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    let content = e
                        .unescape()
                        .map_err(|e| Error::ParseError(format!("Failed to unescape text: {}", e)))?;
                    text.push_str(&content);
                    text.push(' ');
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                if name.local_name().as_ref() == b"t" {
                    in_text_element = false;
                } else if name.local_name().as_ref() == b"p" {
                    // End of paragraph, add newline
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(Error::ParseError(format!(
                    "Error parsing {}: {}",
                    slide_path, e
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_types() {
        let parser = PptxParser;
        let types = parser.supported_types();
        assert!(types.contains(&"application/vnd.openxmlformats-officedocument.presentationml.presentation"));
    }

    #[test]
    fn test_parser_name() {
        let parser = PptxParser;
        assert_eq!(parser.name(), "PptxParser");
    }
}
