# Omniparse

A Rust toolkit for detecting and extracting metadata, text, and content from hundreds of different file formats. Omniparse provides both a command-line interface and a library API, serving as a Rust equivalent to Apache Tika.

## Features

- **Automatic Type Detection**: Identifies file types using magic bytes, content analysis, and extension fallback
- **Multiple Format Support**: Extracts content from text, document, image, and archive formats
- **Rich Metadata Extraction**: Retrieves format-specific metadata including title, author, dates, and more
- **Dual Interface**: Use as a CLI tool or integrate as a library in your Rust applications
- **Pure Rust Implementation**: Minimal dependencies, no external system libraries required
- **Async Support**: Optional async API for non-blocking operations
- **Parallel Processing**: Batch process multiple files in parallel for better performance
- **Streaming Support**: Memory-efficient processing of large files

## Supported Formats

### Text Formats
- Plain Text (TXT)
- JSON
- CSV/TSV
- XML

### Document Formats
- PDF
- Microsoft Word (DOCX)
- OpenDocument Text (ODT)

### Image Formats
- JPEG (with EXIF metadata)
- PNG (with metadata chunks)
- TIFF (with tags)

### Archive Formats
- ZIP
- TAR

## Installation

### As a Library

Add Omniparse to your `Cargo.toml`:

```toml
[dependencies]
omniparse = "0.1"
```

For async support:

```toml
[dependencies]
omniparse = { version = "0.1", features = ["async"] }
```

For parallel processing:

```toml
[dependencies]
omniparse = { version = "0.1", features = ["parallel"] }
```

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

## Performance

Omniparse is designed for performance:

- **Streaming**: Large files are processed using streaming to limit memory usage
- **Parallel Processing**: Batch operations can leverage multiple CPU cores
- **Pure Rust**: No FFI overhead or external process spawning
- **Efficient Detection**: Magic byte detection is fast and accurate

Typical performance on standard hardware:
- Text files (10 MB): < 100ms
- PDF documents: 200-500ms depending on size
- Image metadata: < 50ms

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
