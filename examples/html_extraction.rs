//! HTML extraction example
//!
//! This example demonstrates how to extract text content and metadata from HTML files.
//! The HTML parser extracts visible text while excluding scripts and styles, and
//! captures metadata from meta tags.
//!
//! Run with:
//! ```bash
//! cargo run --example html_extraction
//! ```

use omniparse::{extract_from_path, Content};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "test_data/text/sample.html";
    
    println!("🌐 HTML Extraction Example");
    println!("{}", "=".repeat(60));
    println!("Extracting from: {}\n", file_path);
    
    // Extract content from HTML file
    let result = extract_from_path(file_path)?;
    
    // Display detection info
    println!("📄 MIME Type: {}", result.mime_type);
    println!("🎯 Confidence: {:.2}%\n", result.detection_confidence * 100.0);
    
    // Display HTML-specific metadata
    println!("📋 HTML Metadata:");
    if let Some(title) = result.metadata.get("title") {
        println!("  • Title: {:?}", title);
    }
    if let Some(description) = result.metadata.get("description") {
        println!("  • Description: {:?}", description);
    }
    if let Some(author) = result.metadata.get("author") {
        println!("  • Author: {:?}", author);
    }
    if let Some(keywords) = result.metadata.get("keywords") {
        println!("  • Keywords: {:?}", keywords);
    }
    if let Some(charset) = result.metadata.get("charset") {
        println!("  • Charset: {:?}", charset);
    }
    if let Some(language) = result.metadata.get("language") {
        println!("  • Language: {:?}", language);
    }
    
    // Display extracted text content
    println!("\n📝 Extracted Text Content:");
    match result.content {
        Content::Text(text) => {
            let preview = if text.len() > 500 {
                format!("{}...\n(truncated, {} total characters)", &text[..500], text.len())
            } else {
                text
            };
            println!("{}", preview);
        }
        _ => println!("  [No text content]"),
    }
    
    println!("\n{}", "=".repeat(60));
    println!("✅ HTML extraction complete!");
    println!("\nNote: Scripts and styles are automatically excluded from the extracted text.");
    
    Ok(())
}
