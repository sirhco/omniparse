//! WebP image parser. Extracts dimensions, color type, and any embedded EXIF.

use crate::core::{Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use crate::parsers::image::exif::extract_exif_fields;
use crate::parsers::image::maybe_ocr_content;
use image::io::Reader as ImageReader;
use std::io::Cursor;

pub struct WebpParser;

impl Parser for WebpParser {
    fn name(&self) -> &str {
        "WebpParser"
    }

    fn supported_types(&self) -> &[&str] {
        &["image/webp"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| Error::ParseError(format!("Failed to read WebP: {}", e)))?
            .decode()
            .map_err(|e| Error::ParseError(format!("Failed to decode WebP: {}", e)))?;

        let mut metadata = Metadata::new();
        metadata.insert("width".into(), MetadataValue::Number(img.width() as i64));
        metadata.insert("height".into(), MetadataValue::Number(img.height() as i64));
        metadata.insert(
            "color_type".into(),
            MetadataValue::Text(format!("{:?}", img.color())),
        );

        // WebP stores EXIF in the "EXIF" RIFF chunk. `kamadak-exif`'s reader
        // understands WebP containers natively, so this call is identical to
        // the JPEG/TIFF path.
        for (key, value) in extract_exif_fields(data) {
            metadata.insert(key, value);
        }

        let content = maybe_ocr_content(data, &mut metadata);

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content,
            metadata,
            detection_confidence: 0.0,
        })
    }
}
