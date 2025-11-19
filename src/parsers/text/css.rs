//! CSS parser for extracting content and metadata from CSS stylesheets

use crate::core::{Content, ExtractionResult, Metadata, Result};
use crate::parsers::Parser;
use crate::utils::security::{validate_file_size, FileSizeLimits};
use cssparser::{Parser as CssParserImpl, ParserInput, Token};

/// Parser for CSS stylesheets
pub struct CssParser;

impl CssParser {
    /// Count CSS rules and selectors in the stylesheet
    fn analyze_css(css_content: &str) -> (usize, usize, Vec<String>) {
        let mut rule_count = 0;
        let mut selector_count = 0;
        let mut imports = Vec::new();

        let mut input = ParserInput::new(css_content);
        let mut parser = CssParserImpl::new(&mut input);

        let mut in_selector = false;
        let mut brace_depth = 0;

        // Parse tokens to count rules, selectors, and extract @import statements
        loop {
            let token = match parser.next_including_whitespace_and_comments() {
                Ok(t) => t,
                Err(_) => break,
            };

            match token {
                Token::AtKeyword(keyword) => {
                    if keyword.eq_ignore_ascii_case("import") {
                        // Try to extract the import URL
                        if let Ok(import_url) = Self::extract_import_url(&mut parser) {
                            imports.push(import_url);
                        }
                    }
                }
                Token::CurlyBracketBlock => {
                    // Each curly bracket block typically represents a rule
                    if brace_depth == 0 {
                        rule_count += 1;
                        if in_selector {
                            selector_count += 1;
                            in_selector = false;
                        }
                    }
                    brace_depth += 1;
                }
                Token::Comma => {
                    // Commas in selector lists indicate multiple selectors
                    if brace_depth == 0 && in_selector {
                        selector_count += 1;
                    }
                }
                Token::Ident(_) | Token::Hash(_) | Token::Delim('.') => {
                    // These tokens indicate we're in a selector
                    if brace_depth == 0 {
                        in_selector = true;
                    }
                }
                _ => {}
            }
        }

        // If we ended with a selector that wasn't counted, count it
        if in_selector {
            selector_count += 1;
        }

        (rule_count, selector_count, imports)
    }

    /// Extract import URL from @import statement
    fn extract_import_url(parser: &mut CssParserImpl) -> std::result::Result<String, ()> {
        // Try to get the next meaningful token (skip whitespace)
        loop {
            match parser.next_including_whitespace_and_comments() {
                Ok(Token::WhiteSpace(_)) => continue,
                Ok(Token::QuotedString(s)) => return Ok(s.to_string()),
                Ok(Token::UnquotedUrl(s)) => return Ok(s.to_string()),
                Ok(Token::Function(name)) if name.eq_ignore_ascii_case("url") => {
                    // Parse url() function - look for string inside
                    loop {
                        match parser.next_including_whitespace_and_comments() {
                            Ok(Token::WhiteSpace(_)) => continue,
                            Ok(Token::QuotedString(s)) => return Ok(s.to_string()),
                            Ok(Token::UnquotedUrl(s)) => return Ok(s.to_string()),
                            _ => return Err(()),
                        }
                    }
                }
                _ => return Err(()),
            }
        }
    }
}

impl Parser for CssParser {
    fn name(&self) -> &str {
        "CssParser"
    }

    fn supported_types(&self) -> &[&str] {
        &["text/css"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Validate file size
        validate_file_size(data, FileSizeLimits::CSS, "CSS")?;
        
        // Extract raw CSS content as text
        let content = String::from_utf8_lossy(data).to_string();

        // Analyze CSS to count rules, selectors, and extract imports
        let (rule_count, selector_count, imports) = Self::analyze_css(&content);

        // Build metadata
        let mut metadata = Metadata::new();
        metadata.insert(
            "rule_count".to_string(),
            crate::core::MetadataValue::Number(rule_count as i64),
        );
        metadata.insert(
            "selector_count".to_string(),
            crate::core::MetadataValue::Number(selector_count as i64),
        );

        if !imports.is_empty() {
            metadata.insert(
                "imports".to_string(),
                crate::core::MetadataValue::Text(imports.join(", ")),
            );
        }

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(content),
            metadata,
            detection_confidence: 0.6,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::MetadataValue;

    #[test]
    fn test_css_parser_name() {
        let parser = CssParser;
        assert_eq!(parser.name(), "CssParser");
    }

    #[test]
    fn test_css_parser_supported_types() {
        let parser = CssParser;
        assert_eq!(parser.supported_types(), &["text/css"]);
    }

    #[test]
    fn test_css_parser_basic() {
        let parser = CssParser;
        let css_content = r#"
body {
    margin: 0;
    padding: 0;
}

h1 {
    color: blue;
}
"#;

        let result = parser.parse(css_content.as_bytes(), "text/css").unwrap();

        // Check that content is extracted
        match result.content {
            Content::Text(text) => {
                assert!(text.contains("body"));
                assert!(text.contains("margin"));
            }
            _ => panic!("Expected text content"),
        }

        // Check metadata
        assert!(result.metadata.get("rule_count").is_some());
        assert!(result.metadata.get("selector_count").is_some());
    }

    #[test]
    fn test_css_parser_with_imports() {
        let parser = CssParser;
        let css_content = r#"
@import url("base.css");
@import "theme.css";

body {
    margin: 0;
}
"#;

        let result = parser.parse(css_content.as_bytes(), "text/css").unwrap();

        // Check that imports are extracted
        if let Some(MetadataValue::Text(imports)) = result.metadata.get("imports") {
            assert!(imports.contains("base.css") || imports.contains("theme.css"));
        } else {
            panic!("Expected imports in metadata");
        }
    }

    #[test]
    fn test_css_parser_counts() {
        let parser = CssParser;
        let css_content = r#"
body { margin: 0; }
h1, h2, h3 { color: blue; }
.container { width: 100%; }
"#;

        let result = parser.parse(css_content.as_bytes(), "text/css").unwrap();

        // Check rule count
        if let Some(MetadataValue::Number(count)) = result.metadata.get("rule_count") {
            assert!(*count > 0, "Should have at least one rule");
        } else {
            panic!("Expected rule_count in metadata");
        }

        // Check selector count
        if let Some(MetadataValue::Number(count)) = result.metadata.get("selector_count") {
            assert!(*count > 0, "Should have at least one selector");
        } else {
            panic!("Expected selector_count in metadata");
        }
    }

    #[test]
    fn test_css_parser_empty() {
        let parser = CssParser;
        let css_content = "";

        let result = parser.parse(css_content.as_bytes(), "text/css").unwrap();

        // Should still return valid result with zero counts
        assert_eq!(result.mime_type, "text/css");
        match result.content {
            Content::Text(text) => assert_eq!(text, ""),
            _ => panic!("Expected text content"),
        }
    }
}
