//! Markdown parser.
//!
//! Strips formatting to produce plain text; extracts front-matter-free
//! structural metadata (heading count, link count, code block count).

use crate::core::{Content, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use pulldown_cmark::{Event, HeadingLevel, Parser as MdParser, Tag, TagEnd};

pub struct MarkdownParser;

impl Parser for MarkdownParser {
    fn name(&self) -> &str {
        "MarkdownParser"
    }

    fn supported_types(&self) -> &[&str] {
        &["text/markdown", "text/x-markdown"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        let source = String::from_utf8_lossy(data);
        let (title, metadata, text) = walk_events(&source);

        let mut meta = metadata;
        if let Some(t) = title {
            meta.insert("title".to_string(), MetadataValue::Text(t));
        }

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(text),
            metadata: meta,
            detection_confidence: 0.0,
        })
    }
}

fn walk_events(source: &str) -> (Option<String>, Metadata, String) {
    let mut metadata = Metadata::new();
    let mut text = String::new();
    let mut title: Option<String> = None;

    let mut h_counts = [0u64; 6];
    let mut link_count: u64 = 0;
    let mut image_count: u64 = 0;
    let mut code_block_count: u64 = 0;

    let mut in_heading: Option<HeadingLevel> = None;
    let mut current_heading = String::new();

    for event in MdParser::new(source) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    in_heading = Some(level);
                    current_heading.clear();
                    let idx = level as usize - 1;
                    if idx < 6 {
                        h_counts[idx] += 1;
                    }
                }
                Tag::Link { .. } => link_count += 1,
                Tag::Image { .. } => image_count += 1,
                Tag::CodeBlock(_) => code_block_count += 1,
                _ => {}
            },
            Event::End(end_tag) => match end_tag {
                TagEnd::Heading(level) => {
                    if title.is_none() && matches!(level, HeadingLevel::H1) {
                        let trimmed = current_heading.trim();
                        if !trimmed.is_empty() {
                            title = Some(trimmed.to_string());
                        }
                    }
                    in_heading = None;
                    if !current_heading.trim().is_empty() {
                        text.push_str(&current_heading);
                        text.push('\n');
                    }
                    current_heading.clear();
                }
                TagEnd::Paragraph => text.push_str("\n\n"),
                _ => {}
            },
            Event::Text(t) => {
                if in_heading.is_some() {
                    current_heading.push_str(&t);
                } else {
                    text.push_str(&t);
                }
            }
            Event::Code(c) => {
                if in_heading.is_some() {
                    current_heading.push_str(&c);
                } else {
                    text.push_str(&c);
                }
            }
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            _ => {}
        }
    }

    for (i, c) in h_counts.iter().enumerate() {
        if *c > 0 {
            metadata.insert(
                format!("heading_h{}_count", i + 1),
                MetadataValue::Number(*c as i64),
            );
        }
    }
    if link_count > 0 {
        metadata.insert("link_count".into(), MetadataValue::Number(link_count as i64));
    }
    if image_count > 0 {
        metadata.insert("image_count".into(), MetadataValue::Number(image_count as i64));
    }
    if code_block_count > 0 {
        metadata.insert(
            "code_block_count".into(),
            MetadataValue::Number(code_block_count as i64),
        );
    }

    (title, metadata, text.trim().to_string())
}
