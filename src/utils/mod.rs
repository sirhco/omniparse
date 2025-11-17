//! Utility functions and helpers
//!
//! This module provides utility functions for:
//!
//! - **streaming**: Memory-efficient processing of large files
//! - **parallel**: Parallel batch processing of multiple files
//!
//! # Examples
//!
//! ## Streaming large files
//!
//! ```no_run
//! use omniparse::utils::streaming::{LimitedReader, DEFAULT_MEMORY_LIMIT};
//! use std::fs::File;
//! use std::io::Read;
//!
//! let file = File::open("large_file.txt")?;
//! let mut limited = LimitedReader::new(file, DEFAULT_MEMORY_LIMIT);
//!
//! let mut buffer = Vec::new();
//! match limited.read_to_end(&mut buffer) {
//!     Ok(_) => println!("Read {} bytes", limited.bytes_read()),
//!     Err(e) => println!("Memory limit exceeded: {}", e),
//! }
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! ## Parallel processing
//!
//! ```no_run
//! use omniparse::core::Extractor;
//! use omniparse::utils::parallel::process_files_parallel;
//!
//! let extractor = Extractor::new();
//! let files = vec!["file1.pdf", "file2.docx", "file3.txt"];
//!
//! let results = process_files_parallel(&extractor, &files);
//! for file_result in results {
//!     match file_result.result {
//!         Ok(extraction) => println!("{}: {}", file_result.path, extraction.mime_type),
//!         Err(e) => eprintln!("{}: {}", file_result.path, e),
//!     }
//! }
//! ```

pub mod streaming;
pub mod parallel;
