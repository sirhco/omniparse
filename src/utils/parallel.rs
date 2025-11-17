//! Parallel processing utilities
//!
//! This module provides utilities for processing multiple files in parallel.
//! When the `parallel` feature is enabled, files are processed using Rayon's
//! parallel iterators. Otherwise, files are processed sequentially.
//!
//! # Examples
//!
//! ```no_run
//! use omniparse::core::Extractor;
//! use omniparse::utils::parallel::process_files_parallel;
//!
//! let extractor = Extractor::new();
//! let files = vec!["file1.pdf", "file2.docx", "file3.txt"];
//!
//! let results = process_files_parallel(&extractor, &files);
//!
//! for file_result in results {
//!     match file_result.result {
//!         Ok(extraction) => {
//!             println!("{}: {} ({})",
//!                 file_result.path,
//!                 extraction.mime_type,
//!                 extraction.detection_confidence
//!             );
//!         }
//!         Err(e) => {
//!             eprintln!("{}: Error - {}", file_result.path, e);
//!         }
//!     }
//! }
//! ```

use crate::core::{Extractor, Result};
use crate::core::result::ExtractionResult;
use std::path::Path;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Result of processing a single file in a batch
///
/// This structure contains the file path and the extraction result or error
/// for that file. It's used when processing multiple files to track which
/// files succeeded and which failed.
///
/// # Examples
///
/// ```no_run
/// use omniparse::utils::parallel::FileResult;
///
/// fn print_result(file_result: &FileResult) {
///     match &file_result.result {
///         Ok(extraction) => {
///             println!("{}: Success - {}", file_result.path, extraction.mime_type);
///         }
///         Err(e) => {
///             println!("{}: Error - {}", file_result.path, e);
///         }
///     }
/// }
/// ```
#[derive(Debug)]
pub struct FileResult {
    /// Path to the file that was processed
    pub path: String,
    /// Extraction result or error
    pub result: Result<ExtractionResult>,
}

/// Process multiple files in parallel
///
/// When the `parallel` feature is enabled, this function uses Rayon to process
/// files in parallel across multiple threads. Each file is processed independently,
/// and errors in one file don't affect the processing of others.
///
/// **Note:** This function requires the `parallel` feature to be enabled for
/// true parallel processing. Without the feature, files are processed sequentially.
///
/// # Arguments
///
/// * `extractor` - The extractor to use for processing
/// * `paths` - Slice of file paths to process
///
/// # Returns
///
/// A vector of `FileResult` containing the result for each file.
///
/// # Examples
///
/// ```no_run
/// use omniparse::core::Extractor;
/// use omniparse::utils::parallel::process_files_parallel;
///
/// let extractor = Extractor::new();
/// let files = vec!["doc1.pdf", "doc2.docx", "doc3.txt"];
///
/// let results = process_files_parallel(&extractor, &files);
///
/// let success_count = results.iter().filter(|r| r.result.is_ok()).count();
/// println!("Successfully processed {} out of {} files", success_count, files.len());
/// ```
#[cfg(feature = "parallel")]
pub fn process_files_parallel<P: AsRef<Path> + Sync>(
    extractor: &Extractor,
    paths: &[P],
) -> Vec<FileResult> {
    paths
        .par_iter()
        .map(|path| {
            let path_ref = path.as_ref();
            let result = extractor.extract_from_path(path_ref);
            FileResult {
                path: path_ref.display().to_string(),
                result,
            }
        })
        .collect()
}

/// Process multiple files sequentially (fallback when parallel feature is disabled)
///
/// This is the fallback implementation used when the `parallel` feature is not enabled.
/// It processes files one at a time in order.
#[cfg(not(feature = "parallel"))]
pub fn process_files_parallel<P: AsRef<Path>>(
    extractor: &Extractor,
    paths: &[P],
) -> Vec<FileResult> {
    paths
        .iter()
        .map(|path| {
            let path_ref = path.as_ref();
            let result = extractor.extract_from_path(path_ref);
            FileResult {
                path: path_ref.display().to_string(),
                result,
            }
        })
        .collect()
}

/// Process multiple files sequentially
///
/// This function always processes files sequentially, regardless of whether
/// the `parallel` feature is enabled. Use this when you need deterministic
/// ordering or when parallel processing is not desired.
///
/// # Arguments
///
/// * `extractor` - The extractor to use for processing
/// * `paths` - Slice of file paths to process
///
/// # Returns
///
/// A vector of `FileResult` containing the result for each file, in order.
///
/// # Examples
///
/// ```no_run
/// use omniparse::core::Extractor;
/// use omniparse::utils::parallel::process_files_sequential;
///
/// let extractor = Extractor::new();
/// let files = vec!["file1.txt", "file2.txt", "file3.txt"];
///
/// let results = process_files_sequential(&extractor, &files);
///
/// for (i, file_result) in results.iter().enumerate() {
///     println!("File {}: {}", i + 1, file_result.path);
/// }
/// ```
pub fn process_files_sequential<P: AsRef<Path>>(
    extractor: &Extractor,
    paths: &[P],
) -> Vec<FileResult> {
    paths
        .iter()
        .map(|path| {
            let path_ref = path.as_ref();
            let result = extractor.extract_from_path(path_ref);
            FileResult {
                path: path_ref.display().to_string(),
                result,
            }
        })
        .collect()
}
