//! PDF document parser
//!
//! Parses PDFs via a fallback chain so that real-world inputs (downloads
//! truncated mid-write, files with garbage trailing bytes, content streams
//! whose xref is corrupt) still produce useful output:
//!
//! 1. **Strict** — `lopdf::Document::load_mem` on the raw bytes. Full
//!    metadata + structured text extraction. Most well-formed PDFs.
//! 2. **Repaired xref** — scan backward for the last `%%EOF` marker, drop
//!    anything after it, retry `load_mem`. Recovers PDFs with appended
//!    HTTP-chunk garbage, double-`%%EOF` exports, or corrupt linearization
//!    hints that confuse the strict parser. Same output shape as Strict.
//! 3. **Raw stream scan** — walk every `stream`/`endstream` pair in the
//!    bytes, decompress FlateDecode payloads with `flate2`, regex-extract
//!    `(literal) Tj` and `[…] TJ` content operators. No structural parse,
//!    no per-page split. Metadata limited to `pdf_parse_strategy` +
//!    `pdf_parse_partial`. Last-resort path for PDFs lopdf can't load at
//!    all.
//!
//! Every successful response carries a `pdf_parse_strategy` metadata field
//! (`strict` / `repaired_xref` / `raw_scan`) so callers can tell which
//! tier ran. Strategies 2 and 3 also set `pdf_parse_partial = true` to
//! signal that some metadata may be missing or imprecise.

use crate::core::{Content, Error, ExtractionResult, Metadata, MetadataValue, Result};
use crate::parsers::Parser;
use flate2::read::ZlibDecoder;
use lopdf::Document;
use std::io::Read;

/// Parser for PDF documents
pub struct PdfParser;

impl Parser for PdfParser {
    fn supported_types(&self) -> &[&str] {
        &["application/pdf"]
    }

    fn parse(&self, data: &[u8], mime_type: &str) -> Result<ExtractionResult> {
        // Tier 1 — strict load.
        match Document::load_mem(data) {
            Ok(doc) => return finalize_from_doc(doc, mime_type, "strict", false),
            Err(strict_err) => {
                // Tier 2 — truncate trailing junk after the last %%EOF.
                if let Some(truncated) = repair_trailing_junk(data) {
                    if let Ok(doc) = Document::load_mem(&truncated) {
                        return finalize_from_doc(doc, mime_type, "repaired_xref", true);
                    }
                }
                // Tier 3 — raw content-stream scan. Pure byte heuristic.
                let scanned = raw_scan_text(data);
                let scanned_ok =
                    !scanned.trim().is_empty() && looks_like_text(&scanned);
                if !scanned_ok {
                    // raw_scan failed or rejected as garbage. Try the
                    // fourth tier when the `pdf-extract` feature is on —
                    // it uses a different structural parser that tolerates
                    // linearization and Identity-H + /ToUnicode CMaps
                    // (Lucidchart exports, Word print-to-PDF, etc.).
                    #[cfg(feature = "pdf-extract")]
                    if let Some(extracted) = try_pdf_extract(data) {
                        if looks_like_text(&extracted) {
                            let mut metadata = Metadata::new();
                            metadata.insert(
                                "pdf_parse_strategy".to_string(),
                                MetadataValue::Text("pdf_extract".to_string()),
                            );
                            metadata.insert(
                                "pdf_parse_partial".to_string(),
                                MetadataValue::Boolean(true),
                            );
                            metadata.insert(
                                "pdf_parse_error".to_string(),
                                MetadataValue::Text(strict_err.to_string()),
                            );
                            if let Some(version) = scan_pdf_version(data) {
                                metadata.insert(
                                    "pdf_version".to_string(),
                                    MetadataValue::Text(version),
                                );
                            }
                            return Ok(ExtractionResult {
                                mime_type: mime_type.to_string(),
                                content: Content::Text(extracted),
                                metadata,
                                detection_confidence: 1.0,
                            });
                        }
                    }
                    // Don't hand the caller garbage — surface as a hard
                    // error with a hint at the most likely cause.
                    return Err(Error::ParseError(format!(
                        "Failed to load PDF: {strict_err}. Tried lenient \
                         fallback but recovered text was unreadable (font \
                         CMap, encryption, or non-FlateDecode compression). \
                         {extract_hint}",
                        extract_hint = pdf_extract_hint(),
                    )));
                }
                let mut metadata = Metadata::new();
                metadata.insert(
                    "pdf_parse_strategy".to_string(),
                    MetadataValue::Text("raw_scan".to_string()),
                );
                metadata.insert(
                    "pdf_parse_partial".to_string(),
                    MetadataValue::Boolean(true),
                );
                metadata.insert(
                    "pdf_parse_error".to_string(),
                    MetadataValue::Text(strict_err.to_string()),
                );
                // Recover the PDF version from the header bytes (`%PDF-X.Y`)
                // — independent of the trailer so it works even when xref
                // is destroyed.
                if let Some(version) = scan_pdf_version(data) {
                    metadata.insert(
                        "pdf_version".to_string(),
                        MetadataValue::Text(version),
                    );
                }
                Ok(ExtractionResult {
                    mime_type: mime_type.to_string(),
                    content: Content::Text(scanned),
                    metadata,
                    detection_confidence: 1.0,
                })
            }
        }
    }

    fn name(&self) -> &str {
        "PdfParser"
    }
}

/// Build an `ExtractionResult` from a successfully-loaded `Document`.
/// Handles the OCR-fallback path for image-only PDFs.
fn finalize_from_doc(
    doc: Document,
    mime_type: &str,
    strategy: &str,
    partial: bool,
) -> Result<ExtractionResult> {
    let text = extract_text(&doc)?;
    let mut metadata = extract_metadata(&doc)?;

    // OCR fallback when the text layer is empty.
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

    metadata.insert(
        "pdf_parse_strategy".to_string(),
        MetadataValue::Text(strategy.to_string()),
    );
    if partial {
        metadata.insert(
            "pdf_parse_partial".to_string(),
            MetadataValue::Boolean(true),
        );
    }

    Ok(ExtractionResult {
        mime_type: mime_type.to_string(),
        content: Content::Text(final_text),
        metadata,
        detection_confidence: 1.0,
    })
}

/// Suffix added to the rejection error message: when `pdf-extract` is *not*
/// compiled in, point users at the feature so they can opt-in to better
/// linearized-PDF coverage.
#[cfg(feature = "pdf-extract")]
fn pdf_extract_hint() -> &'static str {
    "(pdf-extract fallback was tried and also failed.)"
}
#[cfg(not(feature = "pdf-extract"))]
fn pdf_extract_hint() -> &'static str {
    "Rebuild with --features pdf-extract for an extra linearized-PDF / \
     CMap-aware fallback parser."
}

/// Tier-4 fallback: parse the PDF with the `pdf-extract` crate. Text-only
/// (no metadata, no per-page split). Output normalized so form-feeds
/// between pages become double newlines, matching the strict-tier shape.
#[cfg(feature = "pdf-extract")]
fn try_pdf_extract(data: &[u8]) -> Option<String> {
    let raw = pdf_extract::extract_text_from_mem(data).ok()?;
    let normalized = raw.replace('\x0c', "\n\n");
    let trimmed = normalized.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Reject raw_scan output that's clearly not human-readable text. Catches
/// the common failure mode where PDFs map text through a font `/Encoding`
/// or `/ToUnicode` CMap — the bytes inside `(...)` operators are glyph
/// indices, not characters, so without the CMap we'd hand the caller
/// strings of control codes like ``.
///
/// Heuristic: reject when fewer than 60% of characters are printable
/// (ASCII graphic + common whitespace + Unicode letters), or when the
/// output contains no run of four or more contiguous alphanumeric
/// characters. Real text always has at least one such run; encrypted /
/// CMap'd noise rarely does.
fn looks_like_text(s: &str) -> bool {
    let mut total = 0usize;
    let mut printable = 0usize;
    let mut run = 0usize;
    let mut max_run = 0usize;
    for c in s.chars() {
        total += 1;
        let is_printable = c.is_ascii_graphic()
            || matches!(c, ' ' | '\t' | '\n' | '\r')
            || c.is_alphabetic();
        if is_printable {
            printable += 1;
        }
        if c.is_ascii_alphanumeric() {
            run += 1;
            if run > max_run {
                max_run = run;
            }
        } else {
            run = 0;
        }
    }
    if total == 0 {
        return false;
    }
    let ratio = (printable as f64) / (total as f64);
    ratio >= 0.60 && max_run >= 4
}

/// Scan the file header for `%PDF-X.Y` and return the version digits, or
/// `None` when the header is missing/malformed. Works on broken files
/// because the version sits in the first ~16 bytes, independent of xref.
fn scan_pdf_version(data: &[u8]) -> Option<String> {
    const HEADER: &[u8] = b"%PDF-";
    let header_pos = data.windows(HEADER.len()).position(|w| w == HEADER)?;
    let rest = &data[header_pos + HEADER.len()..];
    // Read until first whitespace; accept e.g. "1.4", "1.7", "2.0".
    let end = rest
        .iter()
        .position(|b| b.is_ascii_whitespace())
        .unwrap_or(rest.len().min(8));
    let slice = &rest[..end];
    let s = std::str::from_utf8(slice).ok()?;
    if s.contains('.') && s.chars().all(|c| c.is_ascii_digit() || c == '.') {
        Some(s.to_string())
    } else {
        None
    }
}

/// Locate the last `%%EOF` marker and return a copy of `data` truncated
/// just after it. Returns `None` if no `%%EOF` is present or it's already
/// at the end (truncation would be a no-op).
fn repair_trailing_junk(data: &[u8]) -> Option<Vec<u8>> {
    const NEEDLE: &[u8] = b"%%EOF";
    // Scan from the end so we land on the *last* %%EOF.
    let pos = (0..=data.len().saturating_sub(NEEDLE.len()))
        .rev()
        .find(|&i| data[i..].starts_with(NEEDLE))?;
    let end = pos + NEEDLE.len();
    // Allow a trailing newline (the canonical PDF tail).
    let end = match data.get(end) {
        Some(b'\r') if data.get(end + 1) == Some(&b'\n') => end + 2,
        Some(b'\r') | Some(b'\n') => end + 1,
        _ => end,
    };
    if end >= data.len() {
        // No trailing junk to strip; nothing to repair this way.
        return None;
    }
    Some(data[..end].to_vec())
}

/// Walk every `stream` … `endstream` pair in the byte slice, try each of
/// the PDF stream filters we support (FlateDecode, LZWDecode, ASCII85Decode,
/// uncompressed), and concatenate the text recovered from `Tj`/`TJ` content
/// operators. Pure byte-level heuristic — no PDF structure parsed.
///
/// We don't know which `/Filter` the original stream declared, so we try
/// each decoder and only scan the result when it actually contains text
/// operators (`Tj` / `TJ` substring). The `has_text_operators` gate keeps
/// us from feeding garbage from a wrong-filter decode into the operator
/// scanner.
fn raw_scan_text(data: &[u8]) -> String {
    let mut out = String::new();
    for stream in iter_streams(data) {
        // Try every common PDF stream filter; whichever decodes into bytes
        // that contain `Tj`/`TJ` tokens contributes operators. Failed
        // decodes return None and are skipped. The uncompressed-fallback
        // (raw) ensures we also pick up content streams that ship without
        // a filter at all.
        let candidates = [
            inflate_zlib(stream),
            decode_lzw(stream),
            decode_ascii85(stream),
            Some(stream.to_vec()),
        ];
        for decoded in candidates.into_iter().flatten() {
            if has_text_operators(&decoded) {
                scan_content_operators(&decoded, &mut out);
            }
        }
    }
    out
}

fn has_text_operators(data: &[u8]) -> bool {
    find_subsequence(data, b"Tj").is_some() || find_subsequence(data, b"TJ").is_some()
}

/// Decode an LZWDecode-compressed PDF stream. PDF uses early-change LZW
/// with MSB bit order over an 8-bit alphabet. Returns `None` when the
/// payload isn't LZW (so the caller can try the next filter).
fn decode_lzw(data: &[u8]) -> Option<Vec<u8>> {
    use weezl::{decode::Decoder, BitOrder};
    if data.is_empty() {
        return None;
    }
    let mut decoder = Decoder::new(BitOrder::Msb, 8);
    decoder.decode(data).ok()
}

/// Decode an ASCII85Decode-compressed PDF stream. PDF ASCII85 may be
/// wrapped in `<~ … ~>` delimiters; strip them before handing off to the
/// `ascii85` crate.
fn decode_ascii85(data: &[u8]) -> Option<Vec<u8>> {
    let s = std::str::from_utf8(data).ok()?;
    let s = s.trim();
    let s = s.strip_prefix("<~").unwrap_or(s);
    let s = s.strip_suffix("~>").unwrap_or(s);
    ascii85::decode(s).ok()
}

/// Yield each stream payload in `data`. Looks for the exact tokens
/// `stream\n` (or `stream\r\n`) and the next `endstream`. Skips anything
/// it can't bracket cleanly.
fn iter_streams<'a>(data: &'a [u8]) -> impl Iterator<Item = &'a [u8]> + 'a {
    let mut cursor = 0usize;
    std::iter::from_fn(move || {
        loop {
            let rest = data.get(cursor..)?;
            let rel = find_subsequence(rest, b"stream")?;
            cursor += rel + b"stream".len();
            // PDF spec: `stream` keyword is followed by EOL (`\r\n` or `\n`).
            let payload_start = match data.get(cursor) {
                Some(b'\r') if data.get(cursor + 1) == Some(&b'\n') => cursor + 2,
                Some(b'\n') => cursor + 1,
                _ => continue, // not an actual stream keyword (could be `endstream` later, etc.)
            };
            let rest = data.get(payload_start..)?;
            let end_rel = find_subsequence(rest, b"endstream")?;
            let payload_end = payload_start + end_rel;
            cursor = payload_end + b"endstream".len();
            return Some(&data[payload_start..payload_end]);
        }
    })
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

fn inflate_zlib(data: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).ok()?;
    Some(out)
}

/// Pull text out of a decoded content stream by walking for `(...)Tj` and
/// `[...] TJ` operator pairs. Skips PostScript-style comments, handles
/// PDF string escapes, and inserts newlines on `'`/`"` operators.
fn scan_content_operators(data: &[u8], out: &mut String) {
    let mut i = 0usize;
    let n = data.len();
    let mut emitted_text_this_run = false;
    while i < n {
        let b = data[i];
        // Skip PDF comments (`%` to end-of-line).
        if b == b'%' {
            while i < n && data[i] != b'\n' && data[i] != b'\r' {
                i += 1;
            }
            continue;
        }
        // Literal string.
        if b == b'(' {
            let (s, next) = read_literal_string(data, i);
            i = next;
            // Lookahead to the operator that follows. Skip whitespace and
            // numeric tokens (used by TJ kerning).
            let op = peek_next_operator(data, i);
            match op.as_deref() {
                Some(name @ "Tj") | Some(name @ "'") | Some(name @ "\"") => {
                    out.push_str(&s);
                    if name != "Tj" {
                        out.push('\n');
                    }
                    emitted_text_this_run = true;
                }
                Some("TJ") => {
                    // `(s) TJ` is unusual (TJ wants an array) but be lenient.
                    out.push_str(&s);
                    emitted_text_this_run = true;
                }
                _ => {
                    // Could be a string used by some other operator (e.g.,
                    // setfont). Ignore.
                }
            }
            continue;
        }
        // TJ array `[ (a) num (b) num … ] TJ`.
        if b == b'[' {
            let (joined, next) = read_tj_array(data, i);
            i = next;
            if let Some("TJ") = peek_next_operator(data, i).as_deref() {
                out.push_str(&joined);
                emitted_text_this_run = true;
            }
            continue;
        }
        // Detect text-object boundary `ET` → newline so blocks separate.
        if b == b'E'
            && data.get(i..i + 2) == Some(b"ET")
            && is_word_boundary(data, i, 2)
            && emitted_text_this_run
        {
            out.push('\n');
            emitted_text_this_run = false;
            i += 2;
            continue;
        }
        i += 1;
    }
}

/// PDF literal string: balanced parens with escape sequences.
fn read_literal_string(data: &[u8], start: usize) -> (String, usize) {
    let mut s = String::new();
    let mut i = start + 1; // skip opening `(`
    let mut depth = 1usize;
    let n = data.len();
    while i < n && depth > 0 {
        let b = data[i];
        if b == b'\\' {
            i += 1;
            if i >= n {
                break;
            }
            match data[i] {
                b'n' => s.push('\n'),
                b'r' => s.push('\r'),
                b't' => s.push('\t'),
                b'b' => s.push('\x08'),
                b'f' => s.push('\x0c'),
                b'\\' => s.push('\\'),
                b'(' => s.push('('),
                b')' => s.push(')'),
                b'\r' | b'\n' => { /* line continuation — ignore */ }
                c @ b'0'..=b'7' => {
                    // Up to three octal digits.
                    let mut val = (c - b'0') as u32;
                    let mut count = 1;
                    while count < 3 && i + 1 < n {
                        let nxt = data[i + 1];
                        if !(b'0'..=b'7').contains(&nxt) {
                            break;
                        }
                        val = val * 8 + (nxt - b'0') as u32;
                        i += 1;
                        count += 1;
                    }
                    if let Some(c) = char::from_u32(val) {
                        s.push(c);
                    }
                }
                other => s.push(other as char),
            }
            i += 1;
        } else if b == b'(' {
            depth += 1;
            s.push('(');
            i += 1;
        } else if b == b')' {
            depth -= 1;
            if depth > 0 {
                s.push(')');
            }
            i += 1;
        } else {
            s.push(b as char);
            i += 1;
        }
    }
    (s, i)
}

/// Read a `[ … ]` array, returning the concatenation of every literal
/// string element. Numbers (kerning offsets) and hex strings are skipped.
fn read_tj_array(data: &[u8], start: usize) -> (String, usize) {
    let mut s = String::new();
    let mut i = start + 1; // skip `[`
    let n = data.len();
    while i < n {
        match data[i] {
            b']' => {
                i += 1;
                break;
            }
            b'(' => {
                let (piece, next) = read_literal_string(data, i);
                s.push_str(&piece);
                i = next;
            }
            b'<' => {
                // Hex string `<…>`. Skip — rare in normal Tj/TJ flows.
                while i < n && data[i] != b'>' {
                    i += 1;
                }
                if i < n {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    (s, i)
}

/// Skip whitespace + numeric tokens after a string, return the next
/// non-whitespace operator name (sequence of ASCII letters / single
/// quote / double quote).
fn peek_next_operator(data: &[u8], start: usize) -> Option<String> {
    let mut i = start;
    let n = data.len();
    // Skip whitespace + numbers (TJ kerning offsets).
    while i < n {
        let b = data[i];
        if b.is_ascii_whitespace()
            || b.is_ascii_digit()
            || b == b'-'
            || b == b'+'
            || b == b'.'
        {
            i += 1;
        } else {
            break;
        }
    }
    if i >= n {
        return None;
    }
    let op_start = i;
    while i < n {
        let b = data[i];
        if b.is_ascii_alphabetic() || b == b'\'' || b == b'"' {
            i += 1;
        } else {
            break;
        }
    }
    if i == op_start {
        return None;
    }
    Some(std::str::from_utf8(&data[op_start..i]).ok()?.to_string())
}

fn is_word_boundary(data: &[u8], i: usize, len: usize) -> bool {
    let before_ok = i == 0 || !data[i - 1].is_ascii_alphanumeric();
    let after_ok = data
        .get(i + len)
        .map(|b| !b.is_ascii_alphanumeric())
        .unwrap_or(true);
    before_ok && after_ok
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_strips_trailing_junk() {
        let mut bytes = b"%PDF-1.4\n...body...\n%%EOF\n".to_vec();
        let original_len = bytes.len();
        bytes.extend_from_slice(b"garbage appended by middlebox");
        let repaired = repair_trailing_junk(&bytes).expect("should detect trailing junk");
        assert_eq!(repaired.len(), original_len);
        assert!(repaired.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn repair_returns_none_when_no_junk() {
        let bytes = b"%PDF-1.4\n...body...\n%%EOF\n".to_vec();
        assert!(repair_trailing_junk(&bytes).is_none());
    }

    #[test]
    fn raw_scan_extracts_literal_strings() {
        // Hand-crafted content stream with a single Tj literal. Wrap with
        // bogus `stream` / `endstream` markers so iter_streams() picks it up.
        let content = b"BT (Hello World) Tj ET\n";
        let mut blob = Vec::new();
        blob.extend_from_slice(b"%PDF-1.4\n");
        blob.extend_from_slice(b"4 0 obj << /Length 25 >> stream\n");
        blob.extend_from_slice(content);
        blob.extend_from_slice(b"endstream endobj\n");
        let text = raw_scan_text(&blob);
        assert!(text.contains("Hello World"), "got {:?}", text);
    }

    #[test]
    fn raw_scan_extracts_tj_array() {
        let content = b"BT [(Hel) -300 (lo)] TJ ET\n";
        let mut blob = Vec::new();
        blob.extend_from_slice(b"%PDF-1.4\n");
        blob.extend_from_slice(b"4 0 obj << /Length 26 >> stream\n");
        blob.extend_from_slice(content);
        blob.extend_from_slice(b"endstream endobj\n");
        let text = raw_scan_text(&blob);
        assert!(text.contains("Hello"), "got {:?}", text);
    }

    #[test]
    fn raw_scan_decodes_lzw_stream() {
        use weezl::{encode::Encoder, BitOrder};
        let content = b"BT (LZW worked) Tj ET\n";
        let compressed = Encoder::new(BitOrder::Msb, 8).encode(content).unwrap();
        let mut blob = Vec::new();
        blob.extend_from_slice(b"%PDF-1.4\n");
        blob.extend_from_slice(b"4 0 obj << /Filter /LZWDecode >> stream\n");
        blob.extend_from_slice(&compressed);
        blob.extend_from_slice(b"\nendstream endobj\n");
        let text = raw_scan_text(&blob);
        assert!(text.contains("LZW worked"), "got {:?}", text);
    }

    #[test]
    fn raw_scan_decodes_ascii85_stream() {
        let content = b"BT (ASCII85 worked) Tj ET\n";
        let encoded = ascii85::encode(content);
        let mut blob = Vec::new();
        blob.extend_from_slice(b"%PDF-1.4\n");
        blob.extend_from_slice(b"4 0 obj << /Filter /ASCII85Decode >> stream\n");
        blob.extend_from_slice(encoded.as_bytes());
        blob.extend_from_slice(b"\nendstream endobj\n");
        let text = raw_scan_text(&blob);
        assert!(text.contains("ASCII85 worked"), "got {:?}", text);
    }

    #[test]
    fn looks_like_text_accepts_real_text() {
        assert!(looks_like_text("Hello PDF\nWorld"));
        assert!(looks_like_text("The quick brown fox jumps over the lazy dog"));
    }

    #[test]
    fn looks_like_text_rejects_glyph_indices() {
        // Typical CMap'd / encrypted output: control codes, no word runs.
        let garbage: String = (1u8..=12).map(|b| b as char).collect();
        assert!(!looks_like_text(&garbage));
    }

    #[test]
    fn looks_like_text_rejects_short_alphanumeric_islands() {
        // Mostly punctuation + control chars with tiny "ab" island — rejected.
        let s = "\x01\x02\x03ab\x04\x05\x06";
        assert!(!looks_like_text(s));
    }

    #[test]
    fn literal_string_handles_escapes() {
        // (Hello\\nworld\\(paren\\)) → Hello\nworld(paren)
        let input = b"(Hello\\nworld\\(paren\\))";
        let (s, _) = read_literal_string(input, 0);
        assert_eq!(s, "Hello\nworld(paren)");
    }
}
