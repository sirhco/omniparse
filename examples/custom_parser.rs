//! Custom parser example
//!
//! This example demonstrates how to create a custom parser for a file format
//! that is not built into Omniparse. This is useful when you need to support
//! proprietary or specialized file formats.
//!
//! Run with:
//! ```bash
//! cargo run --example custom_parser
//! ```

use omniparse::core::{Content, ExtractionResult, Metadata, MetadataValue, Result};
use omniparse::parsers::{Parser, ParserRegistry};
use std::io::Read;

/// A custom parser for a hypothetical ".myformat" file type
///
/// This parser demonstrates the basic structure needed to implement
/// the Parser trait for a custom file format.
struct MyFormatParser;

impl Parser for MyFormatParser {
    fn supported_types(&self) -> &[&str] {
        // Define the MIME types this parser handles
        &["application/x-myformat"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Convert bytes to string (assuming UTF-8 encoding)
        let content_str = String::from_utf8_lossy(data);
        
        // Parse the custom format
        // In this example, we assume the format is:
        // HEADER: <title>
        // AUTHOR: <author>
        // ---
        // <content>
        
        let mut metadata = Metadata::new();
        let mut content_lines = Vec::new();
        let mut in_header = true;
        
        for line in content_str.lines() {
            if line == "---" {
                in_header = false;
                continue;
            }
            
            if in_header {
                if let Some(title) = line.strip_prefix("HEADER: ") {
                    metadata.insert(
                        "title".to_string(),
                        MetadataValue::Text(title.to_string())
                    );
                } else if let Some(author) = line.strip_prefix("AUTHOR: ") {
                    metadata.insert(
                        "author".to_string(),
                        MetadataValue::Text(author.to_string())
                    );
                }
            } else {
                content_lines.push(line);
            }
        }
        
        let content = content_lines.join("\n");
        
        // Add some format-specific metadata
        metadata.insert(
            "line_count".to_string(),
            MetadataValue::Number(content_lines.len() as i64)
        );
        metadata.insert(
            "character_count".to_string(),
            MetadataValue::Number(content.len() as i64)
        );
        
        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(content),
            metadata,
            detection_confidence: 1.0,
        })
    }

    fn parse_stream(&self, reader: &mut dyn Read, mime_type: &str) -> Result<ExtractionResult> {
        // For this simple example, we'll just read everything into memory
        // In a real implementation, you might want to implement true streaming
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        self.parse(&buffer, mime_type)
    }

    fn name(&self) -> &str {
        "MyFormatParser"
    }
}

fn main() -> Result<()> {
    println!("Custom Parser Example");
    println!("{}", "=".repeat(60));
    
    // Create a sample file in our custom format
    let sample_data = r#"HEADER: My Custom Document
AUTHOR: John Doe
---
This is the content of my custom format file.
It can have multiple lines.
And the parser will extract both the metadata and content.
"#;
    
    println!("Sample data:");
    println!("{}", sample_data);
    println!("{}", "=".repeat(60));
    
    // Create a parser registry and register our custom parser
    let mut registry = ParserRegistry::new();
    registry.register(Box::new(MyFormatParser));
    
    println!("\n✅ Registered custom parser for: application/x-myformat");
    
    // Get the parser and use it
    if let Some(parser) = registry.get_parser("application/x-myformat") {
        println!("📦 Parser name: {}", parser.name());
        
        // Parse the sample data
        let result = parser.parse(sample_data.as_bytes(), "application/x-myformat")?;
        
        println!("\n📄 Extraction Results:");
        println!("{}", "-".repeat(60));
        println!("MIME Type: {}", result.mime_type);
        println!("Confidence: {:.0}%", result.detection_confidence * 100.0);
        
        println!("\n📋 Metadata:");
        for key in result.metadata.keys() {
            if let Some(value) = result.metadata.get(key) {
                println!("  • {}: {:?}", key, value);
            }
        }
        
        println!("\n📝 Content:");
        if let Content::Text(text) = result.content {
            println!("{}", text);
        }
    }
    
    println!("\n{}", "=".repeat(60));
    println!("💡 Tips for creating custom parsers:");
    println!("  1. Implement the Parser trait with your parsing logic");
    println!("  2. Register your parser with ParserRegistry");
    println!("  3. Optionally add magic byte patterns for auto-detection");
    println!("  4. Consider implementing streaming for large files");
    
    Ok(())
}
