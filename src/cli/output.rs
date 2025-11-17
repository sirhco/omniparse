//! Output formatting for CLI
//!
//! This module handles formatting of extraction and detection results for
//! different output formats (text, JSON, YAML). It provides functions to
//! write formatted output to any writer.
//!
//! # Examples
//!
//! ```no_run
//! use omniparse::cli::output::format_extraction_result;
//! use omniparse::cli::args::OutputFormat;
//! use omniparse::extract_from_path;
//! use std::io::stdout;
//!
//! let result = extract_from_path("document.pdf")?;
//! let mut output = stdout();
//!
//! format_extraction_result(&mut output, &result, &OutputFormat::Text, false)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use omniparse::core::result::{Content, ExtractionResult, Metadata, MetadataValue};
use omniparse::detection::DetectionResult;
use serde::Serialize;
use std::io::Write;

use super::args::OutputFormat;

/// Format and write extraction results
///
/// This function formats an extraction result according to the specified format
/// and writes it to the provided writer.
///
/// # Arguments
///
/// * `writer` - The writer to output to (e.g., stdout, file)
/// * `result` - The extraction result to format
/// * `format` - The desired output format
/// * `metadata_only` - If true, only metadata is included (no content)
///
/// # Examples
///
/// ```no_run
/// use omniparse::cli::output::format_extraction_result;
/// use omniparse::cli::args::OutputFormat;
/// use omniparse::extract_from_path;
/// use std::io::stdout;
///
/// let result = extract_from_path("file.txt")?;
/// format_extraction_result(&mut stdout(), &result, &OutputFormat::Json, false)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn format_extraction_result<W: Write + ?Sized>(
    writer: &mut W,
    result: &ExtractionResult,
    format: &OutputFormat,
    metadata_only: bool,
) -> std::io::Result<()> {
    match format {
        OutputFormat::Text => format_text(writer, result, metadata_only),
        OutputFormat::Json => format_json(writer, result, metadata_only),
        OutputFormat::Yaml => format_yaml(writer, result, metadata_only),
    }
}

/// Format and write detection results
///
/// This function formats a type detection result according to the specified
/// format and writes it to the provided writer.
///
/// # Arguments
///
/// * `writer` - The writer to output to
/// * `result` - The detection result to format
/// * `format` - The desired output format
///
/// # Examples
///
/// ```no_run
/// use omniparse::cli::output::format_detection_result;
/// use omniparse::cli::args::OutputFormat;
/// use omniparse::detection::TypeDetector;
/// use std::io::stdout;
///
/// let detector = TypeDetector::new();
/// let result = detector.detect_from_path("file.pdf")?;
/// format_detection_result(&mut stdout(), &result, &OutputFormat::Text)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn format_detection_result<W: Write + ?Sized>(
    writer: &mut W,
    result: &DetectionResult,
    format: &OutputFormat,
) -> std::io::Result<()> {
    match format {
        OutputFormat::Text => format_detection_text(writer, result),
        OutputFormat::Json => format_detection_json(writer, result),
        OutputFormat::Yaml => format_detection_yaml(writer, result),
    }
}

// Text formatting

fn format_text<W: Write + ?Sized>(
    writer: &mut W,
    result: &ExtractionResult,
    metadata_only: bool,
) -> std::io::Result<()> {
    writeln!(writer, "MIME Type: {}", result.mime_type)?;
    writeln!(writer, "Detection Confidence: {:.2}", result.detection_confidence)?;
    writeln!(writer)?;
    
    // Metadata section
    writeln!(writer, "Metadata:")?;
    format_metadata_text(writer, &result.metadata)?;
    writeln!(writer)?;
    
    // Content section (unless metadata-only)
    if !metadata_only {
        writeln!(writer, "Content:")?;
        match &result.content {
            Content::Text(text) => {
                writeln!(writer, "{}", text)?;
            }
            Content::Binary(data) => {
                writeln!(writer, "[Binary data: {} bytes]", data.len())?;
            }
            Content::None => {
                writeln!(writer, "[No content extracted]")?;
            }
        }
    }
    
    Ok(())
}

fn format_metadata_text<W: Write + ?Sized>(writer: &mut W, metadata: &Metadata) -> std::io::Result<()> {
    let mut keys: Vec<_> = metadata.keys().collect();
    keys.sort();
    
    if keys.is_empty() {
        writeln!(writer, "  (none)")?;
    } else {
        for key in keys {
            if let Some(value) = metadata.get(key) {
                write!(writer, "  {}: ", key)?;
                format_metadata_value_text(writer, value)?;
                writeln!(writer)?;
            }
        }
    }
    
    Ok(())
}

fn format_metadata_value_text<W: Write + ?Sized>(
    writer: &mut W,
    value: &MetadataValue,
) -> std::io::Result<()> {
    match value {
        MetadataValue::Text(s) => write!(writer, "{}", s)?,
        MetadataValue::Number(n) => write!(writer, "{}", n)?,
        MetadataValue::Float(f) => write!(writer, "{}", f)?,
        MetadataValue::DateTime(dt) => write!(writer, "{}", dt.to_rfc3339())?,
        MetadataValue::Boolean(b) => write!(writer, "{}", b)?,
        MetadataValue::List(items) => {
            write!(writer, "[")?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    write!(writer, ", ")?;
                }
                format_metadata_value_text(writer, item)?;
            }
            write!(writer, "]")?;
        }
    }
    Ok(())
}

fn format_detection_text<W: Write + ?Sized>(
    writer: &mut W,
    result: &DetectionResult,
) -> std::io::Result<()> {
    writeln!(writer, "MIME Type: {}", result.mime_type)?;
    writeln!(writer, "Confidence: {:.2}", result.confidence)?;
    writeln!(writer, "Detected By: {:?}", result.detected_by)?;
    Ok(())
}

// JSON formatting

#[derive(Serialize)]
struct JsonExtractionOutput<'a> {
    mime_type: &'a str,
    detection_confidence: f32,
    metadata: &'a Metadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<JsonContent<'a>>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum JsonContent<'a> {
    Text(&'a str),
    Binary { bytes: usize },
    None,
}

fn format_json<W: Write + ?Sized>(
    writer: &mut W,
    result: &ExtractionResult,
    metadata_only: bool,
) -> std::io::Result<()> {
    let content = if metadata_only {
        None
    } else {
        Some(match &result.content {
            Content::Text(text) => JsonContent::Text(text),
            Content::Binary(data) => JsonContent::Binary { bytes: data.len() },
            Content::None => JsonContent::None,
        })
    };
    
    let output = JsonExtractionOutput {
        mime_type: &result.mime_type,
        detection_confidence: result.detection_confidence,
        metadata: &result.metadata,
        content,
    };
    
    serde_json::to_writer_pretty(&mut *writer, &output)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    writeln!(writer)?;
    
    Ok(())
}

#[derive(Serialize)]
struct JsonDetectionOutput<'a> {
    mime_type: &'a str,
    confidence: f32,
    detected_by: &'a str,
}

fn format_detection_json<W: Write + ?Sized>(
    writer: &mut W,
    result: &DetectionResult,
) -> std::io::Result<()> {
    let detected_by = match result.detected_by {
        omniparse::detection::DetectionMethod::MagicBytes => "magic_bytes",
        omniparse::detection::DetectionMethod::ContentAnalysis => "content_analysis",
        omniparse::detection::DetectionMethod::Extension => "extension",
        omniparse::detection::DetectionMethod::Unknown => "unknown",
    };
    
    let output = JsonDetectionOutput {
        mime_type: &result.mime_type,
        confidence: result.confidence,
        detected_by,
    };
    
    serde_json::to_writer_pretty(&mut *writer, &output)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    writeln!(writer)?;
    
    Ok(())
}

// YAML formatting

fn format_yaml<W: Write + ?Sized>(
    writer: &mut W,
    result: &ExtractionResult,
    metadata_only: bool,
) -> std::io::Result<()> {
    let content = if metadata_only {
        None
    } else {
        Some(match &result.content {
            Content::Text(text) => JsonContent::Text(text),
            Content::Binary(data) => JsonContent::Binary { bytes: data.len() },
            Content::None => JsonContent::None,
        })
    };
    
    let output = JsonExtractionOutput {
        mime_type: &result.mime_type,
        detection_confidence: result.detection_confidence,
        metadata: &result.metadata,
        content,
    };
    
    serde_yaml::to_writer(writer, &output)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    Ok(())
}

fn format_detection_yaml<W: Write + ?Sized>(
    writer: &mut W,
    result: &DetectionResult,
) -> std::io::Result<()> {
    let detected_by = match result.detected_by {
        omniparse::detection::DetectionMethod::MagicBytes => "magic_bytes",
        omniparse::detection::DetectionMethod::ContentAnalysis => "content_analysis",
        omniparse::detection::DetectionMethod::Extension => "extension",
        omniparse::detection::DetectionMethod::Unknown => "unknown",
    };
    
    let output = JsonDetectionOutput {
        mime_type: &result.mime_type,
        confidence: result.confidence,
        detected_by,
    };
    
    serde_yaml::to_writer(writer, &output)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    Ok(())
}
