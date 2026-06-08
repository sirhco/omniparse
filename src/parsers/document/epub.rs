//! EPUB parser. Reads metadata from OPF and concatenates chapter text in
//! reading order. Delegates container/ZIP handling to the `rbook` crate
//! (Apache-2.0). Supports EPUB 2 and 3.

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use rbook::Epub;
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
        // `rbook` requires the reader to be `'static`, so own the bytes.
        let epub = Epub::read(Cursor::new(data.to_vec()))
            .map_err(|e| Error::ParseError(format!("Failed to open EPUB: {e}")))?;

        let mut metadata = Metadata::new();

        // rbook exposes typed metadata accessors. Each typed entry yields its
        // OPF value via `MetaEntry::value()`. (Note: rbook 0.7 has no typed
        // accessor for `rights`, so that key is not extracted.)
        let md = epub.metadata();
        let fields: [(&str, Option<&str>); 8] = [
            ("title", md.title().map(|t| t.value())),
            ("author", md.creators().next().map(|c| c.value())),
            ("publisher", md.publishers().next().map(|p| p.value())),
            ("language", md.language().map(|l| l.value())),
            ("description", md.description().map(|d| d.value())),
            ("publication_date", md.published_entry().map(|e| e.value())),
            ("keywords", md.tags().next().map(|t| t.value())),
            ("identifier", md.identifier().map(|i| i.value())),
        ];
        for (out_key, value) in fields {
            if let Some(value) = value {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    metadata.insert(out_key.into(), MetadataValue::Text(trimmed.to_string()));
                }
            }
        }

        metadata.insert(
            "spine_count".into(),
            MetadataValue::Number(epub.spine().len() as i64),
        );
        metadata.insert(
            "resource_count".into(),
            MetadataValue::Number(epub.manifest().len() as i64),
        );

        // Walk readable content in canonical order, concatenate textual chapter
        // bodies. Each chapter arrives as XHTML; strip tags with scraper.
        let mut text = String::new();
        let mut reader = epub.reader();
        while let Some(Ok(content)) = reader.read_next() {
            let extracted = strip_html(content.content());
            if !extracted.trim().is_empty() {
                text.push_str(&extracted);
                text.push_str("\n\n");
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
