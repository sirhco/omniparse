//! HTML parser for extracting text content and metadata from HTML documents

use crate::core::{Content, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::utils::security::{validate_file_size, FileSizeLimits};
use scraper::{Html, Selector};

/// Parser for HTML documents
pub struct HtmlParser;

impl HtmlParser {
    /// Extract visible text from HTML, excluding script and style tags
    fn extract_text(document: &Html) -> String {
        let body_selector = Selector::parse("body").unwrap();
        
        // Get the body element, or use the whole document if no body
        let body_element = document.select(&body_selector).next();
        
        // Pre-allocate vector with estimated capacity
        let mut text_parts = Vec::with_capacity(64);
        
        if let Some(body) = body_element {
            // Recursively extract text, skipping script and style elements
            Self::extract_text_from_element(body, &mut text_parts);
        } else {
            // No body tag, try to extract from root
            let root_selector = Selector::parse("*").unwrap();
            for element in document.select(&root_selector) {
                let tag_name = element.value().name();
                if tag_name != "script" && tag_name != "style" && tag_name != "head" {
                    for text in element.text() {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            text_parts.push(trimmed.to_string());
                        }
                    }
                }
            }
        }
        
        // Join with newlines to preserve paragraph boundaries
        text_parts.join("\n")
    }
    
    /// Recursively extract text from an element, excluding script and style tags
    fn extract_text_from_element(element: scraper::ElementRef, text_parts: &mut Vec<String>) {
        let tag_name = element.value().name();
        
        // Skip script and style tags entirely
        if tag_name == "script" || tag_name == "style" {
            return;
        }
        
        // For other elements, extract direct text nodes
        for child in element.children() {
            if let Some(text_node) = child.value().as_text() {
                let text = text_node.trim();
                if !text.is_empty() {
                    text_parts.push(text.to_string());
                }
            } else if let Some(child_element) = scraper::ElementRef::wrap(child) {
                // Recursively process child elements
                Self::extract_text_from_element(child_element, text_parts);
            }
        }
    }
    
    /// Extract metadata from HTML document
    fn extract_metadata(document: &Html) -> Metadata {
        let mut metadata = Metadata::new();
        
        // Extract title
        if let Ok(title_selector) = Selector::parse("title") {
            if let Some(title_element) = document.select(&title_selector).next() {
                let title = title_element.text().collect::<String>().trim().to_string();
                if !title.is_empty() {
                    metadata.insert("title".to_string(), MetadataValue::Text(title));
                }
            }
        }
        
        // Extract meta tags
        if let Ok(meta_selector) = Selector::parse("meta") {
            for meta in document.select(&meta_selector) {
                let element = meta.value();
                
                // Extract description
                if let Some(name) = element.attr("name") {
                    if name.eq_ignore_ascii_case("description") {
                        if let Some(content) = element.attr("content") {
                            metadata.insert("description".to_string(), MetadataValue::Text(content.to_string()));
                        }
                    } else if name.eq_ignore_ascii_case("author") {
                        if let Some(content) = element.attr("content") {
                            metadata.insert("author".to_string(), MetadataValue::Text(content.to_string()));
                        }
                    } else if name.eq_ignore_ascii_case("keywords") {
                        if let Some(content) = element.attr("content") {
                            metadata.insert("keywords".to_string(), MetadataValue::Text(content.to_string()));
                        }
                    }
                }
                
                // Extract charset
                if let Some(charset) = element.attr("charset") {
                    metadata.insert("charset".to_string(), MetadataValue::Text(charset.to_string()));
                }
            }
        }
        
        // Extract language from html tag
        if let Ok(html_selector) = Selector::parse("html") {
            if let Some(html_element) = document.select(&html_selector).next() {
                if let Some(lang) = html_element.value().attr("lang") {
                    metadata.insert("language".to_string(), MetadataValue::Text(lang.to_string()));
                }
            }
        }
        
        metadata
    }
}

impl Parser for HtmlParser {
    fn name(&self) -> &str {
        "HtmlParser"
    }

    fn supported_types(&self) -> &[&str] {
        &["text/html"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Validate file size
        validate_file_size(data, FileSizeLimits::HTML, "HTML")?;
        
        // Convert bytes to string
        let html_string = String::from_utf8_lossy(data).to_string();
        
        // Parse HTML document
        let document = Html::parse_document(&html_string);
        
        // Extract text content
        let text = Self::extract_text(&document);
        
        // Extract metadata
        let metadata = Self::extract_metadata(&document);
        
        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata,
            detection_confidence: 0.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_parser_name() {
        let parser = HtmlParser;
        assert_eq!(parser.name(), "HtmlParser");
    }

    #[test]
    fn test_html_parser_supported_types() {
        let parser = HtmlParser;
        assert_eq!(parser.supported_types(), &["text/html"]);
    }

    #[test]
    fn test_valid_html_extraction() {
        let parser = HtmlParser;
        let html = r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <title>Test Page</title>
                <meta name="description" content="A test page">
                <meta name="author" content="Test Author">
                <meta name="keywords" content="test, html, parser">
                <meta charset="UTF-8">
            </head>
            <body>
                <h1>Welcome</h1>
                <p>This is a test paragraph.</p>
                <p>Another paragraph here.</p>
            </body>
            </html>
        "#;
        
        let result = parser.parse(html.as_bytes(), "text/html").unwrap();
        
        // Check content
        if let Content::Text(text) = result.content {
            assert!(text.contains("Welcome"));
            assert!(text.contains("This is a test paragraph"));
            assert!(text.contains("Another paragraph here"));
        } else {
            panic!("Expected text content");
        }
        
        // Check metadata
        assert_eq!(result.metadata.title(), Some("Test Page"));
        assert_eq!(result.metadata.get("description"), Some(&MetadataValue::Text("A test page".to_string())));
        assert_eq!(result.metadata.get("author"), Some(&MetadataValue::Text("Test Author".to_string())));
        assert_eq!(result.metadata.get("keywords"), Some(&MetadataValue::Text("test, html, parser".to_string())));
        assert_eq!(result.metadata.get("charset"), Some(&MetadataValue::Text("UTF-8".to_string())));
        assert_eq!(result.metadata.get("language"), Some(&MetadataValue::Text("en".to_string())));
    }

    #[test]
    fn test_html_with_scripts_and_styles() {
        let parser = HtmlParser;
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Script Test</title>
                <style>
                    body { color: red; }
                </style>
                <script>
                    console.log("This should not appear");
                </script>
            </head>
            <body>
                <p>Visible content</p>
                <script>alert("Hidden");</script>
                <style>.hidden { display: none; }</style>
            </body>
            </html>
        "#;
        
        let result = parser.parse(html.as_bytes(), "text/html").unwrap();
        
        if let Content::Text(text) = result.content {
            assert!(text.contains("Visible content"));
            assert!(!text.contains("console.log"));
            assert!(!text.contains("alert"));
            assert!(!text.contains("color: red"));
            assert!(!text.contains("display: none"));
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_malformed_html() {
        let parser = HtmlParser;
        let html = r#"
            <html>
            <head>
                <title>Malformed</title>
            </head>
            <body>
                <p>Missing closing tags
                <div>Content here</div>
            </body>
            </html>
        "#;
        
        // Should handle malformed HTML gracefully
        let result = parser.parse(html.as_bytes(), "text/html");
        assert!(result.is_ok());
        
        if let Ok(extraction) = result {
            if let Content::Text(text) = extraction.content {
                assert!(text.contains("Content here"));
                assert!(text.contains("Missing closing tags"));
            }
        }
    }

    #[test]
    fn test_html_without_metadata() {
        let parser = HtmlParser;
        let html = r#"
            <html>
            <body>
                <p>Simple content without metadata</p>
            </body>
            </html>
        "#;
        
        let result = parser.parse(html.as_bytes(), "text/html").unwrap();
        
        if let Content::Text(text) = result.content {
            assert!(text.contains("Simple content without metadata"));
        } else {
            panic!("Expected text content");
        }
        
        // Should have no title
        assert_eq!(result.metadata.title(), None);
    }

    #[test]
    fn test_empty_html() {
        let parser = HtmlParser;
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head><title>Empty</title></head>
            <body></body>
            </html>
        "#;
        
        let result = parser.parse(html.as_bytes(), "text/html").unwrap();
        assert_eq!(result.metadata.title(), Some("Empty"));
    }
}
