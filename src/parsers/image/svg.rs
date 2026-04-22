//! SVG (Scalable Vector Graphics) parser.
//!
//! Treats SVG as structured XML. Extracts `<title>`, `<desc>`, all text-node
//! content, the root `viewBox`, and element counts for common shapes.

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

pub struct SvgParser;

impl Parser for SvgParser {
    fn name(&self) -> &str {
        "SvgParser"
    }

    fn supported_types(&self) -> &[&str] {
        &["image/svg+xml"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        let text = std::str::from_utf8(data)
            .map_err(|e| Error::ParseError(format!("SVG not UTF-8: {e}")))?;
        let mut reader = Reader::from_str(text);
        reader.trim_text(true);

        let mut metadata = Metadata::new();
        let mut content_text = String::new();

        let mut root_seen = false;
        let mut in_title = false;
        let mut in_desc = false;
        let mut in_text = false;
        let mut current_title = String::new();
        let mut current_desc = String::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(e)) => {
                    let name = e.name().as_ref().to_vec();
                    let local = local_name(&name).to_vec();
                    bump_counter(&mut metadata, local.as_slice());
                }
                Ok(Event::Start(e)) => {
                    let name = e.name().as_ref().to_vec();
                    let local = local_name(&name).to_vec();
                    if !root_seen && local.as_slice() == b"svg" {
                        root_seen = true;
                        for attr in e.attributes().with_checks(false).flatten() {
                            let k = attr.key.as_ref();
                            let v = attr
                                .decode_and_unescape_value(&reader)
                                .map(|v| v.into_owned())
                                .unwrap_or_default();
                            match local_name(k) {
                                b"viewBox" => {
                                    metadata.insert("viewbox".into(), MetadataValue::Text(v));
                                }
                                b"width" => {
                                    metadata.insert("width".into(), MetadataValue::Text(v));
                                }
                                b"height" => {
                                    metadata.insert("height".into(), MetadataValue::Text(v));
                                }
                                b"xmlns" => {
                                    metadata.insert("xmlns".into(), MetadataValue::Text(v));
                                }
                                _ => {}
                            }
                        }
                    }
                    match local.as_slice() {
                        b"title" => in_title = true,
                        b"desc" => in_desc = true,
                        b"text" => in_text = true,
                        tag => {
                            bump_counter(&mut metadata, tag);
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name().as_ref().to_vec();
                    match local_name(&name) {
                        b"title" => {
                            in_title = false;
                            if metadata.get("title").is_none() && !current_title.is_empty() {
                                metadata.insert(
                                    "title".into(),
                                    MetadataValue::Text(current_title.trim().to_string()),
                                );
                            }
                            current_title.clear();
                        }
                        b"desc" => {
                            in_desc = false;
                            if metadata.get("description").is_none() && !current_desc.is_empty() {
                                metadata.insert(
                                    "description".into(),
                                    MetadataValue::Text(current_desc.trim().to_string()),
                                );
                            }
                            current_desc.clear();
                        }
                        b"text" => {
                            in_text = false;
                            content_text.push('\n');
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(t)) => {
                    let s = t.unescape().unwrap_or_default().into_owned();
                    if in_title {
                        current_title.push_str(&s);
                    } else if in_desc {
                        current_desc.push_str(&s);
                    } else if in_text {
                        content_text.push_str(&s);
                        content_text.push(' ');
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(Error::ParseError(format!("SVG parse error: {e}")));
                }
                _ => {}
            }
            buf.clear();
        }

        if !root_seen {
            return Err(Error::ParseError("no <svg> root element".into()));
        }

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(content_text.trim().to_string()),
            metadata,
            detection_confidence: 0.0,
        })
    }
}

fn local_name(qname: &[u8]) -> &[u8] {
    match qname.iter().rposition(|&b| b == b':') {
        Some(i) => &qname[i + 1..],
        None => qname,
    }
}

fn bump_counter(metadata: &mut Metadata, tag: &[u8]) {
    let tag_str = match std::str::from_utf8(tag) {
        Ok(s) => s,
        Err(_) => return,
    };
    // Only track counts for the most common visual primitives. Ignore
    // metadata/definition elements.
    let tracked = matches!(
        tag_str,
        "path"
            | "rect"
            | "circle"
            | "ellipse"
            | "line"
            | "polyline"
            | "polygon"
            | "g"
            | "use"
            | "image"
    );
    if !tracked {
        return;
    }
    let key = format!("element_{}_count", tag_str);
    let new = match metadata.get(&key) {
        Some(MetadataValue::Number(n)) => *n + 1,
        _ => 1,
    };
    metadata.insert(key, MetadataValue::Number(new));
}
