//! RTF extraction example
//!
//! This example demonstrates how to extract plain text from Rich Text Format (RTF) files.
//! The RTF parser strips control words and extracts metadata from the \info group.
//!
//! Run with:
//! ```bash
//! cargo run --example rtf_extraction
//! ```

use omniparse::{extract_from_path, Content};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "test_data/text/sample.rtf";
    
    println!("📄 RTF Extraction Example");
    println!("{}", "=".repeat(60));
    println!("Extracting from: {}\n", file_path);
    
    // Extract content from RTF file
    let result = extract_from_path(file_path)?;
    
    // Display detection info
    println!("📄 MIME Type: {}", result.mime_type);
    println!("🎯 Confidence: {:.2}%\n", result.detection_confidence * 100.0);
    
    // Display RTF-specific metadata
    println!("📋 RTF Metadata:");
    if let Some(rtf_version) = result.metadata.get("rtf_version") {
        println!("  • RTF Version: {:?}", rtf_version);
    }
    if let Some(title) = result.metadata.get("title") {
        println!("  • Title: {:?}", title);
    }
    if let Some(author) = result.metadata.get("author") {
        println!("  • Author: {:?}", author);
    }
    if let Some(subject) = result.metadata.get("subject") {
        println!("  • Subject: {:?}", subject);
    }
    if let Some(creation_date) = result.metadata.get("creation_date") {
        println!("  • Creation Date: {:?}", creation_date);
    }
    
    // Display extracted plain text
    println!("\n📝 Extracted Plain Text:");
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
    println!("✅ RTF extraction complete!");
    println!("\nNote: All RTF control words have been stripped to produce plain text.");
    
    Ok(())
}
