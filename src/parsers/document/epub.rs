//! EPUB parser. Reads metadata from OPF and concatenates chapter text from
//! the spine. Delegates container/ZIP handling to the `epub` crate.

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use epub::doc::EpubDoc;
use std::io::Cursor;

pub struct EpubParser;

impl Parser for EpubParser {
    fn name(&self) -> &str {
        "EpubParser"
    }

    fn supported_types(&self) -> &[&str] {
        &["application/epub+zip", "application/epub"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        let mut doc = EpubDoc::from_reader(Cursor::new(data))
            .map_err(|e| Error::ParseError(format!("Failed to open EPUB: {e}")))?;

        let mut metadata = Metadata::new();

        // `doc.metadata` is `Vec<MetadataItem { property, value, .. }>` and
        // `property` is the OPF name ("title", "creator", …). Take the first
        // value per key.
        let first_value = |key: &str| -> Option<String> {
            doc.metadata
                .iter()
                .find(|m| m.property == key)
                .map(|m| m.value.clone())
        };
        for (opf_key, out_key) in [
            ("title", "title"),
            ("creator", "author"),
            ("publisher", "publisher"),
            ("language", "language"),
            ("description", "description"),
            ("rights", "rights"),
            ("date", "publication_date"),
            ("subject", "keywords"),
            ("identifier", "identifier"),
        ] {
            if let Some(value) = first_value(opf_key) {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    metadata.insert(out_key.into(), MetadataValue::Text(trimmed.to_string()));
                }
            }
        }

        metadata.insert(
            "spine_count".into(),
            MetadataValue::Number(doc.spine.len() as i64),
        );
        metadata.insert(
            "resource_count".into(),
            MetadataValue::Number(doc.resources.len() as i64),
        );

        // Walk spine, concatenate textual chapter bodies. Each chapter arrives
        // as XHTML; strip tags with scraper.
        let mut text = String::new();
        loop {
            match doc.get_current_str() {
                Some((html, _mime)) => {
                    let extracted = strip_html(&html);
                    if !extracted.trim().is_empty() {
                        text.push_str(&extracted);
                        text.push_str("\n\n");
                    }
                }
                None => {}
            }
            if !doc.go_next() {
                break;
            }
        }

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text.trim().to_string()),
            metadata,
            detection_confidence: 0.0,
        })
    }
}

fn strip_html(html: &str) -> String {
    use scraper::{Html, Selector};
    let doc = Html::parse_document(html);
    let Ok(body_sel) = Selector::parse("body") else {
        return String::new();
    };
    let mut out = String::new();
    for body in doc.select(&body_sel) {
        for node in body.text() {
            let trimmed = node.trim();
            if trimmed.is_empty() {
                continue;
            }
            out.push_str(trimmed);
            out.push(' ');
        }
        out.push('\n');
    }
    out
}
