//! CSS extraction example
//!
//! This example demonstrates how to extract and analyze CSS stylesheets.
//! The CSS parser counts rules and selectors, and extracts @import statements.
//!
//! Run with:
//! ```bash
//! cargo run --example css_extraction
//! ```

use omniparse::{extract_from_path, Content};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "test_data/text/sample.css";
    
    println!("🎨 CSS Extraction Example");
    println!("{}", "=".repeat(60));
    println!("Extracting from: {}\n", file_path);
    
    // Extract content from CSS file
    let result = extract_from_path(file_path)?;
    
    // Display detection info
    println!("📄 MIME Type: {}", result.mime_type);
    println!("🎯 Confidence: {:.2}%\n", result.detection_confidence * 100.0);
    
    // Display CSS-specific metadata
    println!("📋 CSS Analysis:");
    if let Some(rule_count) = result.metadata.get("rule_count") {
        println!("  • Rule Count: {:?}", rule_count);
    }
    if let Some(selector_count) = result.metadata.get("selector_count") {
        println!("  • Selector Count: {:?}", selector_count);
    }
    if let Some(imports) = result.metadata.get("imports") {
        println!("  • Imports: {:?}", imports);
    }
    if let Some(charset) = result.metadata.get("charset") {
        println!("  • Charset: {:?}", charset);
    }
    
    // Display raw CSS content
    println!("\n📝 CSS Content:");
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
    println!("✅ CSS extraction complete!");
    
    Ok(())
}
