# Omniparse

A Rust toolkit for detecting and extracting metadata, text, and content from hundreds of different file formats. Omniparse provides both a command-line interface and a library API, serving as a Rust equivalent to Apache Tika.

## Features

- **Automatic Type Detection**: Identifies file types using magic bytes, content analysis, and extension fallback
- **Multiple Format Support**: Extracts content from 25+ formats across text, document, image, audio, and archive categories
- **Rich Metadata Extraction**: Full EXIF for JPEG/TIFF, OpenGraph / Twitter / canonical for HTML, ID3 for MP3, OPF for EPUB, version/encryption/forms/annotations for PDF, and more
- **OCR Subsystem**: Optional classical and ML OCR pipelines for images and scanned PDFs. Pure Rust. Models download on first use for the ML backend (or pre-fetch via `omniparse models download`); classical backend has no external dependencies.
- **Production Web Service**: Ship-ready Axum example (`examples/web_service_prod.rs`) with Cloud Logging JSON, Prometheus `/metrics`, liveness/readiness probes, request limits, panic catcher, graceful shutdown — baked into the published Docker image and one-command deployable to Google Cloud Run via `deploy/cloud-run/deploy.sh`.
- **Dual Interface**: Use as a CLI tool or integrate as a library in your Rust applications
- **Pure Rust Implementation**: Minimal dependencies, no external system libraries required
- **Async Support**: Optional async API for non-blocking operations
- **Parallel Processing**: Batch process multiple files in parallel for better performance
- **Streaming Support**: Memory-efficient processing of large files
- **Security Hardening**: ZIP-bomb detection, XML entity limits, archive path-traversal detection, strict prototype validation

## Supported Formats

### Text Formats
- Plain Text (TXT)
- JSON
- CSV/TSV
- XML
- HTML (OpenGraph, Twitter Card, canonical URL, viewport, heading counts)
- CSS
- RTF (Rich Text Format)
- Markdown (via `pulldown-cmark`, optional `markdown` feature, default on)

### Document Formats
- PDF
- Microsoft Word (DOCX, DOC)
- Microsoft Excel (XLSX, XLS)
- Microsoft PowerPoint (PPTX, PPT)
- OpenDocument Text (ODT)
- OpenDocument Spreadsheet (ODS)
- OpenDocument Presentation (ODP)

### Document Formats (added)
- EPUB (OPF metadata, spine walk, chapter text — optional `epub` feature)

### Image Formats
- JPEG (full EXIF via `kamadak-exif`, optional OCR)
- PNG (text chunks including decompressed zTXt/iTXt, optional OCR)
- TIFF (EXIF via shared helper, optional OCR)
- SVG (title, desc, viewBox, text nodes, element counts — optional `svg` feature)
- WebP (dimensions, EXIF, optional OCR — optional `webp` feature)

### Audio Formats
- MP3 (ID3v1/v2 tags — title, artist, album, genre, year, track, duration — optional `mp3` feature)

### Archive Formats
- ZIP (with path-traversal detection via `contains_unsafe_paths` metadata)
- TAR (with path-traversal detection)

## Installation

### As a Library

Add Omniparse to your `Cargo.toml`:

```toml
[dependencies]
omniparse = "0.4"
```

For async support:

```toml
[dependencies]
omniparse = { version = "0.4", features = ["async"] }
```

For parallel processing:

```toml
[dependencies]
omniparse = { version = "0.4", features = ["parallel"] }
```

### OCR — quickstart

Two backends; one env var selects which runs.

```sh
# ML backend (recommended for photos / screenshots / unknown typography)
cargo install omniparse --features ocr-ml
omniparse models download              # one-time, ~12 MB to user cache
OMNIPARSE_OCR=ml omniparse photo.jpg

# Classical backend (pure Rust, no downloads — clean printed scans only)
cargo install omniparse --features ocr
OMNIPARSE_OCR=classical omniparse scan.png
```

Prefer a container? `docker run --rm -p 3000:3000 ghcr.io/sirhco/omniparse-web:latest`
launches the Axum web service with ML models baked in (see
[`Dockerfile`](Dockerfile) and [`examples/WEB_SERVICE_GUIDE.md`](examples/WEB_SERVICE_GUIDE.md)).

📖 **[Full OCR Guide →](OCR_GUIDE.md)** — backend chooser, model-cache CLI,
training custom prototypes, tuning, debugging, library API, FAQ.

### As a CLI Tool

Install using Cargo:

```bash
cargo install omniparse
```

Or build from source:

```bash
git clone https://github.com/omniparse/omniparse
cd omniparse
cargo build --release
```

The binary will be available at `target/release/omniparse`.

## Library Usage

### Basic Extraction

```rust
use omniparse::extract_from_path;

fn main() -> Result<(), omniparse::Error> {
    // Extract from a file
    let result = extract_from_path("document.pdf")?;
    
    println!("MIME type: {}", result.mime_type);
    println!("Confidence: {:.2}", result.detection_confidence);
    
    // Access content
    if let omniparse::Content::Text(text) = result.content {
        println!("Text content: {}", text);
    }
    
    // Access metadata
    if let Some(title) = result.metadata.title() {
        println!("Title: {}", title);
    }
    if let Some(author) = result.metadata.author() {
        println!("Author: {}", author);
    }
    
    Ok(())
}
```

### Extract from Bytes

```rust
use omniparse::extract_from_bytes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read("file.json")?;
    
    // With automatic type detection
    let result = extract_from_bytes(&data, None)?;
    
    // Or with a MIME type hint
    let result = extract_from_bytes(&data, Some("application/json"))?;
    
    println!("Detected: {}", result.mime_type);
    Ok(())
}
```

### Async Extraction

```rust
use omniparse::extract_from_path_async;

#[tokio::main]
async fn main() -> Result<(), omniparse::Error> {
    let result = extract_from_path_async("document.pdf").await?;
    println!("Extracted: {}", result.mime_type);
    Ok(())
}
```

### Check Supported Formats

```rust
use omniparse::{supported_mime_types, is_mime_supported};

fn main() {
    // Get all supported MIME types
    let types = supported_mime_types();
    println!("Supported formats: {}", types.len());
    
    // Check if a specific format is supported
    if is_mime_supported("application/pdf") {
        println!("PDF is supported!");
    }
}
```

### Batch Processing

```rust
use omniparse::core::Extractor;
use omniparse::utils::parallel::process_files_parallel;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let extractor = Extractor::new();
    let files = vec!["file1.pdf", "file2.docx", "file3.txt"];
    
    // Process files in parallel
    let results = process_files_parallel(&extractor, &files);
    
    for file_result in results {
        match file_result.result {
            Ok(extraction) => {
                println!("{}: {} (confidence: {:.2})",
                    file_result.path,
                    extraction.mime_type,
                    extraction.detection_confidence
                );
            }
            Err(e) => {
                eprintln!("{}: Error - {}", file_result.path, e);
            }
        }
    }
    
    Ok(())
}
```

## CLI Usage

### Basic Extraction

```bash
# Extract from a single file
omniparse document.pdf

# Extract from multiple files
omniparse file1.txt file2.docx file3.pdf
```

### Output Formats

```bash
# JSON output
omniparse --format json document.pdf

# YAML output
omniparse --format yaml document.pdf

# Save to file
omniparse --output results.json --format json document.pdf
```

### Metadata Only

```bash
# Extract only metadata, no content
omniparse --metadata-only document.pdf
```

### Type Detection Only

```bash
# Detect file type without extraction
omniparse --detect-only unknown_file.bin
```

### Parallel Processing

```bash
# Process multiple files in parallel
omniparse --parallel *.pdf
```

### Verbose Output

```bash
# Enable verbose logging
omniparse --verbose file1.pdf file2.pdf file3.pdf
```

### Combined Options

```bash
# Metadata only, JSON format, parallel processing
omniparse --metadata-only --format json --parallel --output metadata.json *.pdf
```

### Format-Specific Examples

```bash
# Extract from HTML files (web pages)
omniparse webpage.html index.htm
omniparse --format json --metadata-only page.html

# Extract from CSS files (stylesheets)
omniparse styles.css theme.css
omniparse --format json stylesheet.css  # Get rule and selector counts

# Extract from RTF files (rich text)
omniparse document.rtf letter.rtf
omniparse --metadata-only report.rtf

# Extract from spreadsheets (Excel and OpenDocument)
omniparse data.xlsx spreadsheet.xls budget.ods
omniparse --format json --output data.json financial.xlsx
omniparse --parallel *.xlsx *.xls *.ods  # Process multiple spreadsheets

# Extract from presentations (PowerPoint and OpenDocument)
omniparse slides.pptx presentation.ppt deck.odp
omniparse --metadata-only quarterly-review.pptx  # Get slide count and metadata
omniparse --format json --output slides.json presentation.pptx

# Extract from legacy Office files (DOC, XLS, PPT)
omniparse document.doc old-report.doc
omniparse spreadsheet.xls data-2010.xls
omniparse presentation.ppt slides-archive.ppt

# Mixed format batch processing
omniparse --parallel --format json --output results.json *.html *.css *.rtf *.xlsx *.pptx
```

## Error Handling

Omniparse provides detailed error types for different failure scenarios:

```rust
use omniparse::{extract_from_path, Error};

match extract_from_path("file.xyz") {
    Ok(result) => {
        println!("Success: {}", result.mime_type);
    }
    Err(Error::UnsupportedFormat(mime)) => {
        eprintln!("Format {} is not supported", mime);
    }
    Err(Error::Io(e)) => {
        eprintln!("IO error: {}", e);
    }
    Err(Error::CorruptedFile(msg)) => {
        eprintln!("File is corrupted: {}", msg);
    }
    Err(Error::PartialExtraction { message, partial_result }) => {
        eprintln!("Warning: {}", message);
        println!("Partial content available: {:?}", partial_result.content);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## New Format Support

Omniparse has recently added support for 9 additional document formats:

### Web Formats
- **HTML**: Extract visible text and metadata from web pages
- **CSS**: Analyze stylesheets with rule and selector counting

### Office Formats
- **XLSX/XLS**: Extract data from Excel spreadsheets (modern and legacy)
- **PPTX/PPT**: Extract text from PowerPoint presentations (modern and legacy)
- **DOC**: Extract content from legacy Word documents

### OpenDocument Formats
- **ODS**: Extract data from OpenDocument spreadsheets
- **ODP**: Extract text from OpenDocument presentations

### Rich Text
- **RTF**: Extract plain text from Rich Text Format files

See [SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md) for detailed information about each format.

## Performance

Omniparse is designed for performance:

- **Streaming**: Large files are processed using streaming to limit memory usage
- **Parallel Processing**: Batch operations can leverage multiple CPU cores
- **Pure Rust**: No FFI overhead or external process spawning
- **Efficient Detection**: Magic byte detection is fast and accurate

Typical performance on standard hardware:
- Text files (10 MB): < 100ms
- HTML files (1 MB): < 100ms (actual: ~0.6ms)
- PDF documents: 200-500ms depending on size
- XLSX files (10K cells): < 500ms (actual: ~0.9ms for small files)
- PPTX files (100 slides): < 1000ms (actual: ~0.6ms for small files)
- Image metadata: < 50ms

**All performance targets met or exceeded.** See [FINAL_PERFORMANCE_SUMMARY.md](FINAL_PERFORMANCE_SUMMARY.md) for comprehensive benchmark results.

## Architecture

Omniparse follows a modular architecture:

```
┌─────────────────┐
│   CLI / API     │
└────────┬────────┘
         │
┌────────▼────────┐
│   Extractor     │
└────┬───────┬────┘
     │       │
┌────▼───┐ ┌▼──────────┐
│Detector│ │  Registry  │
└────────┘ └─────┬──────┘
                 │
         ┌───────┴───────┐
         │    Parsers    │
         ├───────────────┤
         │ Text          │
         │ Document      │
         │ Image         │
         │ Archive       │
         └───────────────┘
```

- **Extractor**: Orchestrates detection and parsing
- **Detector**: Identifies file types using multiple methods
- **Registry**: Manages available parsers
- **Parsers**: Format-specific extraction implementations

## Documentation

### Version 0.4 (current)

- **[RELEASE_NOTES_v0.4.0.md](RELEASE_NOTES_v0.4.0.md)** - `omniparse models` CLI, unified `OMNIPARSE_OCR` env var, Dockerfile + GHCR image, production Cloud Run example
- **[OCR_GUIDE.md](OCR_GUIDE.md)** - Single canonical OCR reference: backend chooser, model-cache CLI, training, tuning, debugging
- **[examples/WEB_SERVICE_GUIDE.md](examples/WEB_SERVICE_GUIDE.md)** - Web service guide: minimal demo + production example + Cloud Run deploy
- **[CHANGELOG.md](CHANGELOG.md)** - Full changelog

### Version 0.3

- **[RELEASE_NOTES_v0.3.0.md](RELEASE_NOTES_v0.3.0.md)** - v0.3.0 enhancements, feature flags, env var reference
- **[MIGRATION_v0.3.0.md](MIGRATION_v0.3.0.md)** - Upgrade guide from v0.2.x

### General

- **[SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md)** - Complete list of supported formats
- **[examples/](examples/)** - Working code examples for all formats and OCR modes
- **API Documentation** - Run `cargo doc --open --features "ocr-ml ocr-train"` for full API docs

### Historical

- **[CLI_NEW_FORMATS_GUIDE.md](CLI_NEW_FORMATS_GUIDE.md)** - v0.2 CLI guide for initially-added formats
- **[MIGRATION_GUIDE.md](MIGRATION_GUIDE.md)** - v0.2 migration guide

## Contributing

Contributions are welcome! Areas for contribution:

- Adding support for new file formats
- Improving type detection accuracy
- Performance optimizations
- Documentation improvements
- Bug fixes

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

Inspired by [Apache Tika](https://tika.apache.org/), the Java-based content analysis toolkit.

### Core dependencies

Pure-Rust crates carrying the heavy lifting. License of each is compatible with omniparse's MIT/Apache-2.0 dual license.

| Crate                                                       | Used for                                                                | License             |
| ----------------------------------------------------------- | ----------------------------------------------------------------------- | ------------------- |
| [`lopdf`](https://crates.io/crates/lopdf)                   | Strict-tier PDF parsing (xref / trailer / object dictionary, embedded-image extraction for OCR)            | MIT                 |
| [`pdf-extract`](https://crates.io/crates/pdf-extract)       | 4th-tier PDF fallback for linearized / Identity-H + /ToUnicode CMap PDFs (Lucidchart, Word print-to-PDF). Behind the `pdf-extract` feature | MIT                 |
| [`weezl`](https://crates.io/crates/weezl)                   | LZWDecode stream filter in the raw_scan PDF fallback                    | MIT / Apache-2.0    |
| [`ascii85`](https://crates.io/crates/ascii85)               | ASCII85Decode stream filter in the raw_scan PDF fallback                | MIT / Apache-2.0    |
| [`ocrs`](https://crates.io/crates/ocrs) + [`rten`](https://crates.io/crates/rten) | ML OCR backend (text-detection + text-recognition models)         | MIT                 |
| [`image`](https://crates.io/crates/image), [`kamadak-exif`](https://crates.io/crates/kamadak-exif) | Image decode + EXIF                                            | MIT / Apache-2.0    |
| [`calamine`](https://crates.io/crates/calamine)             | XLSX / XLS / ODS parsing                                                | MIT / Apache-2.0    |
| [`scraper`](https://crates.io/crates/scraper) + [`cssparser`](https://crates.io/crates/cssparser) | HTML + CSS parsing                                              | ISC / MPL-2.0       |
| [`epub`](https://crates.io/crates/epub)                     | EPUB OPF + spine walk                                                   | MIT                 |
| [`id3`](https://crates.io/crates/id3)                       | MP3 ID3v1/v2 tags                                                       | MIT                 |
| [`zip`](https://crates.io/crates/zip), [`tar`](https://crates.io/crates/tar), [`flate2`](https://crates.io/crates/flate2) | Archive walking + deflate                                       | MIT / Apache-2.0    |
