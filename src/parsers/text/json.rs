//! JSON parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use serde_json::Value;

/// Parser for JSON files
pub struct JsonParser;

impl JsonParser {
    /// Analyze JSON structure and extract schema information
    fn analyze_structure(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(_) => "boolean".to_string(),
            Value::Number(_) => "number".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Array(arr) => {
                if arr.is_empty() {
                    "array[]".to_string()
                } else {
                    let first_type = Self::analyze_structure(&arr[0]);
                    format!("array[{}]", first_type)
                }
            }
            Value::Object(obj) => {
                let keys: Vec<String> = obj.keys().cloned().collect();
                format!("object{{{}}}", keys.join(", "))
            }
        }
    }
    
    /// Convert JSON value to a readable text representation
    fn value_to_text(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Array(arr) => {
                arr.iter()
                    .map(Self::value_to_text)
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Value::Object(obj) => {
                obj.iter()
                    .map(|(k, v)| format!("{}: {}", k, Self::value_to_text(v)))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            _ => value.to_string(),
        }
    }
}

impl Parser for JsonParser {
    fn supported_types(&self) -> &[&str] {
        &["application/json", "text/json"]
    }
    
    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Parse JSON
        let json_value: Value = serde_json::from_slice(data)
            .map_err(|e| Error::ParseError(format!("Invalid JSON: {}", e)))?;
        
        // Extract text representation
        let text = Self::value_to_text(&json_value);
        
        // Analyze structure
        let schema_info = Self::analyze_structure(&json_value);
        
        // Build metadata
        let mut metadata = Metadata::new();
        metadata.insert("valid".to_string(), MetadataValue::Boolean(true));
        metadata.insert("schema_info".to_string(), MetadataValue::Text(schema_info));
        
        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata,
            detection_confidence: 0.0, // Will be set by the extractor
        })
    }
    
    fn name(&self) -> &str {
        "JsonParser"
    }
}
