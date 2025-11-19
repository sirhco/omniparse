//! Spreadsheet extraction example
//!
//! This example demonstrates how to extract data from spreadsheet files (XLSX, XLS, ODS).
//! The parsers extract cell values from all sheets and format them as CSV-like text.
//!
//! Run with:
//! ```bash
//! cargo run --example spreadsheet_extraction
//! ```

use omniparse::{extract_from_path, Content};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Spreadsheet Extraction Example");
    println!("{}", "=".repeat(60));
    
    // Test different spreadsheet formats
    let files = vec![
        "test_data/document/sample.xlsx",
        "test_data/document/sample.xls",
        "test_data/document/sample.ods",
    ];
    
    for file_path in files {
        println!("\n📁 Extracting from: {}", file_path);
        println!("{}", "-".repeat(60));
        
        match extract_from_path(file_path) {
            Ok(result) => {
                // Display detection info
                println!("📄 MIME Type: {}", result.mime_type);
                println!("🎯 Confidence: {:.2}%", result.detection_confidence * 100.0);
                
                // Display spreadsheet-specific metadata
                println!("\n📋 Spreadsheet Metadata:");
                if let Some(sheet_count) = result.metadata.get("sheet_count") {
                    println!("  • Sheet Count: {:?}", sheet_count);
                }
                if let Some(sheet_names) = result.metadata.get("sheet_names") {
                    println!("  • Sheet Names: {:?}", sheet_names);
                }
                if let Some(table_count) = result.metadata.get("table_count") {
                    println!("  • Table Count: {:?}", table_count);
                }
                if let Some(table_names) = result.metadata.get("table_names") {
                    println!("  • Table Names: {:?}", table_names);
                }
                if let Some(total_rows) = result.metadata.get("total_rows") {
                    println!("  • Total Rows: {:?}", total_rows);
                }
                if let Some(author) = result.metadata.get("author") {
                    println!("  • Author: {:?}", author);
                }
                if let Some(title) = result.metadata.get("title") {
                    println!("  • Title: {:?}", title);
                }
                
                // Display extracted data
                println!("\n📝 Extracted Data (CSV format):");
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
            }
            Err(e) => {
                eprintln!("❌ Error: {}", e);
            }
        }
    }
    
    println!("\n{}", "=".repeat(60));
    println!("✅ Spreadsheet extraction complete!");
    println!("\nNote: Cell values are extracted (not formulas) and formatted as CSV.");
    
    Ok(())
}
