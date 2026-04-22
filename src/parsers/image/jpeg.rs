//! JPEG image parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::parsers::image::exif::extract_exif_fields;
use image::io::Reader as ImageReader;
use std::io::Cursor;

/// Parser for JPEG images
pub struct JpegParser;

impl Parser for JpegParser {
    fn supported_types(&self) -> &[&str] {
        &["image/jpeg", "image/jpg"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| Error::ParseError(format!("Failed to read JPEG: {}", e)))?
            .decode()
            .map_err(|e| Error::ParseError(format!("Failed to decode JPEG: {}", e)))?;

        let mut metadata = Metadata::new();
        metadata.insert("width".to_string(), MetadataValue::Number(img.width() as i64));
        metadata.insert("height".to_string(), MetadataValue::Number(img.height() as i64));
        metadata.insert(
            "color_type".to_string(),
            MetadataValue::Text(format!("{:?}", img.color())),
        );

        for (key, value) in extract_exif_fields(data) {
            metadata.insert(key, value);
        }

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::None,
            metadata,
            detection_confidence: 0.0,
        })
    }

    fn name(&self) -> &str {
        "JpegParser"
    }
}
