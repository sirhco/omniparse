//! Extraction result types
//!
//! This module defines the data structures returned by extraction operations.
//! The main type is `ExtractionResult`, which contains the detected MIME type,
//! extracted content, metadata, and detection confidence.
//!
//! # Examples
//!
//! ```no_run
//! use omniparse::{extract_from_path, Content};
//!
//! let result = extract_from_path("document.pdf")?;
//!
//! // Access the MIME type
//! println!("Type: {}", result.mime_type);
//!
//! // Access the content
//! match result.content {
//!     Content::Text(text) => println!("Text: {}", text),
//!     Content::Binary(data) => println!("Binary data: {} bytes", data.len()),
//!     Content::None => println!("No content extracted"),
//! }
//!
//! // Access metadata
//! if let Some(title) = result.metadata.title() {
//!     println!("Title: {}", title);
//! }
//! # Ok::<(), omniparse::Error>(())
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of a content extraction operation
///
/// This structure contains all information extracted from a file, including
/// the detected MIME type, content, metadata, and confidence score.
///
/// # Examples
///
/// ```no_run
/// use omniparse::extract_from_path;
///
/// let result = extract_from_path("document.pdf")?;
/// println!("MIME type: {}", result.mime_type);
/// println!("Confidence: {:.2}", result.detection_confidence);
/// println!("Metadata fields: {}", result.metadata.keys().count());
/// # Ok::<(), omniparse::Error>(())
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Detected MIME type of the file
    ///
    /// This is the MIME type determined by the type detection system,
    /// such as "application/pdf" or "text/plain".
    pub mime_type: String,
    
    /// Extracted content
    ///
    /// The actual content extracted from the file. This can be text,
    /// binary data, or none if only metadata was extracted.
    pub content: Content,
    
    /// Extracted metadata
    ///
    /// Structured metadata fields extracted from the file, such as
    /// title, author, creation date, and format-specific properties.
    pub metadata: Metadata,
    
    /// Confidence score of type detection (0.0 to 1.0)
    ///
    /// Higher values indicate more confident detection. Magic byte detection
    /// typically yields 0.9-1.0, content analysis 0.6-0.8, and extension-only
    /// detection 0.3-0.5.
    pub detection_confidence: f32,
}

/// Extracted content variants
///
/// Represents the different types of content that can be extracted from files.
/// Most text-based formats will produce `Content::Text`, while binary formats
/// like images may produce `Content::Binary` or `Content::None` with metadata only.
///
/// # Examples
///
/// ```no_run
/// use omniparse::{extract_from_path, Content};
///
/// let result = extract_from_path("file.txt")?;
///
/// match result.content {
///     Content::Text(text) => {
///         println!("Extracted {} characters", text.len());
///     }
///     Content::Binary(data) => {
///         println!("Binary data: {} bytes", data.len());
///     }
///     Content::None => {
///         println!("No content, metadata only");
///     }
/// }
/// # Ok::<(), omniparse::Error>(())
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Content {
    /// Text content
    ///
    /// UTF-8 encoded text extracted from the file. This is the most common
    /// variant for document and text formats.
    Text(String),
    
    /// Binary content
    ///
    /// Raw binary data. This is used when the content cannot be represented
    /// as text or when binary data is specifically requested.
    Binary(Vec<u8>),
    
    /// No content extracted
    ///
    /// Used when only metadata is available or when content extraction
    /// is not applicable for the file type (e.g., some image formats).
    None,
}

/// Metadata extracted from a file
///
/// A collection of key-value pairs representing metadata fields.
/// Common fields include title, author, creation date, and format-specific
/// properties. The metadata structure provides convenient accessor methods
/// for common fields.
///
/// # Examples
///
/// ```
/// use omniparse::{Metadata, MetadataValue};
///
/// let mut metadata = Metadata::new();
/// metadata.insert("title".to_string(), MetadataValue::Text("My Document".to_string()));
/// metadata.insert("page_count".to_string(), MetadataValue::Number(42));
///
/// assert_eq!(metadata.title(), Some("My Document"));
/// assert_eq!(metadata.get("page_count"), Some(&MetadataValue::Number(42)));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    fields: HashMap<String, MetadataValue>,
}

/// Possible metadata value types
///
/// Metadata values can be of various types to accommodate different kinds
/// of information. This enum provides type-safe representation of metadata.
///
/// # Examples
///
/// ```
/// use omniparse::MetadataValue;
/// use chrono::Utc;
///
/// let text_value = MetadataValue::Text("Example".to_string());
/// let number_value = MetadataValue::Number(42);
/// let float_value = MetadataValue::Float(3.14);
/// let date_value = MetadataValue::DateTime(Utc::now());
/// let bool_value = MetadataValue::Boolean(true);
/// let list_value = MetadataValue::List(vec![
///     MetadataValue::Text("item1".to_string()),
///     MetadataValue::Text("item2".to_string()),
/// ]);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetadataValue {
    /// Text value
    ///
    /// A string value, used for fields like title, author, description, etc.
    Text(String),
    
    /// Integer number
    ///
    /// A 64-bit signed integer, used for counts, page numbers, etc.
    Number(i64),
    
    /// Floating point number
    ///
    /// A 64-bit floating point number, used for measurements, ratios, etc.
    Float(f64),
    
    /// Date and time
    ///
    /// A UTC timestamp, used for creation dates, modification dates, etc.
    DateTime(DateTime<Utc>),
    
    /// Boolean value
    ///
    /// A true/false value, used for flags and binary properties.
    Boolean(bool),
    
    /// List of values
    ///
    /// A list of metadata values, used for multi-valued fields like keywords or authors.
    List(Vec<MetadataValue>),
}

impl Metadata {
    /// Create a new empty Metadata instance
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::Metadata;
    ///
    /// let metadata = Metadata::new();
    /// assert_eq!(metadata.keys().count(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }
    
    /// Get a metadata value by key
    ///
    /// Returns `None` if the key doesn't exist in the metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::{Metadata, MetadataValue};
    ///
    /// let mut metadata = Metadata::new();
    /// metadata.insert("title".to_string(), MetadataValue::Text("Example".to_string()));
    ///
    /// assert!(metadata.get("title").is_some());
    /// assert!(metadata.get("nonexistent").is_none());
    /// ```
    pub fn get(&self, key: &str) -> Option<&MetadataValue> {
        self.fields.get(key)
    }
    
    /// Insert a metadata value
    ///
    /// If the key already exists, the old value is replaced.
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::{Metadata, MetadataValue};
    ///
    /// let mut metadata = Metadata::new();
    /// metadata.insert("page_count".to_string(), MetadataValue::Number(10));
    /// metadata.insert("page_count".to_string(), MetadataValue::Number(20));
    ///
    /// assert_eq!(metadata.get("page_count"), Some(&MetadataValue::Number(20)));
    /// ```
    pub fn insert(&mut self, key: String, value: MetadataValue) {
        self.fields.insert(key, value);
    }
    
    /// Get an iterator over all metadata keys
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::{Metadata, MetadataValue};
    ///
    /// let mut metadata = Metadata::new();
    /// metadata.insert("title".to_string(), MetadataValue::Text("Doc".to_string()));
    /// metadata.insert("author".to_string(), MetadataValue::Text("Alice".to_string()));
    ///
    /// let keys: Vec<_> = metadata.keys().collect();
    /// assert_eq!(keys.len(), 2);
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.fields.keys()
    }
    
    /// Get the title metadata if present
    ///
    /// This is a convenience method that extracts the "title" field if it exists
    /// and is a text value.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use omniparse::extract_from_path;
    ///
    /// let result = extract_from_path("document.pdf")?;
    /// if let Some(title) = result.metadata.title() {
    ///     println!("Document title: {}", title);
    /// }
    /// # Ok::<(), omniparse::Error>(())
    /// ```
    pub fn title(&self) -> Option<&str> {
        match self.get("title") {
            Some(MetadataValue::Text(s)) => Some(s.as_str()),
            _ => None,
        }
    }
    
    /// Get the author metadata if present
    ///
    /// This is a convenience method that extracts the "author" field if it exists
    /// and is a text value.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use omniparse::extract_from_path;
    ///
    /// let result = extract_from_path("document.pdf")?;
    /// if let Some(author) = result.metadata.author() {
    ///     println!("Author: {}", author);
    /// }
    /// # Ok::<(), omniparse::Error>(())
    /// ```
    pub fn author(&self) -> Option<&str> {
        match self.get("author") {
            Some(MetadataValue::Text(s)) => Some(s.as_str()),
            _ => None,
        }
    }
    
    /// Get the creation date metadata if present
    ///
    /// This is a convenience method that extracts the "created" field if it exists
    /// and is a DateTime value.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use omniparse::extract_from_path;
    ///
    /// let result = extract_from_path("document.pdf")?;
    /// if let Some(created) = result.metadata.created() {
    ///     println!("Created: {}", created);
    /// }
    /// # Ok::<(), omniparse::Error>(())
    /// ```
    pub fn created(&self) -> Option<DateTime<Utc>> {
        match self.get("created") {
            Some(MetadataValue::DateTime(dt)) => Some(*dt),
            _ => None,
        }
    }
    
    /// Get the modification date metadata if present
    ///
    /// This is a convenience method that extracts the "modified" field if it exists
    /// and is a DateTime value.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use omniparse::extract_from_path;
    ///
    /// let result = extract_from_path("document.pdf")?;
    /// if let Some(modified) = result.metadata.modified() {
    ///     println!("Last modified: {}", modified);
    /// }
    /// # Ok::<(), omniparse::Error>(())
    /// ```
    pub fn modified(&self) -> Option<DateTime<Utc>> {
        match self.get("modified") {
            Some(MetadataValue::DateTime(dt)) => Some(*dt),
            _ => None,
        }
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new()
    }
}
