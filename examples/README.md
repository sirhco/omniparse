# Omniparse Examples

This directory contains examples demonstrating how to use Omniparse in different scenarios.

## Available Examples

### Basic Examples

#### 1. Basic Extraction (`basic_extraction.rs`)

The simplest way to use Omniparse - extract content and metadata from a file.

**Run:**
```bash
cargo run --example basic_extraction
```

#### 2. Async Extraction (`async_extraction.rs`)

Demonstrates non-blocking async extraction using Tokio.

**Run:**
```bash
cargo run --example async_extraction
```

#### 3. Batch Processing (`batch_processing.rs`)

Process multiple files in parallel for better performance.

**Run:**
```bash
cargo run --example batch_processing
```

#### 4. Custom Parser (`custom_parser.rs`)

Shows how to create and register a custom parser for a new file format.

**Run:**
```bash
cargo run --example custom_parser
```

### Format-Specific Examples

#### 5. HTML Extraction (`html_extraction.rs`)

Extract text and metadata from HTML files, excluding scripts and styles.

**Run:**
```bash
cargo run --example html_extraction
```

#### 6. CSS Extraction (`css_extraction.rs`)

Analyze CSS stylesheets, count rules and selectors, extract imports.

**Run:**
```bash
cargo run --example css_extraction
```

#### 7. RTF Extraction (`rtf_extraction.rs`)

Extract plain text from Rich Text Format files.

**Run:**
```bash
cargo run --example rtf_extraction
```

#### 8. Spreadsheet Extraction (`spreadsheet_extraction.rs`)

Extract data from Excel (XLSX, XLS) and OpenDocument (ODS) spreadsheets.

**Run:**
```bash
cargo run --example spreadsheet_extraction
```

#### 9. Presentation Extraction (`presentation_extraction.rs`)

Extract text from PowerPoint (PPTX, PPT) and OpenDocument (ODP) presentations.

**Run:**
```bash
cargo run --example presentation_extraction
```

#### 10. Legacy Office Extraction (`legacy_office_extraction.rs`)

Extract content from legacy Microsoft Office files (DOC, XLS, PPT).

**Run:**
```bash
cargo run --example legacy_office_extraction
```

### Web Service Examples

#### 11. Web Service (`web_service.rs`)

A complete REST API built with Axum that accepts file uploads and uses Omniparse to extract content and metadata.

**Features:**
- File upload via multipart/form-data
- Multiple endpoints (parse, detect, health)
- JSON responses
- Error handling
- Query parameters

**Run:**
```bash
cargo run --example web_service
# or
cd examples && make server
```

**Test:**
```bash
curl -X POST -F "file=@test_data/text/sample.json" http://localhost:3000/parse
```

#### 12. Web Client (`web_client.rs`)

A programmatic client demonstrating how to interact with the web service using Rust.

**Run:**
```bash
# Start server first
cargo run --example web_service

# Then in another terminal
cargo run --example web_client
# or
cd examples && make client
```

#### 13. Test Script (`test_web_service.sh`)

A bash script that tests all web service endpoints with various file types.

**Run:**
```bash
# Start server first
cargo run --example web_service

# Then in another terminal
bash examples/test_web_service.sh
# or
cd examples && make test
```

#### 14. CLI Test Script for New Formats (`test_new_formats_cli.sh`)

A comprehensive bash script that demonstrates CLI usage with all newly added formats (HTML, CSS, RTF, XLSX, PPTX, ODS, ODP).

**Run:**
```bash
bash examples/test_new_formats_cli.sh
```

**Tests include:**
- HTML extraction with various output formats
- CSS analysis and metadata extraction
- RTF text extraction
- XLSX spreadsheet processing
- PPTX presentation extraction
- ODS and ODP OpenDocument formats
- Type detection for new formats
- Parallel processing with new formats
- Mixed format batch processing

## Quick Start

### Option 1: Using Make

```bash
cd examples

# Terminal 1: Start the server
make server

# Terminal 2: Run tests
make test

# Or run the client
make client
```

### Option 2: Using Cargo

```bash
# Terminal 1: Start the server
cargo run --example web_service

# Terminal 2: Test with curl
curl -X POST -F "file=@test_data/text/sample.json" http://localhost:3000/parse

# Or run the client
cargo run --example web_client
```

## API Quick Reference

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | Service information |
| `/health` | GET | Health check |
| `/parse` | POST | Parse file and extract content |
| `/detect` | POST | Detect file type only |

**Parse endpoint with options:**
```bash
# Full extraction
curl -X POST -F "file=@document.pdf" http://localhost:3000/parse

# Metadata only
curl -X POST -F "file=@document.pdf" http://localhost:3000/parse?metadata_only=true
```

## Documentation

- **[WEB_SERVICE_GUIDE.md](WEB_SERVICE_GUIDE.md)** - Complete guide including:
  - API reference
  - Integration examples (JavaScript, Python, Rust)
  - Production deployment
  - Security considerations
  - Performance tips
  - Troubleshooting

## Supported File Types

Omniparse supports 35+ file formats across multiple categories:

### Text Formats
- Plain Text (TXT)
- JSON
- CSV/TSV
- XML
- HTML
- CSS
- RTF (Rich Text Format)

### Document Formats
- PDF
- Microsoft Word (DOCX, DOC)
- Microsoft Excel (XLSX, XLS)
- Microsoft PowerPoint (PPTX, PPT)
- OpenDocument Text (ODT)
- OpenDocument Spreadsheet (ODS)
- OpenDocument Presentation (ODP)

### Image Formats
- JPEG (with EXIF metadata)
- PNG (with metadata chunks)
- TIFF (with tags)

### Archive Formats
- ZIP
- TAR

See [SUPPORTED_FORMATS.md](../SUPPORTED_FORMATS.md) for complete details.

## Example Responses

### Parse Response
```json
{
  "filename": "sample.json",
  "mime_type": "application/json",
  "detection_confidence": 0.95,
  "metadata": {
    "valid": true,
    "schema_info": "object{author, data, name, version}"
  },
  "content": "author: Omniparse Test\n..."
}
```

### Detection Response
```json
{
  "filename": "sample.docx",
  "mime_type": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "confidence": 0.95,
  "detected_by": "MagicBytes"
}
```

### Error Response
```json
{
  "error": "parse_error",
  "message": "Unsupported format: application/x-unknown"
}
```

## Dependencies

The examples use:
- **axum** - Web framework
- **tokio** - Async runtime
- **reqwest** - HTTP client (for web_client example)
- **serde/serde_json** - Serialization

## Next Steps

1. Read the [WEB_SERVICE_GUIDE.md](WEB_SERVICE_GUIDE.md) for detailed documentation
2. Explore the source code in `web_service.rs` and `web_client.rs`
3. Adapt the examples for your use case
4. Check out the main [README.md](../README.md) for more Omniparse features
