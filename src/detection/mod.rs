//! File type detection functionality
//!
//! This module provides file type detection capabilities using multiple methods:
//!
//! - **Magic bytes**: Detecting files by their binary signatures
//! - **Content analysis**: Heuristic analysis of file content
//! - **Extension fallback**: Using file extensions as a last resort
//!
//! The detection system assigns confidence scores based on the method used,
//! with magic bytes being the most reliable and extensions the least.
//!
//! # Examples
//!
//! ```no_run
//! use omniparse::detection::TypeDetector;
//! use std::path::Path;
//!
//! let detector = TypeDetector::new();
//!
//! // Detect from a file path
//! let result = detector.detect_from_path(Path::new("document.pdf"))?;
//! println!("Detected: {} (confidence: {:.2})", result.mime_type, result.confidence);
//!
//! // Detect from bytes
//! let data = std::fs::read("file.jpg")?;
//! let result = detector.detect_from_bytes(&data);
//! println!("Detected: {}", result.mime_type);
//! # Ok::<(), omniparse::Error>(())
//! ```

pub mod detector;
pub mod magic;
pub mod confidence;

pub use detector::{DetectionMethod, DetectionResult, Detector, TypeDetector};
