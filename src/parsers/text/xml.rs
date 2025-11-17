//! XML parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashSet;

/// Parser for XML files
pub struct XmlParser;

impl XmlParser {
    /// Extract text content and metadata from XML
    fn parse_xml(data: &[u8]) -> Result<(String, Option<String>, Vec<String>)> {
        let mut reader = Reader::from_reader(data);
        reader.trim_text(true);
        
        let mut text_content = Vec::new();
        let mut root_element: Option<String> = None;
        let mut namespaces = HashSet::new();
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    // Capture root element
                    if root_element.is_none() {
                        let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                        root_element = Some(name);
                    }
                    
                    // Extract namespaces from attributes
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            if key.starts_with("xmlns") {
                                let value = String::from_utf8_lossy(&attr.value).to_string();
                                namespaces.insert(value);
                            }
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape()
                        .map_err(|e| Error::ParseError(format!("XML text unescape error: {}", e)))?;
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        text_content.push(trimmed.to_string());
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(Error::ParseError(format!("XML parsing error at position {}: {}", 
                        reader.buffer_position(), e)));
                }
                _ => {}
            }
            buf.clear();
        }
        
        let text = text_content.join(" ");
        let namespaces_vec: Vec<String> = namespaces.into_iter().collect();
        
        Ok((text, root_element, namespaces_vec))
    }
}

impl Parser for XmlParser {
    fn supported_types(&self) -> &[&str] {
        &["application/xml", "text/xml"]
    }
    
    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Parse XML and extract content
        let (text, root_element, namespaces) = Self::parse_xml(data)?;
        
        // Build metadata
        let mut metadata = Metadata::new();
        
        if let Some(root) = root_element {
            metadata.insert("root_element".to_string(), MetadataValue::Text(root));
        }
        
        if !namespaces.is_empty() {
            metadata.insert("namespaces".to_string(), MetadataValue::List(
                namespaces.into_iter().map(MetadataValue::Text).collect()
            ));
        }
        
        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata,
            detection_confidence: 0.0, // Will be set by the extractor
        })
    }
    
    fn name(&self) -> &str {
        "XmlParser"
    }
}
