# Omniparse Web Service Guide

This guide shows you how to build a production-ready web service using Omniparse and Axum.

## Quick Start

### 1. Run the Example Server

```bash
cargo run --example web_service
```

The server starts on `http://localhost:3000`

### 2. Test with cURL

```bash
# Parse a JSON file
curl -X POST -F "file=@test_data/text/sample.json" http://localhost:3000/parse

# Detect file type only
curl -X POST -F "file=@test_data/document/sample.docx" http://localhost:3000/detect

# Get metadata only (no content)
curl -X POST -F "file=@test_data/text/sample.csv" http://localhost:3000/parse?metadata_only=true
```

### 3. Run the Test Script

```bash
bash examples/test_web_service.sh
```

### 4. Run the Client Example

```bash
# In one terminal
cargo run --example web_service

# In another terminal
cargo run --example web_client
```

## API Reference

### Endpoints

#### `GET /`
Root endpoint with service information.

**Response:**
```
Omniparse Web Service

Endpoints:
  POST /parse - Parse file
  POST /detect - Detect file type
  GET /health - Health check
```

#### `GET /health`
Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "service": "omniparse-web"
}
```

#### `POST /parse`
Parse a file and extract content and metadata.

**Request:**
- Content-Type: `multipart/form-data`
- Field: `file` (the file to parse)
- Query Parameters:
  - `metadata_only` (optional, boolean): If true, only metadata is returned

**Response:**
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

**Content Types:**
- Text content: Returns as string
- Binary content: Returns size and hex preview
- No content: Returns null

#### `POST /detect`
Detect file type without parsing content.

**Request:**
- Content-Type: `multipart/form-data`
- Field: `file` (the file to detect)

**Response:**
```json
{
  "filename": "sample.docx",
  "mime_type": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "confidence": 0.95,
  "detected_by": "MagicBytes"
}
```

### Error Responses

All errors return JSON with this structure:

```json
{
  "error": "error_type",
  "message": "Detailed error message"
}
```

**Error Types:**

| Status Code | Error Type | Description |
|-------------|------------|-------------|
| 400 | `multipart_error` | Invalid multipart data |
| 400 | `missing_file` | No file in request |
| 422 | `parse_error` | File parsing failed |

## Integration Examples

### JavaScript/TypeScript (Node.js)

```javascript
const FormData = require('form-data');
const fs = require('fs');
const axios = require('axios');

async function parseFile(filePath) {
  const form = new FormData();
  form.append('file', fs.createReadStream(filePath));
  
  const response = await axios.post('http://localhost:3000/parse', form, {
    headers: form.getHeaders()
  });
  
  return response.data;
}

// Usage
parseFile('document.pdf').then(result => {
  console.log('MIME Type:', result.mime_type);
  console.log('Metadata:', result.metadata);
});
```

### Python

```python
import requests

def parse_file(file_path):
    with open(file_path, 'rb') as f:
        files = {'file': f}
        response = requests.post('http://localhost:3000/parse', files=files)
        return response.json()

# Usage
result = parse_file('document.pdf')
print(f"MIME Type: {result['mime_type']}")
print(f"Metadata: {result['metadata']}")
```

### Rust (using reqwest)

See `examples/web_client.rs` for a complete example.

```rust
use reqwest::multipart;

async fn parse_file(file_path: &str) -> Result<ParseResponse, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let file_data = std::fs::read(file_path)?;
    
    let form = multipart::Form::new()
        .part("file", multipart::Part::bytes(file_data)
            .file_name("document.pdf"));
    
    let response = client
        .post("http://localhost:3000/parse")
        .multipart(form)
        .send()
        .await?
        .json()
        .await?;
    
    Ok(response)
}
```

## Production Deployment

### Configuration

For production, you'll want to add:

1. **Environment Variables**
```rust
let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
let addr = SocketAddr::from(([0, 0, 0, 0], port.parse().unwrap()));
```

2. **Request Size Limits**
```rust
use axum::extract::DefaultBodyLimit;

let app = Router::new()
    .route("/parse", post(parse_file))
    .layer(DefaultBodyLimit::max(10 * 1024 * 1024)); // 10MB limit
```

3. **CORS Support**
```rust
use tower_http::cors::{CorsLayer, Any};

let app = Router::new()
    .route("/parse", post(parse_file))
    .layer(CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any));
```

4. **Logging**
```rust
use tower_http::trace::TraceLayer;

let app = Router::new()
    .route("/parse", post(parse_file))
    .layer(TraceLayer::new_for_http());
```

5. **Rate Limiting**
```rust
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

let governor_conf = Box::new(
    GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(20)
        .finish()
        .unwrap()
);

let app = Router::new()
    .route("/parse", post(parse_file))
    .layer(GovernorLayer { config: Box::leak(governor_conf) });
```

### Docker Deployment

A production-ready multi-stage `Dockerfile` lives at the project root. It
bakes the ML OCR models into the image at `/opt/omniparse/models` (SHA-256
verified at build time) and ships a distroless runtime under a non-root UID.

```sh
# Build locally
docker build -t omniparse-web:dev .

# Or use the pre-published image (multi-arch: linux/amd64 + linux/arm64)
docker pull ghcr.io/sirhco/omniparse-web:latest
docker run --rm -p 3000:3000 ghcr.io/sirhco/omniparse-web:latest
```

For local development, a `docker-compose.yml` is also provided:

```sh
docker compose up --build
```

Runtime knobs honored by the image:

| Env var                  | Default                  | Purpose                              |
| ------------------------ | ------------------------ | ------------------------------------ |
| `OMNIPARSE_BIND`         | `0.0.0.0:3000`           | Address to listen on                 |
| `OMNIPARSE_OCR`          | `ml`                     | OCR backend (`off`/`classical`/`ml`) |
| `OMNIPARSE_OCR_MODELS`   | `/opt/omniparse/models`  | Where to read the rten models from   |

To use a host-side model cache instead of the baked-in one, mount a volume
over `/opt/omniparse/models` (or override `OMNIPARSE_OCR_MODELS` and mount
elsewhere):

```sh
docker run --rm -p 3000:3000 \
  -e OMNIPARSE_OCR_MODELS=/models \
  -v "$PWD/my-models:/models:ro" \
  ghcr.io/sirhco/omniparse-web:latest
```

### Performance Tips

1. **Use Connection Pooling**: Reuse the `Extractor` instance
2. **Enable Parallel Processing**: Use the `parallel` feature for batch operations
3. **Stream Large Files**: Use streaming for files > 10MB
4. **Cache Detection Results**: Cache MIME type detection for known file signatures
5. **Set Timeouts**: Add request timeouts to prevent hanging

## Security Considerations

1. **File Size Limits**: Always set maximum file size limits
2. **File Type Validation**: Validate MIME types before processing
3. **Sanitize Filenames**: Clean user-provided filenames
4. **Rate Limiting**: Implement rate limiting per IP
5. **Input Validation**: Validate all query parameters
6. **Error Messages**: Don't expose internal paths or system info in errors

## Monitoring

Add health checks and metrics:

```rust
#[derive(Serialize)]
struct Metrics {
    requests_total: u64,
    requests_success: u64,
    requests_failed: u64,
    avg_processing_time_ms: f64,
}

async fn metrics() -> Json<Metrics> {
    // Implement metrics collection
    Json(Metrics { /* ... */ })
}
```

## Testing

Run integration tests:

```bash
# Start the server
cargo run --example web_service &
SERVER_PID=$!

# Run tests
bash examples/test_web_service.sh

# Stop the server
kill $SERVER_PID
```

## Troubleshooting

### "Connection refused"
- Ensure the server is running
- Check the port is not already in use: `lsof -i :3000`

### "File too large"
- Increase the body size limit in Axum configuration

### "Parse error"
- Check the file format is supported: `omniparse --help`
- Verify the file is not corrupted

### "Out of memory"
- Enable streaming for large files
- Reduce concurrent request limits

## Further Reading

- [Axum Documentation](https://docs.rs/axum/)
- [Omniparse Documentation](../README.md)
- [Tokio Runtime Guide](https://tokio.rs/)
