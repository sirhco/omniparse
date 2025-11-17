//! Async extraction example
//!
//! This example demonstrates how to use Omniparse's async API for non-blocking
//! file extraction. This is useful when integrating with async web servers or
//! other async applications.
//!
//! Run with:
//! ```bash
//! cargo run --example async_extraction --features async
//! ```
//!
//! Note: This example requires the 'async' feature to be enabled.

#[cfg(feature = "async")]
use omniparse::extract_from_path_async;

#[cfg(feature = "async")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Async Extraction Example");
    println!("{}", "=".repeat(60));
    
    // List of files to process asynchronously
    let files = vec![
        "test_data/text/sample.txt",
        "test_data/text/sample.json",
        "test_data/document/sample.pdf",
    ];
    
    println!("Processing {} files asynchronously...\n", files.len());
    
    // Process files concurrently using tokio::join!
    let results = futures::future::join_all(
        files.iter().map(|file| async move {
            let path = *file;
            println!("🔄 Starting extraction: {}", path);
            
            match extract_from_path_async(path).await {
                Ok(result) => {
                    println!("  ✅ Completed: {} → {}", path, result.mime_type);
                    Ok((path, result))
                }
                Err(e) => {
                    println!("  ❌ Failed: {} → {}", path, e);
                    Err((path, e))
                }
            }
        })
    ).await;
    
    // Display summary
    println!("\n{}", "=".repeat(60));
    println!("📊 Results Summary:\n");
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    for result in results {
        match result {
            Ok((path, extraction)) => {
                success_count += 1;
                println!("✅ {}", path);
                println!("   MIME Type: {}", extraction.mime_type);
                println!("   Confidence: {:.0}%", extraction.detection_confidence * 100.0);
                
                // Show a preview of the content
                match extraction.content {
                    omniparse::Content::Text(text) => {
                        let preview = if text.len() > 100 {
                            format!("{}...", &text[..100])
                        } else {
                            text
                        };
                        println!("   Preview: {}", preview.replace('\n', " "));
                    }
                    omniparse::Content::Binary(data) => {
                        println!("   Binary data: {} bytes", data.len());
                    }
                    omniparse::Content::None => {
                        println!("   No content extracted");
                    }
                }
                println!();
            }
            Err((path, e)) => {
                error_count += 1;
                println!("❌ {}: {}\n", path, e);
            }
        }
    }
    
    println!("{}", "=".repeat(60));
    println!("Total: {} succeeded, {} failed", success_count, error_count);
    
    Ok(())
}

#[cfg(not(feature = "async"))]
fn main() {
    eprintln!("This example requires the 'async' feature to be enabled.");
    eprintln!("Run with: cargo run --example async_extraction --features async");
    std::process::exit(1);
}
