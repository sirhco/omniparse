//! Command-line argument definitions
//!
//! This module defines the CLI argument structure using Clap's derive API.
//! Two invocation shapes are supported:
//!
//! 1. Bare extraction: `omniparse [FILES...]` — the historical default.
//! 2. Subcommands: `omniparse <subcommand> ...` — currently just `models`
//!    for managing the ML OCR model cache.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Omniparse - Extract text and metadata from various file formats
#[derive(Parser, Debug)]
#[command(name = "omniparse")]
#[command(version)]
#[command(about = "A Rust toolkit for detecting and extracting metadata, text, and content from various file formats")]
#[command(long_about = "Omniparse - Extract text and metadata from 35+ file formats\n\n\
Supported formats:\n\
  Text: TXT, JSON, CSV, XML, HTML, CSS, RTF\n\
  Documents: PDF, DOCX, DOC, XLSX, XLS, PPTX, PPT, ODT, ODS, ODP\n\
  Images: JPEG, PNG, TIFF\n\
  Archives: ZIP, TAR\n\n\
Examples:\n\
  omniparse document.pdf\n\
  omniparse --format json webpage.html\n\
  omniparse --metadata-only spreadsheet.xlsx\n\
  omniparse --parallel *.pdf *.docx\n\
  omniparse models download           # pre-fetch ML OCR models\n\
  omniparse models verify             # check sha256 of cached models")]
#[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
pub struct Cli {
    /// Optional subcommand. When absent, the bare extraction flow runs over
    /// `files`.
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Input file paths to process (bare extraction mode).
    ///
    /// One or more file paths to extract content from. Supports glob patterns
    /// when expanded by the shell. Required unless a subcommand is given.
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
    #[arg(short, long)]
    pub metadata_only: bool,

    /// Detect file type only (no extraction)
    #[arg(short, long)]
    pub detect_only: bool,

    /// Output file path (stdout if not specified)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Process files in parallel
    #[arg(short, long)]
    pub parallel: bool,
}

/// Top-level subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Manage the ML OCR model cache (download / verify / inspect).
    Models {
        #[command(subcommand)]
        action: ModelsAction,
    },
}

/// `omniparse models <action>`.
#[derive(Subcommand, Debug)]
pub enum ModelsAction {
    /// Download every required ML OCR model into the cache directory.
    Download {
        /// Re-download even if a valid cached copy is present.
        #[arg(long)]
        force: bool,
    },
    /// Re-hash each cached model and compare against the pinned SHA-256.
    Verify,
    /// Print the resolved model cache directory and exit.
    Path,
    /// List each model: name, on-disk size, sha256, ok/missing.
    List,
}

/// Output format options
#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Yaml,
}
