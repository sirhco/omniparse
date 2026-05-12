//! # Omniparse — Rust content extraction toolkit
//!
//! Apache-Tika-style detection and extraction for 25+ file formats. Pure
//! Rust, no system libraries, optional async / parallel / OCR.
//!
//! ## Supported formats
//!
//! - **Text**: Plain text, JSON, CSV/TSV, XML, HTML (OpenGraph, Twitter,
//!   canonical URL, heading counts), CSS, RTF, Markdown
//! - **Documents**: PDF (version, encryption, form-field / annotation /
//!   attachment counts), DOCX, DOC, XLSX, XLS, PPTX, PPT, ODT, ODS, ODP,
//!   EPUB
//! - **Images**: JPEG (full EXIF), PNG (decompressed zTXt/iTXt), TIFF,
//!   SVG, WebP. Optional OCR routes image text to `Content::Text`.
//! - **Audio**: MP3 (ID3v1/v2)
//! - **Archives**: ZIP, TAR (with path-traversal detection)
//!
//! See [`SUPPORTED_FORMATS.md`] for per-format metadata keys.
//!
//! ## Cargo features
//!
//! | Feature         | Default | Purpose                               |
//! | --------------- | ------- | ------------------------------------- |
//! | `async`         | off     | Tokio-based async extraction          |
//! | `parallel`      | off     | Rayon-based batch processing          |
//! | `markdown`      | **on**  | Markdown parser                       |
//! | `svg`           | **on**  | SVG parser                            |
//! | `webp`          | **on**  | WebP parser                           |
//! | `epub`          | **on**  | EPUB parser                           |
//! | `mp3`           | **on**  | MP3 parser                            |
//! | `pdf`           | **on**  | PDF parser via `lopdf` + lenient fallback (`weezl` / `ascii85`) |
//! | `pdf-extract`   | off     | 4th-tier PDF fallback via `pdf-extract` (linearized / Identity-H PDFs) |
//! | `ocr`           | off     | Classical OCR pipeline                |
//! | `ocr-train`     | off     | TTF → prototype trainer               |
//! | `ocr-parallel`  | off     | Parallel per-region recognition       |
//! | `ocr-ml`        | off     | ML OCR backend (ocrs + rten)          |
//!
//! ## Acknowledgments
//!
//! Omniparse stands on the shoulders of several pure-Rust libraries. The
//! PDF tier specifically uses:
//!
//! - [`lopdf`](https://crates.io/crates/lopdf) — strict-tier PDF parser
//!   (xref / trailer / object dictionary parse, embedded-image extraction
//!   for the OCR path). MIT licensed.
//! - [`weezl`](https://crates.io/crates/weezl) — LZWDecode filter
//!   support in the raw_scan fallback. MIT/Apache-2.0.
//! - [`ascii85`](https://crates.io/crates/ascii85) — ASCII85Decode filter
//!   support in the raw_scan fallback. MIT/Apache-2.0.
//! - [`pdf-extract`](https://crates.io/crates/pdf-extract) (optional, behind
//!   the `pdf-extract` feature) — 4th-tier text extraction for PDFs that
//!   lopdf can't load. MIT licensed.
//!
//! See `Cargo.toml` for the full dependency tree and per-crate version
//! pins.
//!
//! ## PDF parsing tiers
//!
//! Real-world PDFs are messy — truncated downloads, linearized exports,
//! Identity-H + /ToUnicode CMaps, and appended HTTP-chunk garbage all
//! defeat strict parsers. Omniparse's PDF parser is a four-tier fallback
//! chain so the caller almost always gets text:
//!
//! 1. **strict** — `lopdf::Document::load_mem`. Full metadata + per-page
//!    text. Most well-formed PDFs.
//! 2. **repaired_xref** — truncate trailing bytes after the last `%%EOF`,
//!    retry strict load. Catches HTTP-chunk leftovers / double-`%%EOF`.
//! 3. **raw_scan** — walk `stream`/`endstream` byte ranges, decode
//!    FlateDecode / LZWDecode / ASCII85Decode / uncompressed payloads,
//!    regex-extract `Tj` / `TJ` operators. Recovers text from PDFs
//!    lopdf can't load. Output gated by a "looks-like-text" heuristic
//!    so glyph-index / encrypted bytes don't reach the caller.
//! 4. **pdf_extract** (only with `--features pdf-extract`) — re-parse via
//!    [`pdf-extract`](https://crates.io/crates/pdf-extract). Tolerates
//!    linearized PDFs + Identity-H + /ToUnicode CMaps (Lucidchart, Word
//!    print-to-PDF, browser print-to-PDF).
//!
//! Every successful response carries a `pdf_parse_strategy` metadata
//! field (`"strict"` / `"repaired_xref"` / `"raw_scan"` / `"pdf_extract"`).
//! Tiers 2–4 also set `pdf_parse_partial = true` and
//! `pdf_parse_error = "<original lopdf error>"`. Tier 4 is the most
//! important opt-in for shops processing Lucidchart or Word-print
//! exports.
//!
//! ```no_run
//! # #[cfg(feature = "pdf")] {
//! let result = omniparse::extract_from_path("document.pdf")?;
//! if let Some(strategy) = result.metadata.get("pdf_parse_strategy") {
//!     println!("PDF parsed via tier: {strategy:?}");
//! }
//! # }
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! ## Web service example
//!
//! See `examples/web_service_prod.rs` for a Cloud Run-ready Axum service
//! that wraps this library: Cloud Logging JSON output, Prometheus
//! `/metrics`, `/live` + `/ready` probes, body-size + timeout +
//! concurrency limits, panic catcher, graceful shutdown, and a
//! `--healthcheck` mode for distroless containers. The published
//! Docker image uses this binary as its `ENTRYPOINT`.
//!
//! ## Quickstart
//!
//! ```no_run
//! use omniparse::extract_from_path;
//!
//! let result = extract_from_path("document.pdf")?;
//! println!("MIME type: {}", result.mime_type);
//! println!("Content: {:?}", result.content);
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! ## Extract from HTML
//!
//! ```no_run
//! use omniparse::extract_from_path;
//!
//! let result = extract_from_path("webpage.html")?;
//! if let Some(title) = result.metadata.get("title") {
//!     println!("Page title: {:?}", title);
//! }
//! // v0.3: OpenGraph, Twitter, canonical URL, heading counts also available.
//! if let Some(og_title) = result.metadata.get("og_title") {
//!     println!("og:title = {:?}", og_title);
//! }
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! ## Extract from spreadsheets
//!
//! ```no_run
//! use omniparse::extract_from_path;
//!
//! // Works with XLSX, XLS, and ODS
//! let result = extract_from_path("data.xlsx")?;
//! if let Some(sheet_count) = result.metadata.get("sheet_count") {
//!     println!("Number of sheets: {:?}", sheet_count);
//! }
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! ## Extract from bytes with MIME type hint
//!
//! ```no_run
//! use omniparse::extract_from_bytes;
//!
//! let data = std::fs::read("file.json")?;
//! let result = extract_from_bytes(&data, Some("application/json"))?;
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! ## Check supported formats
//!
//! ```
//! use omniparse::{supported_mime_types, is_mime_supported};
//!
//! let types = supported_mime_types();
//! println!("Supported types: {:?}", types);
//!
//! if is_mime_supported("application/pdf") {
//!     println!("PDF is supported!");
//! }
//! ```
//!
//! ## OCR
//!
//! Off by default. One env var selects the backend at runtime:
//!
//! - `OMNIPARSE_OCR=classical` — pure-Rust classical pipeline (`ocr` feature)
//! - `OMNIPARSE_OCR=ml` — ML backend via `ocrs` + `rten` (`ocr-ml` feature)
//! - `OMNIPARSE_OCR=off` / unset — OCR disabled (image parsers extract EXIF only)
//!
//! Image and PDF parsers automatically route through OCR when the gate is
//! set and populate `ocr_status` / `ocr_confidence` / `ocr_applied`
//! metadata.
//!
//! ```no_run
//! # #[cfg(feature = "ocr")] {
//! // OMNIPARSE_OCR=classical (or =ml) activates OCR for image parsers.
//! let result = omniparse::extract_from_path("photo.jpg")?;
//! if let Some(status) = result.metadata.get("ocr_status") {
//!     println!("ocr_status = {status:?}");
//! }
//! # }
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! Direct library use of the classical engine:
//!
//! ```no_run
//! # #[cfg(feature = "ocr")] {
//! use omniparse::ocr::OcrEngine;
//! let engine = OcrEngine::new();
//! let image = image::open("page.png").unwrap();
//! let output = engine.recognize(image)?;
//! println!("{}", output.text);
//! # }
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! ML backend (requires `ocr-ml` feature; pre-trained models are downloaded
//! and SHA-256-verified on first use, or pre-fetched via the CLI
//! `omniparse models download`):
//!
//! ```no_run
//! # #[cfg(feature = "ocr-ml")] {
//! let engine = omniparse::ocr::ml::MlOcrEngine::new()?;
//! let image = image::open("photo.jpg").unwrap();
//! let output = engine.recognize(image)?;
//! println!("{}", output.text);
//! # }
//! # Ok::<(), omniparse::Error>(())
//! ```
//!
//! See [`OCR_GUIDE.md`] for the model-cache CLI, training custom
//! prototypes, tuning, debugging, and the full env-var reference.
//!
//! [`SUPPORTED_FORMATS.md`]: https://github.com/sirhco/omniparse/blob/main/SUPPORTED_FORMATS.md
//! [`OCR_GUIDE.md`]: https://github.com/sirhco/omniparse/blob/main/OCR_GUIDE.md

pub mod core;
pub mod detection;
#[cfg(feature = "ocr")]
pub mod ocr;
pub mod parsers;
pub mod utils;

use std::path::Path;

// Re-export core types for convenience
pub use core::{Error, Result};
pub use core::result::{Content, ExtractionResult, Metadata, MetadataValue};

/// Extract text and metadata from a file at the specified path.
///
/// This function automatically detects the file type using magic bytes and content analysis,
/// then routes the file to the appropriate parser for extraction.
///
/// # Arguments
///
/// * `path` - Path to the file to extract content from
///
/// # Returns
///
/// Returns an `ExtractionResult` containing:
/// - The detected MIME type
/// - Extracted content (text or binary)
/// - Metadata fields (title, author, dates, etc.)
/// - Detection confidence score
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read (IO error)
/// - The file format is not supported
/// - The file is corrupted or malformed
/// - Parsing fails for any reason
///
/// # Examples
///
/// ```no_run
/// use omniparse::extract_from_path;
///
/// let result = extract_from_path("document.pdf")?;
/// println!("Detected type: {}", result.mime_type);
/// 
/// if let omniparse::Content::Text(text) = result.content {
///     println!("Extracted text: {}", text);
/// }
///
/// if let Some(title) = result.metadata.title() {
///     println!("Title: {}", title);
/// }
/// # Ok::<(), omniparse::Error>(())
/// ```
pub fn extract_from_path(path: impl AsRef<Path>) -> Result<ExtractionResult> {
    let extractor = core::Extractor::new();
    extractor.extract_from_path(path)
}

/// Extract text and metadata from a byte slice.
///
/// This function allows extraction from in-memory data. You can optionally provide
/// a MIME type hint to skip type detection and use a specific parser directly.
///
/// # Arguments
///
/// * `data` - Byte slice containing the file data
/// * `mime_hint` - Optional MIME type hint (e.g., "application/pdf")
///
/// # Returns
///
/// Returns an `ExtractionResult` with extracted content and metadata.
///
/// # Errors
///
/// Returns an error if:
/// - The format is not supported
/// - The data is corrupted or malformed
/// - Parsing fails for any reason
///
/// # Examples
///
/// ```no_run
/// use omniparse::extract_from_bytes;
///
/// // Extract with automatic type detection
/// let data = std::fs::read("file.json")?;
/// let result = extract_from_bytes(&data, None)?;
///
/// // Extract with MIME type hint
/// let result = extract_from_bytes(&data, Some("application/json"))?;
/// # Ok::<(), omniparse::Error>(())
/// ```
pub fn extract_from_bytes(data: &[u8], mime_hint: Option<&str>) -> Result<ExtractionResult> {
    let extractor = core::Extractor::new();
    extractor.extract_from_bytes(data, mime_hint)
}

/// Get a list of all supported MIME types.
///
/// This function returns all MIME types that have registered parsers in the system.
/// You can use this to check what formats are supported before attempting extraction.
///
/// # Returns
///
/// A vector of MIME type strings (e.g., "application/pdf", "text/plain")
///
/// # Examples
///
/// ```
/// use omniparse::supported_mime_types;
///
/// let types = supported_mime_types();
/// println!("Omniparse supports {} formats", types.len());
/// for mime_type in types {
///     println!("  - {}", mime_type);
/// }
/// ```
pub fn supported_mime_types() -> Vec<String> {
    parsers::default_registry().supported_types()
}

/// Check if a specific MIME type is supported.
///
/// This is a convenience function to quickly check if a format can be processed
/// without needing to iterate through all supported types.
///
/// # Arguments
///
/// * `mime_type` - The MIME type to check (e.g., "application/pdf")
///
/// # Returns
///
/// `true` if the MIME type is supported, `false` otherwise
///
/// # Examples
///
/// ```
/// use omniparse::is_mime_supported;
///
/// if is_mime_supported("application/pdf") {
///     println!("PDF files are supported!");
/// }
///
/// if !is_mime_supported("application/x-custom") {
///     println!("Custom format is not supported");
/// }
/// ```
pub fn is_mime_supported(mime_type: &str) -> bool {
    parsers::default_registry().get_parser(mime_type).is_some()
}

/// Extract text and metadata from a file asynchronously.
///
/// This is the async version of `extract_from_path`, using Tokio for async file I/O.
/// It provides the same functionality but allows for non-blocking operation in async contexts.
///
/// **Note:** This function is only available when the `async` feature is enabled.
///
/// # Arguments
///
/// * `path` - Path to the file to extract content from
///
/// # Returns
///
/// Returns an `ExtractionResult` containing the extracted content and metadata.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read (IO error)
/// - The file format is not supported
/// - The file is corrupted or malformed
/// - Parsing fails for any reason
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "async")]
/// # async fn example() -> Result<(), omniparse::Error> {
/// use omniparse::extract_from_path_async;
///
/// let result = extract_from_path_async("document.pdf").await?;
/// println!("Detected type: {}", result.mime_type);
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "async")]
pub async fn extract_from_path_async(path: impl AsRef<Path>) -> Result<ExtractionResult> {
    use tokio::io::AsyncReadExt;
    
    let path = path.as_ref();
    
    // Read the file asynchronously
    let mut file = tokio::fs::File::open(path).await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;
    
    // Use the synchronous extraction logic on the buffered data
    // We also need to detect the type from the path
    let extractor = core::Extractor::new();
    let detection = extractor.detector.detect_from_path(path)?;
    
    // Get the parser and parse the data
    let parser = extractor.registry.get_parser(&detection.mime_type)
        .ok_or_else(|| Error::UnsupportedFormat(detection.mime_type.clone()))?;
    
    let mut result = parser.parse(&buffer, &detection.mime_type)?;
    result.detection_confidence = detection.confidence;
    result.mime_type = detection.mime_type;
    
    Ok(result)
}
