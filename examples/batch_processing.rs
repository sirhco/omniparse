//! Batch processing example
//!
//! This example demonstrates how to process multiple files efficiently,
//! both sequentially and in parallel.
//!
//! Run with:
//! ```bash
//! cargo run --example batch_processing
//! ```
//!
//! For parallel processing (requires the 'parallel' feature):
//! ```bash
//! cargo run --example batch_processing --features parallel
//! ```

use omniparse::core::Extractor;
use omniparse::utils::parallel::{process_files_parallel, process_files_sequential};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // List of files to process
    let files = vec![
        "test_data/text/sample.txt",
        "test_data/text/sample.json",
        "test_data/text/sample.csv",
        "test_data/text/sample.xml",
        "test_data/document/sample.pdf",
        "test_data/image/sample.jpg",
        "test_data/archive/sample.zip",
    ];
    
    println!("Batch Processing Example");
    println!("{}", "=".repeat(60));
    println!("Processing {} files\n", files.len());
    
    let extractor = Extractor::new();
    
    // Sequential processing
    println!("📊 Sequential Processing:");
    println!("{}", "-".repeat(60));
    let start = Instant::now();
    let results = process_files_sequential(&extractor, &files);
    let sequential_duration = start.elapsed();
    
    display_results(&results);
    println!("⏱️  Time: {:?}\n", sequential_duration);
    
    // Parallel processing
    println!("⚡ Parallel Processing:");
    println!("{}", "-".repeat(60));
    let start = Instant::now();
    let results = process_files_parallel(&extractor, &files);
    let parallel_duration = start.elapsed();
    
    display_results(&results);
    println!("⏱️  Time: {:?}", parallel_duration);
    
    // Performance comparison
    if cfg!(feature = "parallel") {
        println!("\n📈 Performance Comparison:");
        println!("{}", "-".repeat(60));
        println!("Sequential: {:?}", sequential_duration);
        println!("Parallel:   {:?}", parallel_duration);
        
        if parallel_duration < sequential_duration {
            let speedup = sequential_duration.as_secs_f64() / parallel_duration.as_secs_f64();
            println!("Speedup:    {:.2}x faster", speedup);
        }
    } else {
        println!("\n💡 Tip: Enable the 'parallel' feature for true parallel processing!");
        println!("   cargo run --example batch_processing --features parallel");
    }
    
    Ok(())
}

fn display_results(results: &[omniparse::utils::parallel::FileResult]) {
    let mut success_count = 0;
    let mut error_count = 0;
    
    for file_result in results {
        match &file_result.result {
            Ok(extraction) => {
                success_count += 1;
                println!(
                    "  ✅ {} → {} (confidence: {:.0}%)",
                    file_result.path,
                    extraction.mime_type,
                    extraction.detection_confidence * 100.0
                );
            }
            Err(e) => {
                error_count += 1;
                println!("  ❌ {} → Error: {}", file_result.path, e);
            }
        }
    }
    
    println!("\n📊 Summary: {} succeeded, {} failed", success_count, error_count);
}
