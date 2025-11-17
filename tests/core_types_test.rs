//! Unit tests for core types
//!
//! Tests error types, metadata operations, and extraction result construction.

use omniparse::core::{Content, Error, ExtractionResult, Metadata, MetadataValue};
use chrono::{TimeZone, Utc};
use std::io;

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_io_error_creation() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let error = Error::from(io_err);
        
        assert!(matches!(error, Error::Io(_)));
        assert!(error.to_string().contains("IO error"));
    }

    #[test]
    fn test_unsupported_format_error() {
        let error = Error::UnsupportedFormat("application/x-unknown".to_string());
        
        assert!(matches!(error, Error::UnsupportedFormat(_)));
        assert!(error.to_string().contains("Unsupported format"));
        assert!(error.to_string().contains("application/x-unknown"));
    }

    #[test]
    fn test_corrupted_file_error() {
        let error = Error::CorruptedFile("Invalid header".to_string());
        
        assert!(matches!(error, Error::CorruptedFile(_)));
        assert!(error.to_string().contains("Corrupted file"));
        assert!(error.to_string().contains("Invalid header"));
    }

    #[test]
    fn test_parse_error() {
        let error = Error::ParseError("Unexpected token".to_string());
        
        assert!(matches!(error, Error::ParseError(_)));
        assert!(error.to_string().contains("Parse error"));
        assert!(error.to_string().contains("Unexpected token"));
    }

    #[test]
    fn test_detection_failed_error() {
        let error = Error::DetectionFailed("No magic bytes matched".to_string());
        
        assert!(matches!(error, Error::DetectionFailed(_)));
        assert!(error.to_string().contains("Detection failed"));
        assert!(error.to_string().contains("No magic bytes matched"));
    }

    #[test]
    fn test_partial_extraction_error() {
        let partial_result = ExtractionResult {
            mime_type: "text/plain".to_string(),
            content: Content::Text("Partial content".to_string()),
            metadata: Metadata::new(),
            detection_confidence: 0.8,
        };
        
        let error = Error::PartialExtraction {
            message: "Could not extract all pages".to_string(),
            partial_result: Box::new(partial_result),
        };
        
        assert!(matches!(error, Error::PartialExtraction { .. }));
        assert!(error.to_string().contains("Partial extraction"));
        assert!(error.to_string().contains("Could not extract all pages"));
        
        // Verify we can access the partial result
        if let Error::PartialExtraction { partial_result, .. } = error {
            assert_eq!(partial_result.mime_type, "text/plain");
            match &partial_result.content {
                Content::Text(text) => assert_eq!(text, "Partial content"),
                _ => panic!("Expected text content"),
            }
        }
    }

    #[test]
    fn test_error_conversion_from_io() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let error: Error = io_err.into();
        
        assert!(matches!(error, Error::Io(_)));
    }
}

#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    fn test_metadata_new() {
        let metadata = Metadata::new();
        assert_eq!(metadata.keys().count(), 0);
    }

    #[test]
    fn test_metadata_insert_and_get() {
        let mut metadata = Metadata::new();
        
        metadata.insert("title".to_string(), MetadataValue::Text("Test Document".to_string()));
        
        assert_eq!(metadata.keys().count(), 1);
        assert!(metadata.get("title").is_some());
        assert_eq!(
            metadata.get("title"),
            Some(&MetadataValue::Text("Test Document".to_string()))
        );
    }

    #[test]
    fn test_metadata_get_nonexistent() {
        let metadata = Metadata::new();
        assert!(metadata.get("nonexistent").is_none());
    }

    #[test]
    fn test_metadata_insert_replaces_existing() {
        let mut metadata = Metadata::new();
        
        metadata.insert("count".to_string(), MetadataValue::Number(10));
        metadata.insert("count".to_string(), MetadataValue::Number(20));
        
        assert_eq!(metadata.keys().count(), 1);
        assert_eq!(metadata.get("count"), Some(&MetadataValue::Number(20)));
    }

    #[test]
    fn test_metadata_multiple_fields() {
        let mut metadata = Metadata::new();
        
        metadata.insert("title".to_string(), MetadataValue::Text("Doc".to_string()));
        metadata.insert("page_count".to_string(), MetadataValue::Number(42));
        metadata.insert("ratio".to_string(), MetadataValue::Float(1.5));
        metadata.insert("published".to_string(), MetadataValue::Boolean(true));
        
        assert_eq!(metadata.keys().count(), 4);
        assert!(metadata.get("title").is_some());
        assert!(metadata.get("page_count").is_some());
        assert!(metadata.get("ratio").is_some());
        assert!(metadata.get("published").is_some());
    }

    #[test]
    fn test_metadata_title_accessor() {
        let mut metadata = Metadata::new();
        
        // No title initially
        assert!(metadata.title().is_none());
        
        // Add title
        metadata.insert("title".to_string(), MetadataValue::Text("My Document".to_string()));
        assert_eq!(metadata.title(), Some("My Document"));
        
        // Wrong type should return None
        metadata.insert("title".to_string(), MetadataValue::Number(123));
        assert!(metadata.title().is_none());
    }

    #[test]
    fn test_metadata_author_accessor() {
        let mut metadata = Metadata::new();
        
        // No author initially
        assert!(metadata.author().is_none());
        
        // Add author
        metadata.insert("author".to_string(), MetadataValue::Text("Alice".to_string()));
        assert_eq!(metadata.author(), Some("Alice"));
        
        // Wrong type should return None
        metadata.insert("author".to_string(), MetadataValue::Boolean(true));
        assert!(metadata.author().is_none());
    }

    #[test]
    fn test_metadata_created_accessor() {
        let mut metadata = Metadata::new();
        
        // No created date initially
        assert!(metadata.created().is_none());
        
        // Add created date
        let date = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        metadata.insert("created".to_string(), MetadataValue::DateTime(date));
        assert_eq!(metadata.created(), Some(date));
        
        // Wrong type should return None
        metadata.insert("created".to_string(), MetadataValue::Text("2024-01-15".to_string()));
        assert!(metadata.created().is_none());
    }

    #[test]
    fn test_metadata_modified_accessor() {
        let mut metadata = Metadata::new();
        
        // No modified date initially
        assert!(metadata.modified().is_none());
        
        // Add modified date
        let date = Utc.with_ymd_and_hms(2024, 2, 20, 14, 45, 30).unwrap();
        metadata.insert("modified".to_string(), MetadataValue::DateTime(date));
        assert_eq!(metadata.modified(), Some(date));
        
        // Wrong type should return None
        metadata.insert("modified".to_string(), MetadataValue::Number(20240220));
        assert!(metadata.modified().is_none());
    }

    #[test]
    fn test_metadata_keys_iterator() {
        let mut metadata = Metadata::new();
        
        metadata.insert("key1".to_string(), MetadataValue::Text("value1".to_string()));
        metadata.insert("key2".to_string(), MetadataValue::Number(42));
        metadata.insert("key3".to_string(), MetadataValue::Boolean(true));
        
        let keys: Vec<_> = metadata.keys().collect();
        assert_eq!(keys.len(), 3);
        
        // Check all keys are present (order doesn't matter with HashMap)
        let key_strings: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
        assert!(key_strings.contains(&"key1".to_string()));
        assert!(key_strings.contains(&"key2".to_string()));
        assert!(key_strings.contains(&"key3".to_string()));
    }

    #[test]
    fn test_metadata_default() {
        let metadata = Metadata::default();
        assert_eq!(metadata.keys().count(), 0);
    }
}

#[cfg(test)]
mod metadata_value_tests {
    use super::*;

    #[test]
    fn test_metadata_value_text() {
        let value = MetadataValue::Text("Hello".to_string());
        assert!(matches!(value, MetadataValue::Text(_)));
    }

    #[test]
    fn test_metadata_value_number() {
        let value = MetadataValue::Number(42);
        assert!(matches!(value, MetadataValue::Number(42)));
    }

    #[test]
    fn test_metadata_value_float() {
        let value = MetadataValue::Float(3.14);
        assert!(matches!(value, MetadataValue::Float(_)));
    }

    #[test]
    fn test_metadata_value_datetime() {
        let date = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let value = MetadataValue::DateTime(date);
        assert!(matches!(value, MetadataValue::DateTime(_)));
    }

    #[test]
    fn test_metadata_value_boolean() {
        let value = MetadataValue::Boolean(true);
        assert!(matches!(value, MetadataValue::Boolean(true)));
    }

    #[test]
    fn test_metadata_value_list() {
        let list = vec![
            MetadataValue::Text("item1".to_string()),
            MetadataValue::Text("item2".to_string()),
        ];
        let value = MetadataValue::List(list);
        assert!(matches!(value, MetadataValue::List(_)));
        
        if let MetadataValue::List(items) = value {
            assert_eq!(items.len(), 2);
        }
    }

    #[test]
    fn test_metadata_value_equality() {
        let v1 = MetadataValue::Text("test".to_string());
        let v2 = MetadataValue::Text("test".to_string());
        let v3 = MetadataValue::Text("other".to_string());
        
        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }
}

#[cfg(test)]
mod content_tests {
    use super::*;

    #[test]
    fn test_content_text() {
        let content = Content::Text("Hello, world!".to_string());
        
        match content {
            Content::Text(text) => assert_eq!(text, "Hello, world!"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_content_binary() {
        let data = vec![0x89, 0x50, 0x4E, 0x47];
        let content = Content::Binary(data.clone());
        
        match content {
            Content::Binary(bytes) => assert_eq!(bytes, data),
            _ => panic!("Expected binary content"),
        }
    }

    #[test]
    fn test_content_none() {
        let content = Content::None;
        assert!(matches!(content, Content::None));
    }
}

#[cfg(test)]
mod extraction_result_tests {
    use super::*;

    #[test]
    fn test_extraction_result_construction() {
        let mut metadata = Metadata::new();
        metadata.insert("title".to_string(), MetadataValue::Text("Test".to_string()));
        
        let result = ExtractionResult {
            mime_type: "text/plain".to_string(),
            content: Content::Text("Sample text".to_string()),
            metadata,
            detection_confidence: 0.95,
        };
        
        assert_eq!(result.mime_type, "text/plain");
        assert_eq!(result.detection_confidence, 0.95);
        assert!(matches!(result.content, Content::Text(_)));
        assert_eq!(result.metadata.title(), Some("Test"));
    }

    #[test]
    fn test_extraction_result_with_binary_content() {
        let result = ExtractionResult {
            mime_type: "image/png".to_string(),
            content: Content::Binary(vec![1, 2, 3, 4]),
            metadata: Metadata::new(),
            detection_confidence: 1.0,
        };
        
        assert_eq!(result.mime_type, "image/png");
        match result.content {
            Content::Binary(data) => assert_eq!(data.len(), 4),
            _ => panic!("Expected binary content"),
        }
    }

    #[test]
    fn test_extraction_result_with_no_content() {
        let mut metadata = Metadata::new();
        metadata.insert("width".to_string(), MetadataValue::Number(1920));
        metadata.insert("height".to_string(), MetadataValue::Number(1080));
        
        let result = ExtractionResult {
            mime_type: "image/jpeg".to_string(),
            content: Content::None,
            metadata,
            detection_confidence: 0.9,
        };
        
        assert_eq!(result.mime_type, "image/jpeg");
        assert!(matches!(result.content, Content::None));
        assert_eq!(result.metadata.get("width"), Some(&MetadataValue::Number(1920)));
        assert_eq!(result.metadata.get("height"), Some(&MetadataValue::Number(1080)));
    }

    #[test]
    fn test_extraction_result_with_full_metadata() {
        let mut metadata = Metadata::new();
        metadata.insert("title".to_string(), MetadataValue::Text("Document".to_string()));
        metadata.insert("author".to_string(), MetadataValue::Text("Alice".to_string()));
        
        let created = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let modified = Utc.with_ymd_and_hms(2024, 2, 1, 15, 30, 0).unwrap();
        
        metadata.insert("created".to_string(), MetadataValue::DateTime(created));
        metadata.insert("modified".to_string(), MetadataValue::DateTime(modified));
        metadata.insert("page_count".to_string(), MetadataValue::Number(10));
        
        let result = ExtractionResult {
            mime_type: "application/pdf".to_string(),
            content: Content::Text("PDF content".to_string()),
            metadata,
            detection_confidence: 0.98,
        };
        
        assert_eq!(result.metadata.title(), Some("Document"));
        assert_eq!(result.metadata.author(), Some("Alice"));
        assert_eq!(result.metadata.created(), Some(created));
        assert_eq!(result.metadata.modified(), Some(modified));
        assert_eq!(result.metadata.get("page_count"), Some(&MetadataValue::Number(10)));
    }

    #[test]
    fn test_extraction_result_confidence_range() {
        let result_low = ExtractionResult {
            mime_type: "text/plain".to_string(),
            content: Content::Text("text".to_string()),
            metadata: Metadata::new(),
            detection_confidence: 0.3,
        };
        
        let result_high = ExtractionResult {
            mime_type: "application/pdf".to_string(),
            content: Content::Text("pdf".to_string()),
            metadata: Metadata::new(),
            detection_confidence: 1.0,
        };
        
        assert!(result_low.detection_confidence >= 0.0);
        assert!(result_low.detection_confidence <= 1.0);
        assert!(result_high.detection_confidence >= 0.0);
        assert!(result_high.detection_confidence <= 1.0);
    }
}
