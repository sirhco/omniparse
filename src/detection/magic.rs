//! Magic byte patterns for file type detection

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

/// Get the default magic byte patterns database
pub fn get_magic_patterns() -> Vec<MagicPattern> {
    vec![
        // Document formats
        MagicPattern::new("application/pdf", 0, b"%PDF-".to_vec()),
        MagicPattern::new("application/vnd.openxmlformats-officedocument.wordprocessingml.document", 0, 
            vec![0x50, 0x4B, 0x03, 0x04]), // ZIP signature (DOCX is ZIP-based)
        MagicPattern::new("application/vnd.oasis.opendocument.text", 0, 
            vec![0x50, 0x4B, 0x03, 0x04]), // ZIP signature (ODT is ZIP-based)
        MagicPattern::new("application/msword", 0, 
            vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]), // OLE2 signature
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
        MagicPattern::new("image/svg+xml", 0, b"<?xml".to_vec()),
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
}
