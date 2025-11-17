//! PDF document parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use lopdf::Document;

/// Parser for PDF documents
pub struct PdfParser;

impl Parser for PdfParser {
    fn supported_types(&self) -> &[&str] {
        &["application/pdf"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Load the PDF document
        let doc = Document::load_mem(data).map_err(|e| {
            Error::ParseError(format!("Failed to load PDF: {}", e))
        })?;

        // Extract text content
        let text = extract_text(&doc)?;

        // Extract metadata
        let metadata = extract_metadata(&doc)?;

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata,
            detection_confidence: 1.0,
        })
    }

    fn name(&self) -> &str {
        "PdfParser"
    }
}

/// Extract text content from PDF document
fn extract_text(doc: &Document) -> Result<String> {
    let mut text = String::new();
    let pages = doc.get_pages();

    for (page_num, _) in pages.iter() {
        match doc.extract_text(&[*page_num]) {
            Ok(page_text) => {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(&page_text);
            }
            Err(e) => {
                // Continue with other pages even if one fails
                eprintln!("Warning: Failed to extract text from page {}: {}", page_num, e);
            }
        }
    }

    Ok(text)
}

/// Extract metadata from PDF document
fn extract_metadata(doc: &Document) -> Result<Metadata> {
    let mut metadata = Metadata::new();

    // Get page count
    let page_count = doc.get_pages().len() as i64;
    metadata.insert("page_count".to_string(), MetadataValue::Number(page_count));

    // Try to extract document info dictionary
    if let Ok(info_dict) = doc.trailer.get(b"Info") {
        if let Ok(info_ref) = info_dict.as_reference() {
            if let Ok(info_obj) = doc.get_object(info_ref) {
                if let Ok(info_dict) = info_obj.as_dict() {
                    // Extract title
                    if let Ok(title) = info_dict.get(b"Title") {
                        if let Ok(title_str) = title.as_string() {
                            metadata.insert("title".to_string(), MetadataValue::Text(title_str.to_string()));
                        }
                    }

                    // Extract author
                    if let Ok(author) = info_dict.get(b"Author") {
                        if let Ok(author_str) = author.as_string() {
                            metadata.insert("author".to_string(), MetadataValue::Text(author_str.to_string()));
                        }
                    }

                    // Extract creation date
                    if let Ok(creation_date) = info_dict.get(b"CreationDate") {
                        if let Ok(date_str) = creation_date.as_string() {
                            // PDF dates are in format: D:YYYYMMDDHHmmSSOHH'mm'
                            // For simplicity, store as text for now
                            metadata.insert("creation_date".to_string(), MetadataValue::Text(date_str.to_string()));
                        }
                    }

                    // Extract subject
                    if let Ok(subject) = info_dict.get(b"Subject") {
                        if let Ok(subject_str) = subject.as_string() {
                            metadata.insert("subject".to_string(), MetadataValue::Text(subject_str.to_string()));
                        }
                    }

                    // Extract creator
                    if let Ok(creator) = info_dict.get(b"Creator") {
                        if let Ok(creator_str) = creator.as_string() {
                            metadata.insert("creator".to_string(), MetadataValue::Text(creator_str.to_string()));
                        }
                    }

                    // Extract producer
                    if let Ok(producer) = info_dict.get(b"Producer") {
                        if let Ok(producer_str) = producer.as_string() {
                            metadata.insert("producer".to_string(), MetadataValue::Text(producer_str.to_string()));
                        }
                    }
                }
            }
        }
    }

    Ok(metadata)
}
