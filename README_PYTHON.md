# Omniparse Python Bindings

Python bindings for Omniparse - a high-performance library for detecting and extracting metadata, text, and content from various file formats.

## Features

- **Multi-format Support**: Extract content from PDF, Office documents, images, archives, and more
- **Automatic Detection**: Intelligent MIME type detection based on file content
- **Rich Metadata**: Extract titles, authors, dates, and format-specific metadata
- **High Performance**: Native Rust implementation with Python bindings via PyO3
- **Thread-Safe**: Release the GIL during I/O and parsing for true concurrent processing
- **Type Hints**: Full type stub support for IDE autocomplete and static type checking

## Installation

Install from PyPI:

```bash
pip install omniparse
```

### Requirements

- Python 3.8 or higher
- No additional dependencies required (pre-built wheels available)

### Building from Source

If you need to build from source:

```bash
# Install maturin
pip install maturin

# Clone the repository
git clone https://github.com/sirhco/omniparse.git
cd omniparse

# Build and install in development mode
maturin develop --release
```

## Quick Start

### Basic Usage

Extract content from a file:

```python
import omniparse

# Extract from a file path
result = omniparse.extract_from_path("document.pdf")

print(f"MIME Type: {result.mime_type}")
print(f"Confidence: {result.detection_confidence}")
print(f"Content: {result.content}")
print(f"Metadata: {result.metadata}")
```

### Extract from Bytes

Process file data already in memory:

```python
import omniparse

# Read file into memory
with open("document.pdf", "rb") as f:
    data = f.read()

# Extract with optional MIME type hint
result = omniparse.extract_from_bytes(data, mime_hint="application/pdf")
print(result.content)
```

### Query Supported Formats

Check which formats are supported:

```python
import omniparse

# Get all supported MIME types
formats = omniparse.supported_mime_types()
print(f"Supports {len(formats)} formats")

# Check if a specific format is supported
if omniparse.is_mime_supported("application/pdf"):
    print("PDF extraction is supported")
```

## API Reference

### Functions

#### `extract_from_path(path: str) -> ExtractionResult`

Extract content and metadata from a file.

**Parameters:**
- `path` (str): Path to the file to extract from

**Returns:**
- `ExtractionResult`: Object containing extracted data

**Raises:**
- `IOError`: If the file cannot be read or does not exist
- `ValueError`: If the file format is unsupported or corrupted
- `RuntimeError`: If detection fails or extraction is only partially successful

**Example:**
```python
result = omniparse.extract_from_path("report.pdf")
```

#### `extract_from_bytes(data: bytes, mime_hint: Optional[str] = None) -> ExtractionResult`

Extract content and metadata from raw bytes.

**Parameters:**
- `data` (bytes): Raw bytes of the file content
- `mime_hint` (Optional[str]): Optional MIME type hint to assist detection

**Returns:**
- `ExtractionResult`: Object containing extracted data

**Raises:**
- `ValueError`: If the data format is unsupported or corrupted
- `RuntimeError`: If detection fails or extraction is only partially successful

**Example:**
```python
with open("data.json", "rb") as f:
    result = omniparse.extract_from_bytes(f.read())
```

#### `supported_mime_types() -> List[str]`

Get a list of all supported MIME types.

**Returns:**
- `List[str]`: List of supported MIME type strings

**Example:**
```python
formats = omniparse.supported_mime_types()
```

#### `is_mime_supported(mime_type: str) -> bool`

Check if a specific MIME type is supported.

**Parameters:**
- `mime_type` (str): MIME type string to check

**Returns:**
- `bool`: True if supported, False otherwise

**Example:**
```python
if omniparse.is_mime_supported("text/csv"):
    # Process CSV file
    pass
```

### Classes

#### `ExtractionResult`

Result object containing extracted file data.

**Properties:**

- `mime_type` (str): The detected MIME type (e.g., "application/pdf")
- `content` (Optional[Union[str, bytes]]): Extracted content
  - `str` for text-based formats
  - `bytes` for binary formats
  - `None` if no content extracted
- `metadata` (Dict[str, Any]): Dictionary of extracted metadata
- `detection_confidence` (float): Confidence score (0.0-1.0) for MIME type detection

**Common Metadata Keys:**
- `title`: Document title
- `author`: Document author
- `created`: Creation date (ISO 8601 string)
- `modified`: Last modification date (ISO 8601 string)
- `page_count`: Number of pages
- `word_count`: Number of words

## Error Handling

Omniparse uses standard Python exceptions:

```python
import omniparse

try:
    result = omniparse.extract_from_path("document.pdf")
    print(result.content)
    
except IOError as e:
    # File not found or cannot be read
    print(f"File access error: {e}")
    
except ValueError as e:
    # Unsupported format or corrupted file
    print(f"Format or parsing error: {e}")
    
except RuntimeError as e:
    # Detection failed or partial extraction
    print(f"Processing error: {e}")
```

### Error Types

- **IOError**: Raised when file cannot be accessed or read
- **ValueError**: Raised for unsupported formats or corrupted files
- **RuntimeError**: Raised when detection fails or extraction is incomplete

## Supported Formats

Omniparse supports a wide range of file formats:

### Documents
- PDF (`.pdf`)
- Microsoft Word (`.doc`, `.docx`)
- Microsoft PowerPoint (`.ppt`, `.pptx`)
- Microsoft Excel (`.xls`, `.xlsx`)
- OpenDocument (`.odt`, `.ods`, `.odp`)
- Rich Text Format (`.rtf`)

### Text Formats
- Plain Text (`.txt`)
- JSON (`.json`)
- XML (`.xml`)
- CSV (`.csv`)
- HTML (`.html`, `.htm`)
- CSS (`.css`)

### Images
- JPEG (`.jpg`, `.jpeg`)
- PNG (`.png`)
- TIFF (`.tiff`, `.tif`)

### Archives
- ZIP (`.zip`)
- TAR (`.tar`)

## Performance Tips

### Concurrent Processing

Omniparse releases the Python GIL during I/O and parsing, enabling true concurrent processing:

```python
import omniparse
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

def process_file(file_path):
    """Process a single file."""
    try:
        result = omniparse.extract_from_path(str(file_path))
        return {
            'path': file_path,
            'mime_type': result.mime_type,
            'success': True
        }
    except Exception as e:
        return {
            'path': file_path,
            'error': str(e),
            'success': False
        }

# Process multiple files concurrently
files = list(Path("documents").glob("*.pdf"))

with ThreadPoolExecutor(max_workers=4) as executor:
    results = list(executor.map(process_file, files))

# Print results
for result in results:
    if result['success']:
        print(f"✓ {result['path']}: {result['mime_type']}")
    else:
        print(f"✗ {result['path']}: {result['error']}")
```

### Batch Processing

For processing many files efficiently:

```python
import omniparse
from pathlib import Path

def batch_extract(directory, pattern="*.*"):
    """Extract content from all files matching pattern."""
    results = []
    
    for file_path in Path(directory).glob(pattern):
        if file_path.is_file():
            try:
                result = omniparse.extract_from_path(str(file_path))
                results.append({
                    'file': file_path.name,
                    'mime_type': result.mime_type,
                    'content_length': len(result.content) if result.content else 0,
                    'metadata': result.metadata
                })
            except Exception as e:
                print(f"Error processing {file_path}: {e}")
    
    return results

# Process all PDFs in a directory
results = batch_extract("documents", "*.pdf")
print(f"Processed {len(results)} files")
```

### Memory Efficiency

For large files, consider processing in chunks or streaming:

```python
import omniparse

# For very large files, extract metadata only if possible
result = omniparse.extract_from_path("large_document.pdf")

# Access metadata without loading full content
print(f"Pages: {result.metadata.get('page_count', 'unknown')}")
print(f"Author: {result.metadata.get('author', 'unknown')}")

# Only access content if needed
if result.content:
    # Process content in chunks or save to disk
    content_preview = result.content[:1000]  # First 1000 chars
    print(f"Preview: {content_preview}")
```

### Performance Characteristics

- **Extraction Overhead**: < 10% compared to native Rust implementation
- **Thread Scalability**: Linear scaling up to CPU core count
- **Memory Overhead**: < 5% compared to native Rust implementation
- **Type Conversion**: < 1ms for typical results

## Examples

### Extract Metadata Only

```python
import omniparse

result = omniparse.extract_from_path("document.pdf")

# Access metadata
metadata = result.metadata
print(f"Title: {metadata.get('title', 'Untitled')}")
print(f"Author: {metadata.get('author', 'Unknown')}")
print(f"Created: {metadata.get('created', 'Unknown')}")
print(f"Pages: {metadata.get('page_count', 'Unknown')}")
```

### Process Multiple Formats

```python
import omniparse
from pathlib import Path

def extract_all(directory):
    """Extract content from all supported files in directory."""
    for file_path in Path(directory).iterdir():
        if not file_path.is_file():
            continue
        
        try:
            result = omniparse.extract_from_path(str(file_path))
            print(f"\n{file_path.name}")
            print(f"  Type: {result.mime_type}")
            print(f"  Confidence: {result.detection_confidence:.2f}")
            
            if result.content:
                preview = result.content[:100]
                print(f"  Preview: {preview}...")
                
        except ValueError as e:
            print(f"  Skipped: {e}")

extract_all("mixed_documents")
```

### Validate File Format

```python
import omniparse

def validate_file_type(file_path, expected_mime):
    """Validate that a file matches expected MIME type."""
    result = omniparse.extract_from_path(file_path)
    
    if result.mime_type == expected_mime:
        print(f"✓ Valid {expected_mime}")
        return True
    else:
        print(f"✗ Expected {expected_mime}, got {result.mime_type}")
        return False

# Validate a PDF file
validate_file_type("document.pdf", "application/pdf")
```

## Type Checking

Omniparse includes full type hints for static type checking with mypy:

```python
import omniparse
from typing import List, Dict, Any

def process_documents(paths: List[str]) -> List[Dict[str, Any]]:
    """Process multiple documents and return metadata."""
    results: List[Dict[str, Any]] = []
    
    for path in paths:
        result: omniparse.ExtractionResult = omniparse.extract_from_path(path)
        results.append({
            'mime_type': result.mime_type,
            'metadata': result.metadata
        })
    
    return results
```

Run mypy to check types:

```bash
mypy your_script.py
```

## Contributing

Contributions are welcome! Please see the main repository for contribution guidelines.

## License

This project is licensed under either of:

- MIT License
- Apache License, Version 2.0

at your option.

## Links

- **Repository**: https://github.com/sirhco/omniparse
- **Issue Tracker**: https://github.com/sirhco/omniparse/issues
- **PyPI**: https://pypi.org/project/omniparse/

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and changes.
