//! RTF parser for extracting plain text from Rich Text Format documents

use crate::core::{Content, ExtractionResult, Metadata, MetadataValue, Result, Error};
use crate::parsers::Parser;
use crate::utils::security::{validate_file_size, FileSizeLimits};

/// Parser for RTF (Rich Text Format) documents
pub struct RtfParser;

impl RtfParser {
    /// Strip RTF control words and extract plain text
    fn strip_control_words(rtf: &str) -> String {
        let mut result = String::new();
        let mut chars = rtf.chars().peekable();
        let mut in_control_word = false;
        let mut brace_depth = 0;
        let mut skip_depth = Vec::new(); // Track depths where we should skip
        
        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    brace_depth += 1;
                    // Check if this is a group we should skip
                    if let Some(&'\\') = chars.peek() {
                        let mut temp_chars = chars.clone();
                        temp_chars.next(); // skip '\'
                        
                        // Check for various groups to skip
                        let next_word = Self::peek_control_word(&mut temp_chars);
                        match next_word.as_str() {
                            "*" | "info" | "fonttbl" | "colortbl" | "stylesheet" | 
                            "title" | "author" | "subject" | "creatim" => {
                                skip_depth.push(brace_depth);
                            }
                            _ => {}
                        }
                    }
                }
                '}' => {
                    // Check if we're exiting a skip group
                    if let Some(&depth) = skip_depth.last() {
                        if depth == brace_depth {
                            skip_depth.pop();
                        }
                    }
                    
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    }
                }
                '\\' => {
                    // Skip everything if we're in a skip group
                    if !skip_depth.is_empty() {
                        continue;
                    }
                    
                    in_control_word = true;
                    
                    // Check for special characters
                    if let Some(&next_ch) = chars.peek() {
                        match next_ch {
                            '\'' => {
                                // Hex encoded character: \'XX
                                chars.next(); // consume '
                                let hex1 = chars.next();
                                let hex2 = chars.next();
                                
                                if let (Some(h1), Some(h2)) = (hex1, hex2) {
                                    let hex_str = format!("{}{}", h1, h2);
                                    if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                                        // Try to decode as Windows-1252 or Latin-1
                                        result.push(byte as char);
                                    }
                                }
                                in_control_word = false;
                            }
                            '\\' | '{' | '}' => {
                                // Escaped special characters
                                chars.next();
                                result.push(next_ch);
                                in_control_word = false;
                            }
                            '\n' | '\r' => {
                                // Control symbol followed by newline
                                chars.next();
                                in_control_word = false;
                            }
                            '*' => {
                                // Skip control symbol - just consume it
                                chars.next();
                                in_control_word = false;
                            }
                            _ => {
                                // Regular control word - consume until space or non-alphanumeric
                                let mut control_word = String::new();
                                while let Some(&c) = chars.peek() {
                                    if c.is_alphanumeric() || c == '-' {
                                        control_word.push(c);
                                        chars.next();
                                    } else {
                                        break;
                                    }
                                }
                                
                                // Check for control words that should add text
                                match control_word.as_str() {
                                    "par" | "line" => result.push('\n'),
                                    "tab" => result.push('\t'),
                                    _ => {}
                                }
                                
                                // Skip optional space after control word
                                if let Some(&' ') = chars.peek() {
                                    chars.next();
                                }
                                
                                in_control_word = false;
                            }
                        }
                    }
                }
                _ if !in_control_word && skip_depth.is_empty() && brace_depth > 0 => {
                    // Regular text character
                    if ch != '\r' && ch != '\n' {
                        result.push(ch);
                    }
                }
                _ => {}
            }
        }
        
        result.trim().to_string()
    }
    
    /// Peek at the next control word without consuming characters
    fn peek_control_word(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
        let mut word = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_alphanumeric() || c == '*' {
                word.push(c);
                chars.next();
            } else {
                break;
            }
        }
        word
    }
    
    /// Extract metadata from RTF document
    fn extract_metadata(rtf: &str) -> Metadata {
        let mut metadata = Metadata::new();
        
        // Extract RTF version
        if let Some(version_pos) = rtf.find(r"{\rtf") {
            let version_str = &rtf[version_pos + 5..];
            if let Some(first_char) = version_str.chars().next() {
                if first_char.is_numeric() {
                    metadata.insert("rtf_version".to_string(), MetadataValue::Text(first_char.to_string()));
                }
            }
        }
        
        // Extract info group metadata
        if let Some(info_start) = rtf.find(r"{\info") {
            let info_section = &rtf[info_start..];
            
            // Find the end of the info group (matching closing brace)
            let mut brace_count = 0;
            let mut info_end = 0;
            for (i, ch) in info_section.chars().enumerate() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => {
                        brace_count -= 1;
                        if brace_count == 0 {
                            info_end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            
            if info_end > 0 {
                let info_content = &info_section[..info_end];
                
                // Extract title
                if let Some(title) = Self::extract_info_field(info_content, "title") {
                    metadata.insert("title".to_string(), MetadataValue::Text(title));
                }
                
                // Extract author
                if let Some(author) = Self::extract_info_field(info_content, "author") {
                    metadata.insert("author".to_string(), MetadataValue::Text(author));
                }
                
                // Extract subject
                if let Some(subject) = Self::extract_info_field(info_content, "subject") {
                    metadata.insert("subject".to_string(), MetadataValue::Text(subject));
                }
                
                // Extract creation date
                if let Some(creatim_start) = info_content.find(r"{\creatim") {
                    let creatim_section = &info_content[creatim_start..];
                    if let Some(date) = Self::extract_rtf_date(creatim_section) {
                        metadata.insert("creation_date".to_string(), MetadataValue::Text(date));
                    }
                }
            }
        }
        
        // Extract character encoding (default to Windows-1252 for RTF)
        metadata.insert("encoding".to_string(), MetadataValue::Text("Windows-1252".to_string()));
        
        metadata
    }
    
    /// Extract a field from the info group
    fn extract_info_field(info_content: &str, field_name: &str) -> Option<String> {
        let pattern = format!(r"{{\{}", field_name);
        if let Some(field_start) = info_content.find(&pattern) {
            let field_section = &info_content[field_start + pattern.len()..];
            
            // Find the closing brace
            let mut brace_count = 1;
            let mut field_end = 0;
            for (i, ch) in field_section.chars().enumerate() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => {
                        brace_count -= 1;
                        if brace_count == 0 {
                            field_end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            
            if field_end > 0 {
                let field_value = &field_section[..field_end].trim();
                // Strip any remaining control words from the field value
                let cleaned = Self::strip_control_words(&format!("{{\\rtf1 {}}}", field_value));
                if !cleaned.is_empty() {
                    return Some(cleaned);
                }
            }
        }
        None
    }
    
    /// Extract RTF date from creatim group
    fn extract_rtf_date(creatim_section: &str) -> Option<String> {
        // RTF dates are in format: {\creatim\yr2024\mo1\dy15\hr10\min30}
        let mut year = None;
        let mut month = None;
        let mut day = None;
        
        // Extract year
        if let Some(yr_pos) = creatim_section.find(r"\yr") {
            let yr_str = &creatim_section[yr_pos + 3..];
            if let Some(num_end) = yr_str.find(|c: char| !c.is_numeric()) {
                if let Ok(y) = yr_str[..num_end].parse::<i32>() {
                    year = Some(y);
                }
            }
        }
        
        // Extract month
        if let Some(mo_pos) = creatim_section.find(r"\mo") {
            let mo_str = &creatim_section[mo_pos + 3..];
            if let Some(num_end) = mo_str.find(|c: char| !c.is_numeric()) {
                if let Ok(m) = mo_str[..num_end].parse::<u32>() {
                    month = Some(m);
                }
            }
        }
        
        // Extract day
        if let Some(dy_pos) = creatim_section.find(r"\dy") {
            let dy_str = &creatim_section[dy_pos + 3..];
            if let Some(num_end) = dy_str.find(|c: char| !c.is_numeric()) {
                if let Ok(d) = dy_str[..num_end].parse::<u32>() {
                    day = Some(d);
                }
            }
        }
        
        // Format date if we have all components
        if let (Some(y), Some(m), Some(d)) = (year, month, day) {
            return Some(format!("{:04}-{:02}-{:02}", y, m, d));
        }
        
        None
    }
}

impl Parser for RtfParser {
    fn name(&self) -> &str {
        "RtfParser"
    }

    fn supported_types(&self) -> &[&str] {
        &["application/rtf"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Validate file size
        validate_file_size(data, FileSizeLimits::RTF, "RTF")?;
        
        // Convert to string
        let rtf_content = String::from_utf8_lossy(data);
        
        // Validate RTF header
        if !rtf_content.starts_with(r"{\rtf1") && !rtf_content.starts_with(r"{\rtf") {
            return Err(Error::ParseError("Invalid RTF header".to_string()));
        }
        
        // Extract metadata
        let metadata = Self::extract_metadata(&rtf_content);
        
        // Strip control words to get plain text
        let plain_text = Self::strip_control_words(&rtf_content);
        
        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(plain_text),
            metadata,
            detection_confidence: 0.90,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtf_parser_name() {
        let parser = RtfParser;
        assert_eq!(parser.name(), "RtfParser");
    }

    #[test]
    fn test_rtf_parser_supported_types() {
        let parser = RtfParser;
        assert_eq!(parser.supported_types(), &["application/rtf"]);
    }
}
