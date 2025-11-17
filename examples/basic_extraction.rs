//! Basic extraction example
//!
//! This example demonstrates the simplest way to use Omniparse to extract
//! content and metadata from a file.
//!
//! Run with:
//! ```bash
//! cargo run --example basic_extraction
//! ```

use omniparse::{extract_from_path, Content};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // You can change this to any file you want to test
    let file_path = "test_data/text/sample.txt";
    
    println!("Extracting content from: {}", file_path);
    println!("{}", "=".repeat(60));
    
    // Extract content from the file
    let result = extract_from_path(file_path)?;
    
    // Display the detected MIME type
    println!("\n📄 MIME Type: {}", result.mime_type);
    println!("🎯 Detection Confidence: {:.2}%", result.detection_confidence * 100.0);
    
    // Display metadata
    println!("\n📋 Metadata:");
    let mut keys: Vec<_> = result.metadata.keys().collect();
    keys.sort();
    
    if keys.is_empty() {
        println!("  (no metadata available)");
    } else {
        for key in keys {
            if let Some(value) = result.metadata.get(key) {
                println!("  • {}: {:?}", key, value);
            }
        }
    }
    
    // Display common metadata fields using convenience methods
    if let Some(title) = result.metadata.title() {
        println!("\n📖 Title: {}", title);
    }
    if let Some(author) = result.metadata.author() {
        println!("✍️  Author: {}", author);
    }
    if let Some(created) = result.metadata.created() {
        println!("📅 Created: {}", created);
    }
    if let Some(modified) = result.metadata.modified() {
        println!("🔄 Modified: {}", modified);
    }
    
    // Display content
    println!("\n📝 Content:");
    match result.content {
        Content::Text(text) => {
            let preview = if text.len() > 500 {
                format!("{}... (truncated, {} total characters)", &text[..500], text.len())
            } else {
                text
            };
            println!("{}", preview);
        }
        Content::Binary(data) => {
            println!("  [Binary data: {} bytes]", data.len());
        }
        Content::None => {
            println!("  [No content extracted]");
        }
    }
    
    println!("\n{}", "=".repeat(60));
    println!("✅ Extraction complete!");
    
    Ok(())
}
