# Omniparse Examples

This directory contains examples demonstrating how to use Omniparse in different scenarios.

## Available Examples

### 1. Web Service (`web_service.rs`)

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

### 2. Web Client (`web_client.rs`)

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

### 3. Test Script (`test_web_service.sh`)

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

The web service supports all file types that Omniparse supports:

- **Text**: TXT, JSON, CSV, XML
- **Documents**: PDF, DOCX, ODT
- **Archives**: ZIP, TAR, GZIP
- **Images**: PNG, JPEG, GIF (metadata only)
- And more...

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
