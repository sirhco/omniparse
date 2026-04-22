//! Type detection implementation

use std::path::Path;
use std::fs::File;
use std::io::Read;
use crate::core::Result;
use super::magic::{MagicPattern, get_magic_patterns, detect_openxml_type, detect_ole2_type};
use super::confidence::calculate_confidence;

/// Detection method used to identify file type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionMethod {
    /// Detected via magic byte patterns
    MagicBytes,
    /// Detected via content analysis heuristics
    ContentAnalysis,
    /// Detected via file extension only
    Extension,
    /// Detection method unknown or failed
    Unknown,
}

/// Result of type detection operation
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// Detected MIME type
    pub mime_type: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Method used for detection
    pub detected_by: DetectionMethod,
}

/// Trait for type detectors
pub trait Detector {
    /// Detect file type from data and optional filename
    fn detect(&self, data: &[u8], filename: Option<&str>) -> DetectionResult;
}

/// Main type detector implementation
pub struct TypeDetector {
    magic_patterns: Vec<MagicPattern>,
}

impl TypeDetector {
    /// Create a new TypeDetector with default magic patterns
    pub fn new() -> Self {
        Self {
            magic_patterns: get_magic_patterns(),
        }
    }
    
    /// Detect file type from bytes
    pub fn detect_from_bytes(&self, data: &[u8]) -> DetectionResult {
        // Check for OpenXML formats first (XLSX, PPTX, DOCX, etc.)
        // This needs to be done before generic ZIP detection
        if data.len() >= 4 && &data[0..4] == b"PK\x03\x04" {
            if let Some(mime_type) = detect_openxml_type(data) {
                return DetectionResult {
                    mime_type,
                    confidence: 0.95,
                    detected_by: DetectionMethod::MagicBytes,
                };
            }
        }
        
        // Check for OLE2 formats (DOC, XLS, PPT)
        // This needs to be done before generic OLE2 detection
        if data.len() >= 8 && &data[0..8] == b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1" {
            if let Some((mime_type, confidence)) = detect_ole2_type(data) {
                return DetectionResult {
                    mime_type,
                    confidence,
                    detected_by: DetectionMethod::MagicBytes,
                };
            }
        }
        
        // Try magic bytes
        if let Some((mime_type, confidence)) = self.check_magic_bytes(data) {
            return DetectionResult {
                mime_type,
                confidence,
                detected_by: DetectionMethod::MagicBytes,
            };
        }
        
        // Try content analysis
        if let Some((mime_type, confidence)) = self.analyze_content(data) {
            return DetectionResult {
                mime_type,
                confidence,
                detected_by: DetectionMethod::ContentAnalysis,
            };
        }
        
        // Unknown type
        DetectionResult {
            mime_type: "application/octet-stream".to_string(),
            confidence: 0.1,
            detected_by: DetectionMethod::Unknown,
        }
    }
    
    /// Detect file type from a file path
    pub fn detect_from_path(&self, path: &Path) -> Result<DetectionResult> {
        // Read first 8KB for magic byte detection
        let mut file = File::open(path)?;
        let mut buffer = vec![0u8; 8192];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);
        
        // Check for OpenXML formats first (XLSX, PPTX, DOCX, etc.)
        // This needs to be done before generic ZIP detection
        if buffer.len() >= 4 && &buffer[0..4] == b"PK\x03\x04" {
            if let Some(mime_type) = detect_openxml_type(&buffer) {
                return Ok(DetectionResult {
                    mime_type,
                    confidence: 0.95,
                    detected_by: DetectionMethod::MagicBytes,
                });
            }
        }
        
        // Check for OLE2 formats (DOC, XLS, PPT)
        // This needs to be done before generic OLE2 detection
        if buffer.len() >= 8 && &buffer[0..8] == b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1" {
            if let Some((mime_type, confidence)) = detect_ole2_type(&buffer) {
                return Ok(DetectionResult {
                    mime_type,
                    confidence,
                    detected_by: DetectionMethod::MagicBytes,
                });
            }
        }
        
        // Try magic bytes
        if let Some((mime_type, confidence)) = self.check_magic_bytes(&buffer) {
            return Ok(DetectionResult {
                mime_type,
                confidence,
                detected_by: DetectionMethod::MagicBytes,
            });
        }
        
        // Try content analysis
        if let Some((mime_type, confidence)) = self.analyze_content(&buffer) {
            return Ok(DetectionResult {
                mime_type,
                confidence,
                detected_by: DetectionMethod::ContentAnalysis,
            });
        }
        
        // Try extension as fallback
        if let Some((mime_type, confidence)) = self.detect_from_extension(path) {
            return Ok(DetectionResult {
                mime_type,
                confidence,
                detected_by: DetectionMethod::Extension,
            });
        }
        
        // Unknown type
        Ok(DetectionResult {
            mime_type: "application/octet-stream".to_string(),
            confidence: 0.1,
            detected_by: DetectionMethod::Unknown,
        })
    }
    
    /// Check magic bytes against known patterns
    fn check_magic_bytes(&self, data: &[u8]) -> Option<(String, f32)> {
        for pattern in &self.magic_patterns {
            if pattern.matches(data) {
                let confidence = calculate_confidence(DetectionMethod::MagicBytes);
                return Some((pattern.mime_type.clone(), confidence));
            }
        }
        None
    }
    
    /// Analyze content using heuristics
    fn analyze_content(&self, data: &[u8]) -> Option<(String, f32)> {
        if data.is_empty() {
            return None;
        }
        
        // Check if content is valid UTF-8 text
        if let Ok(text) = std::str::from_utf8(data) {
            // Check for JSON-like content
            let trimmed = text.trim();
            if (trimmed.starts_with('{') && trimmed.ends_with('}')) ||
               (trimmed.starts_with('[') && trimmed.ends_with(']')) {
                let confidence = calculate_confidence(DetectionMethod::ContentAnalysis);
                return Some(("application/json".to_string(), confidence));
            }
            
            // Check for XML-like content
            if trimmed.starts_with('<') && trimmed.contains('>') {
                let confidence = calculate_confidence(DetectionMethod::ContentAnalysis);
                if trimmed.contains("<?xml") {
                    return Some(("text/xml".to_string(), confidence));
                } else if trimmed.contains("<html") || trimmed.contains("<HTML") {
                    return Some(("text/html".to_string(), confidence));
                } else if trimmed.contains("<svg") {
                    return Some(("image/svg+xml".to_string(), confidence));
                }
                return Some(("text/xml".to_string(), confidence));
            }
            
            // Check for CSS-like content
            if self.looks_like_css(text) {
                // CSS detection has lower confidence (0.6) as specified in requirements
                return Some(("text/css".to_string(), 0.6));
            }
            
            // Check for CSV-like content
            if text.lines().count() > 1 {
                let first_line = text.lines().next().unwrap_or("");
                if first_line.contains(',') || first_line.contains('\t') {
                    let confidence = calculate_confidence(DetectionMethod::ContentAnalysis);
                    return Some(("text/csv".to_string(), confidence));
                }
            }
            
            // Default to plain text
            let confidence = calculate_confidence(DetectionMethod::ContentAnalysis);
            return Some(("text/plain".to_string(), confidence));
        }
        
        None
    }
    
    /// Check if content looks like CSS
    fn looks_like_css(&self, text: &str) -> bool {
        let trimmed = text.trim();
        
        // Empty or very short content is unlikely to be CSS
        if trimmed.len() < 10 {
            return false;
        }
        
        // Count CSS-like patterns
        let mut css_indicators = 0;
        
        // Check for CSS selectors and rules (e.g., "selector { property: value; }")
        if trimmed.contains('{') && trimmed.contains('}') && trimmed.contains(':') {
            css_indicators += 1;
        }
        
        // Check for common CSS at-rules
        let at_rules = ["@import", "@media", "@charset", "@font-face", "@keyframes", "@supports"];
        for at_rule in &at_rules {
            if trimmed.contains(at_rule) {
                css_indicators += 1;
                break;
            }
        }
        
        // Check for common CSS properties
        let common_properties = [
            "color:", "background:", "margin:", "padding:", "font-",
            "border:", "width:", "height:", "display:", "position:"
        ];
        for prop in &common_properties {
            if trimmed.contains(prop) {
                css_indicators += 1;
                break;
            }
        }
        
        // Check for semicolons (common in CSS)
        if trimmed.contains(';') {
            css_indicators += 1;
        }
        
        // Require at least 2 indicators to consider it CSS
        css_indicators >= 2
    }
    
    /// Detect from file extension as fallback
    fn detect_from_extension(&self, path: &Path) -> Option<(String, f32)> {
        let extension = path.extension()?.to_str()?.to_lowercase();
        
        let mime_type = match extension.as_str() {
            // Documents
            "pdf" => "application/pdf",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "odt" => "application/vnd.oasis.opendocument.text",
            "rtf" => "application/rtf",
            
            // Spreadsheets
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "ods" => "application/vnd.oasis.opendocument.spreadsheet",
            
            // Presentations
            "ppt" => "application/vnd.ms-powerpoint",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "odp" => "application/vnd.oasis.opendocument.presentation",
            
            // Images
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "tiff" | "tif" => "image/tiff",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "ico" => "image/x-icon",
            
            // Archives
            "zip" => "application/zip",
            "tar" => "application/x-tar",
            "gz" => "application/gzip",
            "bz2" => "application/x-bzip2",
            "7z" => "application/x-7z-compressed",
            "rar" => "application/x-rar-compressed",
            "xz" => "application/x-xz",
            
            // Text
            "txt" => "text/plain",
            "csv" => "text/csv",
            "json" => "application/json",
            "xml" => "text/xml",
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "md" | "markdown" => "text/markdown",
            "epub" => "application/epub+zip",
            
            // Audio
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "flac" => "audio/x-flac",
            "ogg" => "audio/ogg",
            "m4a" => "audio/x-m4a",
            
            // Video
            "mp4" => "video/mp4",
            "avi" => "video/x-msvideo",
            "mkv" => "video/x-matroska",
            "webm" => "video/webm",
            "mov" => "video/quicktime",
            
            _ => return None,
        };
        
        let confidence = calculate_confidence(DetectionMethod::Extension);
        Some((mime_type.to_string(), confidence))
    }
}

impl Default for TypeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector for TypeDetector {
    fn detect(&self, data: &[u8], filename: Option<&str>) -> DetectionResult {
        // Check for OpenXML formats first (XLSX, PPTX, DOCX, etc.)
        // This needs to be done before generic ZIP detection
        if data.len() >= 4 && &data[0..4] == b"PK\x03\x04" {
            if let Some(mime_type) = detect_openxml_type(data) {
                return DetectionResult {
                    mime_type,
                    confidence: 0.95,
                    detected_by: DetectionMethod::MagicBytes,
                };
            }
        }
        
        // Check for OLE2 formats (DOC, XLS, PPT)
        // This needs to be done before generic OLE2 detection
        if data.len() >= 8 && &data[0..8] == b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1" {
            if let Some((mime_type, confidence)) = detect_ole2_type(data) {
                return DetectionResult {
                    mime_type,
                    confidence,
                    detected_by: DetectionMethod::MagicBytes,
                };
            }
        }
        
        // Try magic bytes
        if let Some((mime_type, confidence)) = self.check_magic_bytes(data) {
            return DetectionResult {
                mime_type,
                confidence,
                detected_by: DetectionMethod::MagicBytes,
            };
        }
        
        // Try content analysis
        if let Some((mime_type, confidence)) = self.analyze_content(data) {
            return DetectionResult {
                mime_type,
                confidence,
                detected_by: DetectionMethod::ContentAnalysis,
            };
        }
        
        // Try extension if filename provided
        if let Some(filename) = filename {
            let path = Path::new(filename);
            if let Some((mime_type, confidence)) = self.detect_from_extension(path) {
                return DetectionResult {
                    mime_type,
                    confidence,
                    detected_by: DetectionMethod::Extension,
                };
            }
        }
        
        // Unknown type
        DetectionResult {
            mime_type: "application/octet-stream".to_string(),
            confidence: 0.1,
            detected_by: DetectionMethod::Unknown,
        }
    }
}
