//! MP3 parser — ID3v1 / ID3v2 tag extraction via the `id3` crate.

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use id3::{Tag, TagLike};
use std::io::Cursor;

pub struct Mp3Parser;

impl Parser for Mp3Parser {
    fn name(&self) -> &str {
        "Mp3Parser"
    }

    fn supported_types(&self) -> &[&str] {
        &["audio/mpeg", "audio/mp3"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        let mut metadata = Metadata::new();

        let tag = Tag::read_from2(Cursor::new(data))
            .map_err(|e| Error::ParseError(format!("Failed to read ID3: {e}")))?;

        if let Some(v) = tag.title() {
            metadata.insert("title".into(), MetadataValue::Text(v.to_string()));
        }
        if let Some(v) = tag.artist() {
            metadata.insert("artist".into(), MetadataValue::Text(v.to_string()));
        }
        if let Some(v) = tag.album() {
            metadata.insert("album".into(), MetadataValue::Text(v.to_string()));
        }
        if let Some(v) = tag.album_artist() {
            metadata.insert("album_artist".into(), MetadataValue::Text(v.to_string()));
        }
        if let Some(v) = tag.genre() {
            metadata.insert("genre".into(), MetadataValue::Text(v.to_string()));
        }
        if let Some(v) = tag.year() {
            metadata.insert("year".into(), MetadataValue::Number(v as i64));
        }
        if let Some(v) = tag.track() {
            metadata.insert("track".into(), MetadataValue::Number(v as i64));
        }
        if let Some(v) = tag.total_tracks() {
            metadata.insert("total_tracks".into(), MetadataValue::Number(v as i64));
        }
        if let Some(v) = tag.disc() {
            metadata.insert("disc".into(), MetadataValue::Number(v as i64));
        }
        if let Some(v) = tag.duration() {
            metadata.insert("duration_ms".into(), MetadataValue::Number(v as i64));
        }

        // Collect comment text as the extracted content — closest thing MP3
        // tags have to user-visible prose.
        let mut content_text = String::new();
        for comment in tag.comments() {
            if !comment.text.trim().is_empty() {
                content_text.push_str(&comment.text);
                content_text.push('\n');
            }
        }

        let content = if content_text.trim().is_empty() {
            Content::None
        } else {
            Content::Text(content_text.trim().to_string())
        };

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content,
            metadata,
            detection_confidence: 0.0,
        })
    }
}
