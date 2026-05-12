//! Command-line interface functionality
//!
//! This module contains the CLI implementation for Omniparse, including:
//!
//! - **args**: Command-line argument parsing using Clap
//! - **output**: Output formatting for different formats (text, JSON, YAML)
//!
//! The CLI supports various options for controlling extraction behavior,
//! output format, and processing mode (sequential or parallel).

pub mod args;
pub mod models;
pub mod output;
