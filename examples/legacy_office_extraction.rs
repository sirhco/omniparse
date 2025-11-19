//! Legacy Office format extraction example
//!
//! This example demonstrates how to extract content from legacy Microsoft Office files
//! (DOC, XLS, PPT). These binary formats use OLE2 structure and have more limited
//! extraction capabilities compared to modern formats.
//!
//! Run with:
//! ```bash
//! cargo run --example legacy_office_extraction
//! ```

use omniparse::{extract_from_path, Content};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📁 Legacy Office Format Extraction Example");
    println!("{}", "=".repeat(60));
    
    // Test different legacy Office formats
    let files = vec![
        ("test_data/document/sample.doc", "Word Document"),
        ("test_data/document/sample.xls", "Excel Spreadsheet"),
        ("test_data/document/sample.ppt", "PowerPoint Presentation"),
    ];
    
    for (file_path, format_name) in files {
        println!("\n📄 Extracting from: {} ({})", file_path, format_name);
        println!("{}", "-".repeat(60));
        
        match extract_from_path(file_path) {
            Ok(result) => {
                // Display detection info
                println!("📄 MIME Type: {}", result.mime_type);
                println!("🎯 Confidence: {:.2}%", result.detection_confidence * 100.0);
                
                // Display metadata
                println!("\n📋 Document Metadata:");
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
                
                // Display extracted content
                println!("\n📝 Extracted Content:");
                match result.content {
                    Content::Text(text) => {
                        if text.is_empty() {
                            println!("  (no text content extracted)");
                        } else {
                            let preview = if text.len() > 500 {
                                format!("{}...\n(truncated, {} total characters)", &text[..500], text.len())
                            } else {
                                text
                            };
                            println!("{}", preview);
                        }
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
    println!("✅ Legacy Office extraction complete!");
    println!("\nNote: Legacy formats have limited extraction capabilities compared to modern formats.");
    println!("For best results, use DOCX, XLSX, and PPTX formats instead.");
    
    Ok(())
}
