//! Presentation extraction example
//!
//! This example demonstrates how to extract text from presentation files (PPTX, PPT, ODP).
//! The parsers extract text from all slides and include speaker notes where available.
//!
//! Run with:
//! ```bash
//! cargo run --example presentation_extraction
//! ```

use omniparse::{extract_from_path, Content};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📽️  Presentation Extraction Example");
    println!("{}", "=".repeat(60));
    
    // Test different presentation formats
    let files = vec![
        "test_data/document/sample.pptx",
        "test_data/document/sample.ppt",
        "test_data/document/sample.odp",
    ];
    
    for file_path in files {
        println!("\n📁 Extracting from: {}", file_path);
        println!("{}", "-".repeat(60));
        
        match extract_from_path(file_path) {
            Ok(result) => {
                // Display detection info
                println!("📄 MIME Type: {}", result.mime_type);
                println!("🎯 Confidence: {:.2}%", result.detection_confidence * 100.0);
                
                // Display presentation-specific metadata
                println!("\n📋 Presentation Metadata:");
                if let Some(slide_count) = result.metadata.get("slide_count") {
                    println!("  • Slide Count: {:?}", slide_count);
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
                if let Some(has_notes) = result.metadata.get("has_notes") {
                    println!("  • Has Speaker Notes: {:?}", has_notes);
                }
                if let Some(creation_date) = result.metadata.get("creation_date") {
                    println!("  • Creation Date: {:?}", creation_date);
                }
                
                // Display extracted slide content
                println!("\n📝 Extracted Slide Content:");
                match result.content {
                    Content::Text(text) => {
                        let preview = if text.len() > 800 {
                            format!("{}...\n(truncated, {} total characters)", &text[..800], text.len())
                        } else {
                            text
                        };
                        println!("{}", preview);
                    }
                    _ => println!("  [No text content]"),
                }
            }
            Err(e) => {
                eprintln!("❌ Error: {}", e);
            }
        }
    }
    
    println!("\n{}", "=".repeat(60));
    println!("✅ Presentation extraction complete!");
    println!("\nNote: Slides are separated with clear boundaries, and speaker notes are included.");
    
    Ok(())
}
