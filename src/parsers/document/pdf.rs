//! PDF document parser

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use lopdf::Document;

/// Parser for PDF documents
pub struct PdfParser;

impl Parser for PdfParser {
    fn supported_types(&self) -> &[&str] {
        &["application/pdf"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        let doc = Document::load_mem(data).map_err(|e| {
            Error::ParseError(format!("Failed to load PDF: {}", e))
        })?;

        let text = extract_text(&doc)?;
        let mut metadata = extract_metadata(&doc)?;

        // If the text layer is empty (scanned PDF with only rasters), try
        // OCR on embedded images. Feature + runtime gate must both be on.
        let final_text = if text.trim().is_empty() {
            let (ocr_text, ocr_info) = maybe_ocr_pdf_images(&doc);
            if let Some(info) = ocr_info {
                for (k, v) in info {
                    metadata.insert(k, v);
                }
            }
            ocr_text.unwrap_or(text)
        } else {
            text
        };

        Ok(ExtractionResult {
            mime_type: mime_type.to_string(),
            content: Content::Text(final_text),
            metadata,
            detection_confidence: 1.0,
        })
    }

    fn name(&self) -> &str {
        "PdfParser"
    }
}

/// Extract text content from PDF document
fn extract_text(doc: &Document) -> Result<String> {
    let mut text = String::new();
    let pages = doc.get_pages();

    for (page_num, _) in pages.iter() {
        match doc.extract_text(&[*page_num]) {
            Ok(page_text) => {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(&page_text);
            }
            Err(e) => {
                // Continue with other pages even if one fails
                eprintln!("Warning: Failed to extract text from page {}: {}", page_num, e);
            }
        }
    }

    Ok(text)
}

/// Extract metadata from PDF document
fn extract_metadata(doc: &Document) -> Result<Metadata> {
    let mut metadata = Metadata::new();

    // Page count
    let pages = doc.get_pages();
    metadata.insert("page_count".to_string(), MetadataValue::Number(pages.len() as i64));

    // PDF spec version ("1.4", "1.7", "2.0", ...)
    metadata.insert(
        "pdf_version".to_string(),
        MetadataValue::Text(doc.version.clone()),
    );

    // Encryption flag — presence of /Encrypt in the trailer indicates an encrypted doc.
    let encrypted = doc.trailer.get(b"Encrypt").is_ok();
    metadata.insert("encrypted".to_string(), MetadataValue::Boolean(encrypted));

    // Info dictionary (Title/Author/Subject/Creator/Producer/CreationDate)
    if let Some(info_dict) = resolve_info_dict(doc) {
        for (pdf_key, out_key) in [
            (&b"Title"[..], "title"),
            (&b"Author"[..], "author"),
            (&b"Subject"[..], "subject"),
            (&b"Creator"[..], "creator"),
            (&b"Producer"[..], "producer"),
            (&b"CreationDate"[..], "creation_date"),
            (&b"ModDate"[..], "modification_date"),
            (&b"Keywords"[..], "keywords"),
        ] {
            if let Ok(obj) = info_dict.get(pdf_key) {
                if let Ok(s) = obj.as_string() {
                    metadata.insert(out_key.to_string(), MetadataValue::Text(s.to_string()));
                }
            }
        }
    }

    // Catalog-level fields: page_layout, page_mode, AcroForm, Names/EmbeddedFiles
    if let Some(catalog) = resolve_catalog(doc) {
        if let Ok(page_layout) = catalog.get(b"PageLayout") {
            if let Ok(s) = page_layout.as_name_str() {
                metadata.insert("page_layout".to_string(), MetadataValue::Text(s.to_string()));
            }
        }
        if let Ok(page_mode) = catalog.get(b"PageMode") {
            if let Ok(s) = page_mode.as_name_str() {
                metadata.insert("page_mode".to_string(), MetadataValue::Text(s.to_string()));
            }
        }

        // AcroForm /Fields count (top-level form field array length)
        let form_fields_count = catalog
            .get(b"AcroForm")
            .ok()
            .and_then(|v| dereference(doc, v))
            .and_then(|d| d.as_dict().ok())
            .and_then(|d| d.get(b"Fields").ok())
            .and_then(|v| dereference(doc, v))
            .and_then(|v| v.as_array().ok())
            .map(|a| a.len())
            .unwrap_or(0);
        metadata.insert(
            "form_fields_count".to_string(),
            MetadataValue::Number(form_fields_count as i64),
        );

        // Attachments: /Names -> /EmbeddedFiles -> /Names array (pairs of [name, fileSpec])
        let attachments_count = catalog
            .get(b"Names")
            .ok()
            .and_then(|v| dereference(doc, v))
            .and_then(|d| d.as_dict().ok())
            .and_then(|d| d.get(b"EmbeddedFiles").ok())
            .and_then(|v| dereference(doc, v))
            .and_then(|d| d.as_dict().ok())
            .and_then(|d| d.get(b"Names").ok())
            .and_then(|v| dereference(doc, v))
            .and_then(|v| v.as_array().ok())
            .map(|a| a.len() / 2)
            .unwrap_or(0);
        metadata.insert(
            "attachments_count".to_string(),
            MetadataValue::Number(attachments_count as i64),
        );
    }

    // Annotations: sum /Annots array lengths across pages.
    let annotations_count: usize = pages
        .values()
        .filter_map(|oid| doc.get_object(*oid).ok())
        .filter_map(|obj| obj.as_dict().ok())
        .filter_map(|page_dict| page_dict.get(b"Annots").ok())
        .filter_map(|v| dereference(doc, v))
        .filter_map(|v| v.as_array().ok().map(|a| a.len()))
        .sum();
    metadata.insert(
        "annotations_count".to_string(),
        MetadataValue::Number(annotations_count as i64),
    );

    Ok(metadata)
}

fn resolve_info_dict(doc: &Document) -> Option<&lopdf::Dictionary> {
    let info = doc.trailer.get(b"Info").ok()?;
    dereference(doc, info)?.as_dict().ok()
}

fn resolve_catalog(doc: &Document) -> Option<&lopdf::Dictionary> {
    let root = doc.trailer.get(b"Root").ok()?;
    dereference(doc, root)?.as_dict().ok()
}

fn dereference<'a>(doc: &'a Document, obj: &'a lopdf::Object) -> Option<&'a lopdf::Object> {
    match obj {
        lopdf::Object::Reference(r) => doc.get_object(*r).ok(),
        other => Some(other),
    }
}

/// Walk every page's `/Resources/XObject` dict, collect `Subtype = /Image`
/// streams whose filters we can pass straight to a container-aware decoder
/// (primarily `DCTDecode` = JPEG). Returns `(image_bytes, mime_type_hint)`
/// tuples.
fn collect_embedded_images(doc: &Document) -> Vec<(Vec<u8>, &'static str)> {
    let mut out = Vec::new();
    let mut seen_ids: std::collections::HashSet<lopdf::ObjectId> = std::collections::HashSet::new();

    for (_, page_id) in doc.get_pages() {
        let Ok(page) = doc.get_object(page_id) else { continue };
        let Ok(page_dict) = page.as_dict() else { continue };
        let Some(resources) = page_dict
            .get(b"Resources")
            .ok()
            .and_then(|v| dereference(doc, v))
            .and_then(|v| v.as_dict().ok())
        else {
            continue;
        };
        let Some(xobjects) = resources
            .get(b"XObject")
            .ok()
            .and_then(|v| dereference(doc, v))
            .and_then(|v| v.as_dict().ok())
        else {
            continue;
        };

        for (_name, obj) in xobjects.iter() {
            let id = match obj {
                lopdf::Object::Reference(r) => *r,
                _ => continue,
            };
            if !seen_ids.insert(id) {
                continue;
            }
            let Ok(obj) = doc.get_object(id) else { continue };
            let Ok(stream) = obj.as_stream() else { continue };
            let subtype = stream
                .dict
                .get(b"Subtype")
                .ok()
                .and_then(|v| v.as_name_str().ok())
                .unwrap_or("");
            if subtype != "Image" {
                continue;
            }

            // Filter classification. `DCTDecode` = JPEG payload → hand to
            // `image` crate as-is. `JPXDecode` = JPEG2000 (rarely supported;
            // skipped). `FlateDecode` / `CCITTFaxDecode` produce raw pixel
            // data that needs explicit reconstruction from ColorSpace +
            // BitsPerComponent — out of scope for Tier 1.
            let filters = stream.filters().unwrap_or_default();
            let filter_name = filters.first().map(String::as_str).unwrap_or("");
            match filter_name {
                "DCTDecode" => {
                    out.push((stream.content.clone(), "image/jpeg"));
                }
                _ => {
                    // Skip unsupported-filter images silently; count surfaced
                    // via metadata in the caller.
                }
            }
        }
    }
    out
}

/// Run OCR on every embedded image in the PDF, concatenate recognized text.
/// Returns the recognized text (if any) and diagnostic metadata entries.
fn maybe_ocr_pdf_images(
    _doc: &Document,
) -> (Option<String>, Option<Vec<(String, MetadataValue)>>) {
    #[cfg(feature = "ocr")]
    {
        if !crate::ocr::runtime_enabled() {
            return (None, None);
        }

        let images = collect_embedded_images(_doc);
        let total = images.len();
        if images.is_empty() {
            return (
                None,
                Some(vec![
                    (
                        "ocr_status".to_string(),
                        MetadataValue::Text("no_embedded_images".into()),
                    ),
                    (
                        "ocr_applied".to_string(),
                        MetadataValue::Boolean(false),
                    ),
                ]),
            );
        }

        let mut text = String::new();
        let mut recognized_count = 0usize;
        let mut confidences: Vec<f32> = Vec::new();

        for (i, (bytes, _hint)) in images.iter().enumerate() {
            match crate::ocr::run_ocr(bytes) {
                crate::ocr::OcrAttempt::Recognized {
                    text: t,
                    mean_confidence,
                } => {
                    if !text.is_empty() {
                        text.push_str("\n\n");
                    }
                    text.push_str(&format!("[image {} of {}]\n", i + 1, total));
                    text.push_str(&t);
                    recognized_count += 1;
                    confidences.push(mean_confidence);
                }
                _ => {}
            }
        }

        let mut info = vec![
            (
                "ocr_status".to_string(),
                MetadataValue::Text(
                    if recognized_count > 0 {
                        "recognized"
                    } else {
                        "no_text_found"
                    }
                    .to_string(),
                ),
            ),
            (
                "ocr_applied".to_string(),
                MetadataValue::Boolean(recognized_count > 0),
            ),
            (
                "ocr_images_total".to_string(),
                MetadataValue::Number(total as i64),
            ),
            (
                "ocr_images_recognized".to_string(),
                MetadataValue::Number(recognized_count as i64),
            ),
        ];
        if !confidences.is_empty() {
            let mean = confidences.iter().sum::<f32>() / confidences.len() as f32;
            info.push((
                "ocr_confidence".to_string(),
                MetadataValue::Float(mean as f64),
            ));
        }

        if recognized_count > 0 {
            (Some(text), Some(info))
        } else {
            (None, Some(info))
        }
    }
    #[cfg(not(feature = "ocr"))]
    {
        (None, None)
    }
}
