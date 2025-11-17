//! Command-line argument definitions
//!
//! This module defines the CLI argument structure using Clap's derive API.
//! It provides a type-safe way to parse and access command-line arguments.
//!
//! # Examples
//!
//! ```no_run
//! use clap::Parser;
//! use omniparse::cli::args::Cli;
//!
//! let args = Cli::parse();
//! println!("Processing {} files", args.files.len());
//! println!("Output format: {:?}", args.format);
//! ```

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Omniparse - Extract text and metadata from various file formats
///
/// This is the main CLI structure that defines all command-line arguments
/// and options for the Omniparse tool.
///
/// # Examples
///
/// Basic usage:
/// ```bash
/// omniparse document.pdf
/// omniparse --format json file1.txt file2.docx
/// omniparse --metadata-only --output results.json *.pdf
/// ```
#[derive(Parser, Debug)]
#[command(name = "omniparse")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Input file paths to process
    ///
    /// One or more file paths to extract content from. Supports glob patterns
    /// when expanded by the shell.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Output format
    ///
    /// Controls how the extraction results are formatted. Options are:
    /// - text: Human-readable plain text (default)
    /// - json: JSON format for programmatic processing
    /// - yaml: YAML format
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: OutputFormat,

    /// Extract metadata only (no content)
    ///
    /// When enabled, only metadata is extracted and displayed. The actual
    /// content (text or binary) is not included in the output.
    #[arg(short, long)]
    pub metadata_only: bool,

    /// Detect file type only (no extraction)
    ///
    /// When enabled, only file type detection is performed. No parsing or
    /// content extraction occurs. Useful for quickly identifying file types.
    #[arg(short, long)]
    pub detect_only: bool,

    /// Output file path (stdout if not specified)
    ///
    /// If provided, results are written to this file instead of stdout.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Enable verbose output
    ///
    /// Prints additional information to stderr, including progress messages
    /// and summary statistics for batch operations.
    #[arg(short, long)]
    pub verbose: bool,

    /// Process files in parallel
    ///
    /// When enabled and the `parallel` feature is available, files are
    /// processed in parallel using multiple threads for better performance.
    #[arg(short, long)]
    pub parallel: bool,
}

/// Output format options
///
/// Defines the available output formats for extraction results.
#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    /// Plain text output
    ///
    /// Human-readable format with labeled sections for metadata and content.
    Text,
    /// JSON output
    ///
    /// Structured JSON format suitable for programmatic processing.
    Json,
    /// YAML output
    ///
    /// YAML format, similar to JSON but more human-readable.
    Yaml,
}
