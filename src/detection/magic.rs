//! Magic byte patterns for file type detection

use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Magic byte pattern for file type detection
#[derive(Debug, Clone)]
pub struct MagicPattern {
    /// MIME type this pattern identifies
    pub mime_type: String,
    /// Offset in bytes where pattern should be found
    pub offset: usize,
    /// Byte pattern to match
    pub pattern: Vec<u8>,
    /// Optional mask to apply before matching
    pub mask: Option<Vec<u8>>,
}

impl MagicPattern {
    /// Create a new magic pattern
    pub fn new(mime_type: &str, offset: usize, pattern: Vec<u8>) -> Self {
        Self {
            mime_type: mime_type.to_string(),
            offset,
            pattern,
            mask: None,
        }
    }
    
    /// Create a new magic pattern with a mask
    pub fn with_mask(mime_type: &str, offset: usize, pattern: Vec<u8>, mask: Vec<u8>) -> Self {
        Self {
            mime_type: mime_type.to_string(),
            offset,
            pattern,
            mask: Some(mask),
        }
    }
    
    /// Check if this pattern matches the given data
    pub fn matches(&self, data: &[u8]) -> bool {
        // Check if data is long enough
        if data.len() < self.offset + self.pattern.len() {
            return false;
        }
        
        let data_slice = &data[self.offset..self.offset + self.pattern.len()];
        
        // Apply mask if present
        if let Some(mask) = &self.mask {
            for i in 0..self.pattern.len() {
                if (data_slice[i] & mask[i]) != (self.pattern[i] & mask[i]) {
                    return false;
                }
            }
            true
        } else {
            data_slice == self.pattern.as_slice()
        }
    }
}

/// Detect OpenXML format by checking `[Content_Types].xml`
/// Returns the specific MIME type if it's an OpenXML document, None otherwise
pub fn detect_openxml_type(data: &[u8]) -> Option<String> {
    // Check if it starts with ZIP signature
    if data.len() < 4 || &data[0..4] != b"PK\x03\x04" {
        return None;
    }

    // Try to open as ZIP archive
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor).ok()?;

    // First check for OpenDocument format (has mimetype file)
    if let Ok(mut mimetype_file) = archive.by_name("mimetype") {
        let mut mimetype = String::new();
        if mimetype_file.read_to_string(&mut mimetype).is_ok() {
            let trimmed = mimetype.trim();
            // Return the exact MIME type from the mimetype file
            if trimmed.starts_with("application/vnd.oasis.opendocument.") {
                return Some(trimmed.to_string());
            }
        }
    }

    // If not OpenDocument, check for OpenXML format (has [Content_Types].xml)
    if let Ok(mut content_types_file) = archive.by_name("[Content_Types].xml") {
        let mut content = String::new();
        if content_types_file.read_to_string(&mut content).is_ok() {
            // Check for specific content type entries to distinguish formats
            if content.contains("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet") {
                return Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string());
            } else if content.contains("application/vnd.openxmlformats-officedocument.presentationml.presentation") {
                return Some("application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string());
            } else if content.contains("application/vnd.openxmlformats-officedocument.wordprocessingml.document") {
                return Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string());
            }
        }
    }

    None
}

/// Detect OLE2 format type by examining the directory structure
/// Returns the specific MIME type if it's a recognized OLE2 document (DOC, XLS, PPT), None otherwise
pub fn detect_ole2_type(data: &[u8]) -> Option<(String, f32)> {
    // Check if it starts with OLE2 signature
    if data.len() < 512 || &data[0..8] != b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1" {
        return None;
    }

    // OLE2 files contain directory entries that identify the document type
    // We'll search for specific stream names that are unique to each format
    
    // Convert data to searchable format for stream name detection
    // Stream names in OLE2 are stored in the directory entries
    
    // Look for Word-specific streams
    // Word documents contain "WordDocument" stream
    if contains_stream_name(data, b"WordDocument") || 
       contains_stream_name(data, b"Word") {
        return Some(("application/msword".to_string(), 0.90));
    }
    
    // Look for Excel-specific streams
    // Excel workbooks contain "Workbook" or "Book" stream
    if contains_stream_name(data, b"Workbook") || 
       contains_stream_name(data, b"Book") {
        return Some(("application/vnd.ms-excel".to_string(), 0.90));
    }
    
    // Look for PowerPoint-specific streams
    // PowerPoint presentations contain "PowerPoint Document" or "Current User" stream
    if contains_stream_name(data, b"PowerPoint Document") ||
       contains_stream_name(data, b"Current User") {
        return Some(("application/vnd.ms-powerpoint".to_string(), 0.90));
    }
    
    // If we can't determine the specific type, return None
    // This will allow fallback to extension-based detection
    None
}

/// Check if the OLE2 data contains a specific stream name
/// This searches through the binary data for stream name patterns
fn contains_stream_name(data: &[u8], stream_name: &[u8]) -> bool {
    // OLE2 directory entries start at sector 0 (after the 512-byte header)
    // Each directory entry is 128 bytes
    // The first 64 bytes of each entry contain the name in UTF-16LE
    
    // Search through the data for the stream name
    // We'll look for both ASCII and UTF-16LE encoded versions
    
    // Check for ASCII version (simple search)
    if search_bytes(data, stream_name) {
        return true;
    }
    
    // Check for UTF-16LE version (with null bytes between characters)
    let utf16le_name = to_utf16le(stream_name);
    if search_bytes(data, &utf16le_name) {
        return true;
    }
    
    false
}

/// Search for a byte pattern in data
fn search_bytes(data: &[u8], pattern: &[u8]) -> bool {
    if pattern.is_empty() || data.len() < pattern.len() {
        return false;
    }
    
    for i in 0..=data.len() - pattern.len() {
        if &data[i..i + pattern.len()] == pattern {
            return true;
        }
    }
    
    false
}

/// Convert ASCII bytes to UTF-16LE encoding
fn to_utf16le(ascii: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(ascii.len() * 2);
    for &byte in ascii {
        result.push(byte);
        result.push(0);
    }
    result
}

/// Get the default magic byte patterns database
pub fn get_magic_patterns() -> Vec<MagicPattern> {
    vec![
        // Document formats
        MagicPattern::new("application/pdf", 0, b"%PDF-".to_vec()),
        MagicPattern::new("application/vnd.openxmlformats-officedocument.wordprocessingml.document", 0, 
            vec![0x50, 0x4B, 0x03, 0x04]), // ZIP signature (DOCX is ZIP-based)
        MagicPattern::new("application/vnd.oasis.opendocument.text", 0, 
            vec![0x50, 0x4B, 0x03, 0x04]), // ZIP signature (ODT is ZIP-based)
        // Note: OLE2 signature (DOC, XLS, PPT) is handled by detect_ole2_type() function
        // to distinguish between different OLE2 formats
        MagicPattern::new("application/rtf", 0, b"{\\rtf".to_vec()),
        
        // Image formats
        MagicPattern::new("image/jpeg", 0, vec![0xFF, 0xD8, 0xFF]),
        MagicPattern::new("image/png", 0, vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
        MagicPattern::new("image/gif", 0, b"GIF87a".to_vec()),
        MagicPattern::new("image/gif", 0, b"GIF89a".to_vec()),
        MagicPattern::new("image/tiff", 0, vec![0x49, 0x49, 0x2A, 0x00]), // Little-endian TIFF
        MagicPattern::new("image/tiff", 0, vec![0x4D, 0x4D, 0x00, 0x2A]), // Big-endian TIFF
        MagicPattern::new("image/bmp", 0, b"BM".to_vec()),
        MagicPattern::new("image/webp", 8, b"WEBP".to_vec()),
        MagicPattern::new("image/x-icon", 0, vec![0x00, 0x00, 0x01, 0x00]),
        // Require the literal <svg tag — the previous `<?xml` pattern
        // swallowed every XML file into image/svg+xml.
        MagicPattern::new("image/svg+xml", 0, b"<svg".to_vec()),
        
        // Archive formats
        MagicPattern::new("application/zip", 0, vec![0x50, 0x4B, 0x03, 0x04]),
        MagicPattern::new("application/zip", 0, vec![0x50, 0x4B, 0x05, 0x06]), // Empty ZIP
        MagicPattern::new("application/zip", 0, vec![0x50, 0x4B, 0x07, 0x08]), // Spanned ZIP
        MagicPattern::new("application/x-tar", 257, b"ustar".to_vec()),
        MagicPattern::new("application/gzip", 0, vec![0x1F, 0x8B]),
        MagicPattern::new("application/x-bzip2", 0, b"BZh".to_vec()),
        MagicPattern::new("application/x-7z-compressed", 0, vec![0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]),
        MagicPattern::new("application/x-rar-compressed", 0, b"Rar!\x1A\x07".to_vec()),
        MagicPattern::new("application/x-xz", 0, vec![0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00]),
        
        // Text formats
        MagicPattern::new("text/html", 0, b"<!DOCTYPE html".to_vec()),
        MagicPattern::new("text/html", 0, b"<html".to_vec()),
        MagicPattern::new("text/html", 0, b"<HTML".to_vec()),
        MagicPattern::new("text/xml", 0, b"<?xml".to_vec()),
        MagicPattern::new("application/json", 0, b"{".to_vec()),
        MagicPattern::new("application/json", 0, b"[".to_vec()),
        
        // Audio formats
        MagicPattern::new("audio/mpeg", 0, vec![0xFF, 0xFB]),
        MagicPattern::new("audio/mpeg", 0, vec![0xFF, 0xF3]),
        MagicPattern::new("audio/mpeg", 0, vec![0xFF, 0xF2]),
        MagicPattern::new("audio/mpeg", 0, b"ID3".to_vec()),
        MagicPattern::new("audio/wav", 0, b"RIFF".to_vec()),
        MagicPattern::new("audio/x-flac", 0, b"fLaC".to_vec()),
        MagicPattern::new("audio/ogg", 0, b"OggS".to_vec()),
        MagicPattern::new("audio/x-m4a", 4, b"ftyp".to_vec()),
        
        // Video formats
        MagicPattern::new("video/mp4", 4, b"ftyp".to_vec()),
        MagicPattern::new("video/x-msvideo", 0, b"RIFF".to_vec()),
        MagicPattern::new("video/x-matroska", 0, vec![0x1A, 0x45, 0xDF, 0xA3]),
        MagicPattern::new("video/webm", 0, vec![0x1A, 0x45, 0xDF, 0xA3]),
        MagicPattern::new("video/quicktime", 4, b"moov".to_vec()),
        MagicPattern::new("video/quicktime", 4, b"mdat".to_vec()),
        
        // Executable formats
        MagicPattern::new("application/x-executable", 0, vec![0x7F, 0x45, 0x4C, 0x46]), // ELF
        MagicPattern::new("application/x-mach-binary", 0, vec![0xFE, 0xED, 0xFA, 0xCE]), // Mach-O 32-bit
        MagicPattern::new("application/x-mach-binary", 0, vec![0xFE, 0xED, 0xFA, 0xCF]), // Mach-O 64-bit
        MagicPattern::new("application/x-msdownload", 0, b"MZ".to_vec()), // PE/COFF
        
        // Font formats
        MagicPattern::new("font/ttf", 0, vec![0x00, 0x01, 0x00, 0x00]),
        MagicPattern::new("font/otf", 0, b"OTTO".to_vec()),
        MagicPattern::new("font/woff", 0, b"wOFF".to_vec()),
        MagicPattern::new("font/woff2", 0, b"wOF2".to_vec()),
        
        // Database formats
        MagicPattern::new("application/x-sqlite3", 0, b"SQLite format 3\0".to_vec()),
        
        // Office formats (additional)
        MagicPattern::new("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", 0, 
            vec![0x50, 0x4B, 0x03, 0x04]), // XLSX
        MagicPattern::new("application/vnd.openxmlformats-officedocument.presentationml.presentation", 0, 
            vec![0x50, 0x4B, 0x03, 0x04]), // PPTX
        MagicPattern::new("application/vnd.oasis.opendocument.spreadsheet", 0, 
            vec![0x50, 0x4B, 0x03, 0x04]), // ODS
        MagicPattern::new("application/vnd.oasis.opendocument.presentation", 0, 
            vec![0x50, 0x4B, 0x03, 0x04]), // ODP
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_magic_pattern_matches() {
        let pattern = MagicPattern::new("application/pdf", 0, b"%PDF-".to_vec());
        assert!(pattern.matches(b"%PDF-1.4"));
        assert!(!pattern.matches(b"Not a PDF"));
    }
    
    #[test]
    fn test_magic_pattern_with_offset() {
        let pattern = MagicPattern::new("application/x-tar", 257, b"ustar".to_vec());
        let mut data = vec![0u8; 262];
        data[257..262].copy_from_slice(b"ustar");
        assert!(pattern.matches(&data));
    }
    
    #[test]
    fn test_magic_pattern_with_mask() {
        // Pattern: 0xF0, 0x0F with mask 0xF0, 0x0F
        // This means: check high nibble of first byte (0xF0) and low nibble of second byte (0x0F)
        let pattern = MagicPattern::with_mask(
            "test/masked",
            0,
            vec![0xF0, 0x0F],
            vec![0xF0, 0x0F],
        );
        // 0xF5 & 0xF0 = 0xF0, matches 0xF0 & 0xF0 = 0xF0 ✓
        // 0x0F & 0x0F = 0x0F, matches 0x0F & 0x0F = 0x0F ✓
        assert!(pattern.matches(&[0xF5, 0x0F]));
        
        // 0x0F & 0xF0 = 0x00, doesn't match 0xF0 & 0xF0 = 0xF0 ✗
        assert!(!pattern.matches(&[0x0F, 0xF0]));
    }
    
    #[test]
    fn test_get_magic_patterns_count() {
        let patterns = get_magic_patterns();
        assert!(patterns.len() >= 50, "Should have at least 50 MIME types");
    }
    
    #[test]
    fn test_detect_ole2_type_with_word_document() {
        // Create a minimal OLE2 file with WordDocument stream name
        let mut data = vec![0u8; 1024];
        // OLE2 header
        data[0..8].copy_from_slice(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1");
        // Add "WordDocument" in UTF-16LE somewhere in the data
        let word_doc_utf16 = to_utf16le(b"WordDocument");
        data[512..512 + word_doc_utf16.len()].copy_from_slice(&word_doc_utf16);
        
        let result = detect_ole2_type(&data);
        assert!(result.is_some());
        let (mime_type, confidence) = result.unwrap();
        assert_eq!(mime_type, "application/msword");
        assert_eq!(confidence, 0.90);
    }
    
    #[test]
    fn test_detect_ole2_type_with_workbook() {
        // Create a minimal OLE2 file with Workbook stream name
        let mut data = vec![0u8; 1024];
        // OLE2 header
        data[0..8].copy_from_slice(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1");
        // Add "Workbook" in UTF-16LE somewhere in the data
        let workbook_utf16 = to_utf16le(b"Workbook");
        data[512..512 + workbook_utf16.len()].copy_from_slice(&workbook_utf16);
        
        let result = detect_ole2_type(&data);
        assert!(result.is_some());
        let (mime_type, confidence) = result.unwrap();
        assert_eq!(mime_type, "application/vnd.ms-excel");
        assert_eq!(confidence, 0.90);
    }
    
    #[test]
    fn test_detect_ole2_type_with_powerpoint() {
        // Create a minimal OLE2 file with PowerPoint Document stream name
        let mut data = vec![0u8; 1024];
        // OLE2 header
        data[0..8].copy_from_slice(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1");
        // Add "Current User" in UTF-16LE somewhere in the data (PPT-specific)
        let current_user_utf16 = to_utf16le(b"Current User");
        data[512..512 + current_user_utf16.len()].copy_from_slice(&current_user_utf16);
        
        let result = detect_ole2_type(&data);
        assert!(result.is_some());
        let (mime_type, confidence) = result.unwrap();
        assert_eq!(mime_type, "application/vnd.ms-powerpoint");
        assert_eq!(confidence, 0.90);
    }
    
    #[test]
    fn test_detect_ole2_type_invalid_header() {
        let data = vec![0u8; 512];
        let result = detect_ole2_type(&data);
        assert!(result.is_none());
    }
    
    #[test]
    fn test_detect_ole2_type_too_short() {
        let data = vec![0xD0, 0xCF, 0x11, 0xE0];
        let result = detect_ole2_type(&data);
        assert!(result.is_none());
    }
    
    #[test]
    fn test_to_utf16le() {
        let ascii = b"Test";
        let utf16le = to_utf16le(ascii);
        assert_eq!(utf16le, vec![b'T', 0, b'e', 0, b's', 0, b't', 0]);
    }
    
    #[test]
    fn test_search_bytes() {
        let data = b"Hello World";
        assert!(search_bytes(data, b"World"));
        assert!(search_bytes(data, b"Hello"));
        assert!(!search_bytes(data, b"Goodbye"));
        assert!(!search_bytes(data, b""));
    }
    
    #[test]
    fn test_contains_stream_name() {
        let mut data = vec![0u8; 1024];
        // Add "WordDocument" in UTF-16LE
        let word_doc_utf16 = to_utf16le(b"WordDocument");
        data[100..100 + word_doc_utf16.len()].copy_from_slice(&word_doc_utf16);
        
        assert!(contains_stream_name(&data, b"WordDocument"));
        assert!(!contains_stream_name(&data, b"Workbook"));
    }
}
