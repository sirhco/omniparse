# Omniparse Supported File Formats

This document provides a comprehensive list of all file formats supported by Omniparse, organized by category.

## Quick Reference

| Format | Extensions | MIME Type(s) | Status |
|--------|-----------|--------------|--------|
| Plain Text | `.txt` | `text/plain` | ✅ Supported |
| JSON | `.json` | `application/json`, `text/json` | ✅ Supported |
| CSV | `.csv` | `text/csv` | ✅ Supported |
| TSV | `.tsv` | `text/tab-separated-values` | ✅ Supported |
| XML | `.xml` | `application/xml`, `text/xml` | ✅ Supported |
| PDF | `.pdf` | `application/pdf` | ✅ Supported |
| DOCX | `.docx` | `application/vnd.openxmlformats-officedocument.wordprocessingml.document` | ✅ Supported |
| ODT | `.odt` | `application/vnd.oasis.opendocument.text` | ✅ Supported |
| JPEG | `.jpg`, `.jpeg` | `image/jpeg`, `image/jpg` | ✅ Supported |
| PNG | `.png` | `image/png` | ✅ Supported |
| TIFF | `.tif`, `.tiff` | `image/tiff`, `image/tif` | ✅ Supported |
| ZIP | `.zip` | `application/zip`, `application/x-zip-compressed` | ✅ Supported |
| TAR | `.tar` | `application/x-tar`, `application/tar` | ✅ Supported |
| HTML | `.html`, `.htm` | `text/html` | ✅ Supported |
| CSS | `.css` | `text/css` | ✅ Supported |
| RTF | `.rtf` | `application/rtf` | ✅ Supported |
| DOC | `.doc` | `application/msword` | ✅ Supported |
| XLS | `.xls` | `application/vnd.ms-excel` | ✅ Supported |
| XLSX | `.xlsx` | `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` | ✅ Supported |
| PPT | `.ppt` | `application/vnd.ms-powerpoint` | ✅ Supported |
| PPTX | `.pptx` | `application/vnd.openxmlformats-officedocument.presentationml.presentation` | ✅ Supported |
| ODS | `.ods` | `application/vnd.oasis.opendocument.spreadsheet` | ✅ Supported |
| ODP | `.odp` | `application/vnd.oasis.opendocument.presentation` | ✅ Supported |

## Summary

Omniparse currently supports **35+ MIME types** across **4 major categories**:

- **Text Formats**: 10 MIME types
- **Document Formats**: 19 MIME types  
- **Image Formats**: 4 MIME types
- **Archive Formats**: 4 MIME types

---

## Supported vs Requested Formats Comparison

| MIME Type | Status | Category |
|-----------|--------|----------|
| `text/plain` | ✅ **Supported** | Text |
| `text/html` | ✅ **Supported** | Text |
| `text/css` | ✅ **Supported** | Text |
| `text/csv` | ✅ **Supported** | Text |
| `text/xml` | ✅ **Supported** | Text |
| `application/pdf` | ✅ **Supported** | Document |
| `application/msword` (.doc) | ✅ **Supported** | Document |
| `application/vnd.openxmlformats-officedocument.wordprocessingml.document` (.docx) | ✅ **Supported** | Document |
| `application/vnd.ms-excel` (.xls) | ✅ **Supported** | Document |
| `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` (.xlsx) | ✅ **Supported** | Document |
| `application/vnd.ms-powerpoint` (.ppt) | ✅ **Supported** | Document |
| `application/vnd.openxmlformats-officedocument.presentationml.presentation` (.pptx) | ✅ **Supported** | Document |
| `application/rtf` | ✅ **Supported** | Document |
| `application/vnd.oasis.opendocument.text` (.odt) | ✅ **Supported** | Document |
| `application/vnd.oasis.opendocument.spreadsheet` (.ods) | ✅ **Supported** | Document |
| `application/vnd.oasis.opendocument.presentation` (.odp) | ✅ **Supported** | Document |

**Summary**: 16 out of 16 requested formats are now supported (100%)

---

## Text Formats (10 MIME types)

### Plain Text
- **MIME Types**: `text/plain`
- **Extensions**: `.txt`
- **Features**:
  - Automatic encoding detection (UTF-8, UTF-16LE, UTF-16BE, ASCII)
  - BOM (Byte Order Mark) handling
  - Character and line count extraction
- **Metadata Extracted**:
  - `character_count`: Number of characters
  - `line_count`: Number of lines
  - `encoding`: Detected encoding (UTF-8, UTF-16LE, UTF-16BE, ASCII)

### JSON
- **MIME Types**: `application/json`, `text/json`
- **Extensions**: `.json`
- **Features**:
  - Full JSON parsing and validation
  - Schema structure analysis
  - Nested object and array support
- **Metadata Extracted**:
  - `valid`: JSON validity (boolean)
  - `schema_info`: Structure description (e.g., "object{author, data, name}")

### CSV
- **MIME Types**: `text/csv`, `text/tab-separated-values`
- **Extensions**: `.csv`, `.tsv`
- **Features**:
  - Automatic delimiter detection (comma, semicolon, tab, pipe)
  - Header extraction
  - Row and column counting
- **Metadata Extracted**:
  - `column_count`: Number of columns
  - `row_count`: Number of data rows
  - `headers`: List of column headers
  - `delimiter`: Detected delimiter type

### XML
- **MIME Types**: `application/xml`, `text/xml`
- **Extensions**: `.xml`
- **Features**:
  - Full XML parsing
  - Namespace extraction
  - Root element identification
  - Text content extraction
- **Metadata Extracted**:
  - `root_element`: Root XML element name
  - `namespaces`: List of XML namespaces

### HTML
- **MIME Types**: `text/html`
- **Extensions**: `.html`, `.htm`
- **Features**:
  - HTML parsing with DOM structure
  - Visible text extraction (excludes scripts and styles)
  - Meta tag extraction
  - Paragraph boundary preservation
- **Metadata Extracted**:
  - `title`: Document title from `<title>` tag
  - `description`: Meta description
  - `author`: Meta author
  - `keywords`: Meta keywords
  - `charset`: Character encoding
  - `language`: Document language from `<html lang>` attribute

### CSS
- **MIME Types**: `text/css`
- **Extensions**: `.css`
- **Features**:
  - CSS parsing and validation
  - Rule and selector counting
  - @import statement extraction
  - Raw CSS content preservation
- **Metadata Extracted**:
  - `rule_count`: Number of CSS rules
  - `selector_count`: Number of selectors
  - `imports`: List of @import URLs
  - `charset`: Character set from @charset rule

### RTF (Rich Text Format)
- **MIME Types**: `application/rtf`
- **Extensions**: `.rtf`
- **Features**:
  - RTF control word stripping
  - Plain text extraction
  - Metadata extraction from \info group
  - Embedded object handling
- **Metadata Extracted**:
  - `rtf_version`: RTF version number
  - `title`: Document title
  - `author`: Document author
  - `subject`: Document subject
  - `creation_date`: Creation timestamp

---

## Document Formats (19 MIME types)

### PDF
- **MIME Types**: `application/pdf`
- **Extensions**: `.pdf`
- **Features**:
  - Multi-page text extraction
  - Document metadata extraction
  - Page counting
  - Form-field, annotation, attachment counts (`form_fields_count`,
    `annotations_count`, `attachments_count`)
  - Optional OCR fallback for image-only / scanned PDFs (DCTDecode images,
    `ocr` or `ocr-ml` feature required)
  - **Lenient parsing** (v0.4.1+): three-tier fallback chain so real-world
    PDFs with trailing garbage, missing trailers, or corrupted xref still
    yield text. See [PDF parsing tiers](#pdf-parsing-tiers) below.
- **Metadata Extracted**:
  - `page_count`: Number of pages
  - `pdf_version`: PDF spec version (e.g. `"1.4"`, `"2.0"`)
  - `encrypted`: Whether the document has an `/Encrypt` dictionary
  - `title`, `author`, `subject`, `creator`, `producer`, `keywords`:
    document Info fields
  - `creation_date`, `modification_date`
  - `page_layout`, `page_mode`: catalog hints
  - `form_fields_count`, `annotations_count`, `attachments_count`
  - `pdf_parse_strategy`: which tier extracted the content
    (`strict` / `repaired_xref` / `raw_scan`)
  - `pdf_parse_partial`: `true` when a fallback tier ran (some metadata
    may be missing)
  - `pdf_parse_error`: the strict-tier error string when `raw_scan` ran

#### PDF parsing tiers

1. **strict** — `lopdf::Document::load_mem`. Full metadata, structured
   per-page text. Works on most well-formed PDFs.
2. **repaired_xref** — scans backward for the last `%%EOF` marker,
   truncates trailing junk, retries strict load. Recovers PDFs with
   appended HTTP-chunk leftovers, double-`%%EOF` exports, etc. Same
   output shape as strict.
3. **raw_scan** — walks every `stream` / `endstream` pair in the bytes,
   tries each supported PDF stream filter in order (FlateDecode, LZWDecode,
   ASCII85Decode, uncompressed), and regex-extracts `(literal) Tj` and
   `[...] TJ` content operators from any decoder output that contains
   text-operator tokens. No structural parse → no per-page split, no rich
   metadata, but recovers text from PDFs lopdf can't load.
   Output is gated by a "looks-like-text" heuristic (≥60% printable
   chars + at least one 4-char alphanumeric run); if the recovered bytes
   are glyph indices (custom font `/Encoding` or `/ToUnicode` CMap), encrypted
   noise, or use a stream filter we don't decode, the tier falls through
   to tier 4 (when enabled) or surfaces a parse error.
4. **pdf_extract** (optional, requires `--features pdf-extract`) —
   re-parse via the [`pdf-extract`](https://crates.io/crates/pdf-extract)
   crate. Different structural parser that tolerates linearized PDFs and
   Identity-H + /ToUnicode CMaps — catches Lucidchart exports, modern
   Word/PowerPoint print-to-PDF, browser print-to-PDF, and similar
   real-world inputs that all three lopdf-based tiers reject. Same
   `looks_like_text` gate applies. Text-only (no per-page split, no rich
   metadata beyond `pdf_version` scanned from the header).

#### Known PDF limitations

- `FlateDecode`, `JPXDecode`, `CCITTFaxDecode` image filters: OCR path
  silently skips affected images (only `DCTDecode` / JPEG flows through).
- Encrypted PDFs: no user-password support; `encrypted: true` is reported
  but content extraction will fail.
- Vector-only / no-text-layer / no-image PDFs: nothing to extract.
- Severely truncated files (header gone, no recoverable streams): hard
  error.
- Repair the `qpdf` way for files our parser still can't handle:
  ```sh
  qpdf --linearize broken.pdf fixed.pdf
  ```

### Microsoft Word (DOCX)
- **MIME Types**: 
  - `application/vnd.openxmlformats-officedocument.wordprocessingml.document`
  - `application/docx` (alternative)
- **Extensions**: `.docx`
- **Features**:
  - Full text extraction from document.xml
  - Core properties extraction
  - Paragraph-aware text extraction
- **Metadata Extracted**:
  - `title`: Document title
  - `author`: Document author (from creator field)
  - `subject`: Document subject
  - `description`: Document description
  - `creation_date`: Creation date
  - `modified_date`: Last modification date
  - `last_modified_by`: Last modifier
  - `revision`: Revision number

### Microsoft Word (DOC) - Legacy
- **MIME Types**: `application/msword`
- **Extensions**: `.doc`
- **Features**:
  - OLE2 structure parsing
  - Text stream extraction
  - Document properties extraction
- **Metadata Extracted**:
  - `title`: Document title
  - `author`: Document author
  - `subject`: Document subject
  - `creation_date`: Creation date
- **Note**: Legacy binary format with limited extraction capabilities compared to DOCX

### Microsoft Excel (XLSX)
- **MIME Types**: 
  - `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`
  - `application/xlsx` (alternative)
- **Extensions**: `.xlsx`
- **Features**:
  - Multi-sheet extraction
  - Cell value extraction (calculated values, not formulas)
  - CSV-like formatting per sheet
  - Document properties extraction
- **Metadata Extracted**:
  - `sheet_count`: Number of sheets
  - `sheet_names`: List of sheet names
  - `total_rows`: Total rows across all sheets
  - `total_columns`: Maximum columns across sheets
  - `author`: Document author
  - `title`: Document title
  - `creation_date`: Creation timestamp
- **Error Handling**: Detects password-protected and corrupted files

### Microsoft Excel (XLS) - Legacy
- **MIME Types**: `application/vnd.ms-excel`
- **Extensions**: `.xls`
- **Features**:
  - Multi-sheet extraction using calamine library
  - Cell value extraction
  - CSV-like formatting per sheet
- **Metadata Extracted**:
  - `sheet_count`: Number of sheets
  - `sheet_names`: List of sheet names
  - Document properties if available

### Microsoft PowerPoint (PPTX)
- **MIME Types**: 
  - `application/vnd.openxmlformats-officedocument.presentationml.presentation`
  - `application/pptx` (alternative)
- **Extensions**: `.pptx`
- **Features**:
  - Multi-slide text extraction
  - Speaker notes extraction
  - Slide boundary preservation
  - Document properties extraction
- **Metadata Extracted**:
  - `slide_count`: Number of slides
  - `title`: Presentation title
  - `author`: Presentation author
  - `subject`: Presentation subject
  - `creation_date`: Creation timestamp
  - `has_notes`: Whether speaker notes exist

### Microsoft PowerPoint (PPT) - Legacy
- **MIME Types**: `application/vnd.ms-powerpoint`
- **Extensions**: `.ppt`
- **Features**:
  - OLE2 structure parsing
  - Text extraction from PowerPoint streams
  - Basic metadata extraction
- **Metadata Extracted**:
  - `slide_count`: Number of slides (if available)
  - `title`: Presentation title
  - `author`: Presentation author
- **Note**: Legacy binary format with limited extraction capabilities compared to PPTX

### OpenDocument Text (ODT)
- **MIME Types**: 
  - `application/vnd.oasis.opendocument.text`
  - `application/odt` (alternative)
- **Extensions**: `.odt`
- **Features**:
  - Full text extraction from content.xml
  - Comprehensive metadata extraction
  - Paragraph-aware text extraction
- **Metadata Extracted**:
  - `title`: Document title
  - `author`: Document author
  - `subject`: Document subject
  - `description`: Document description
  - `creation_date`: Creation date
  - `modified_date`: Last modification date
  - `generator`: Creating application
  - `editing_cycles`: Number of editing sessions
  - `page_count`: Number of pages
  - `word_count`: Word count
  - `character_count`: Character count

### OpenDocument Spreadsheet (ODS)
- **MIME Types**: 
  - `application/vnd.oasis.opendocument.spreadsheet`
  - `application/ods` (alternative)
- **Extensions**: `.ods`
- **Features**:
  - Multi-table extraction from content.xml
  - Cell value extraction with repeated cell/row handling
  - CSV-like formatting per table
  - Metadata extraction from meta.xml
- **Metadata Extracted**:
  - `table_count`: Number of tables/sheets
  - `table_names`: List of table names
  - `author`: Document author
  - `title`: Document title
  - `creation_date`: Creation timestamp

### OpenDocument Presentation (ODP)
- **MIME Types**: 
  - `application/vnd.oasis.opendocument.presentation`
  - `application/odp` (alternative)
- **Extensions**: `.odp`
- **Features**:
  - Multi-slide text extraction from content.xml
  - Slide boundary preservation
  - Metadata extraction from meta.xml
- **Metadata Extracted**:
  - `slide_count`: Number of slides
  - `title`: Presentation title
  - `author`: Presentation author
  - `creation_date`: Creation timestamp

---

## Image Formats (4 MIME types)

### JPEG
- **MIME Types**: `image/jpeg`, `image/jpg`
- **Extensions**: `.jpg`, `.jpeg`
- **Features**:
  - Dimension extraction
  - Color type detection
  - EXIF metadata detection
- **Metadata Extracted**:
  - `width`: Image width in pixels
  - `height`: Image height in pixels
  - `color_type`: Color space information
  - `exif_present`: Whether EXIF data is present (boolean)
- **Content**: None (metadata only)

### PNG
- **MIME Types**: `image/png`
- **Extensions**: `.png`
- **Features**:
  - Dimension extraction
  - Color type detection
  - PNG metadata chunk extraction (tEXt, iTXt, zTXt)
- **Metadata Extracted**:
  - `width`: Image width in pixels
  - `height`: Image height in pixels
  - `color_type`: Color space information
  - `text_*`: Text chunks from PNG metadata
  - `itext_*`: International text chunks
  - `ztext_*`: Compressed text chunks
- **Content**: None (metadata only)

### TIFF
- **MIME Types**: `image/tiff`, `image/tif`
- **Extensions**: `.tif`, `.tiff`
- **Features**:
  - Dimension extraction
  - Multi-page detection
  - TIFF tag extraction
  - Byte order detection
- **Metadata Extracted**:
  - `width`: Image width in pixels
  - `height`: Image height in pixels
  - `color_type`: Color space information
  - `byte_order`: Endianness (little-endian or big-endian)
  - `page_count`: Number of pages/IFDs
  - `multi_page`: Whether file contains multiple pages (boolean)
  - `tiff_*`: TIFF-specific tags
- **Content**: None (metadata only)

---

## Archive Formats (4 MIME types)

### ZIP
- **MIME Types**: 
  - `application/zip`
  - `application/x-zip-compressed` (alternative)
- **Extensions**: `.zip`
- **Features**:
  - File listing extraction
  - Size calculation (compressed and uncompressed)
  - Compression ratio calculation
- **Metadata Extracted**:
  - `file_count`: Number of files in archive
  - `total_size`: Total uncompressed size in bytes
  - `compressed_size`: Total compressed size in bytes
  - `compression_ratio`: Compression ratio (0.0 to 1.0)
  - `files`: List of file paths in archive
- **Content**: Text listing of archive contents

### TAR
- **MIME Types**: 
  - `application/x-tar`
  - `application/tar` (alternative)
- **Extensions**: `.tar`, `.tar.gz`, `.tgz`
- **Features**:
  - File listing with details
  - Size calculation
  - Modification time extraction
- **Metadata Extracted**:
  - `file_count`: Number of files in archive
  - `total_size`: Total size in bytes
  - `files`: List of files with size and modification time
- **Content**: Text listing of archive contents

---

## Detection Methods

Omniparse uses multiple methods to detect file types, in order of priority:

1. **Magic Bytes**: Binary signatures at the start of files (most reliable)
2. **Content Analysis**: Analyzing file structure and patterns
3. **Extension Fallback**: Using file extension as a last resort

### Detection Confidence Levels

Each detection method provides a confidence score:
- **Magic Bytes**: 0.95 (95% confidence)
- **Content Analysis**: 0.70 (70% confidence)
- **Extension**: 0.50 (50% confidence)

---

## Usage Examples

### Check Supported Formats Programmatically

```rust
use omniparse::{supported_mime_types, is_mime_supported};

// Get all supported MIME types
let types = supported_mime_types();
println!("Supported formats: {}", types.len());

for mime_type in types {
    println!("  - {}", mime_type);
}

// Check if a specific format is supported
if is_mime_supported("application/pdf") {
    println!("PDF is supported!");
}
```

### CLI Usage

```bash
# Check if a file type is supported
omniparse --detect-only unknown_file.bin

# Extract from a supported format
omniparse document.pdf
omniparse data.json
omniparse archive.zip
```

---

## Recently Added Formats

The following formats have been recently added to Omniparse:

### Text Formats
- ✅ **HTML** (`text/html`) - HyperText Markup Language with DOM parsing
- ✅ **CSS** (`text/css`) - Cascading Style Sheets with rule analysis
- ✅ **RTF** (`application/rtf`) - Rich Text Format with control word parsing

### Document Formats
- ✅ **Legacy Word** (`application/msword`, `.doc`) - Old Microsoft Word format via OLE2 parsing
- ✅ **Excel** (`application/vnd.ms-excel`, `.xls`) - Old Microsoft Excel format
- ✅ **Excel XLSX** (`application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`, `.xlsx`) - Modern Excel format
- ✅ **PowerPoint** (`application/vnd.ms-powerpoint`, `.ppt`) - Old PowerPoint format
- ✅ **PowerPoint PPTX** (`application/vnd.openxmlformats-officedocument.presentationml.presentation`, `.pptx`) - Modern PowerPoint format
- ✅ **ODS** (`application/vnd.oasis.opendocument.spreadsheet`) - OpenDocument Spreadsheet
- ✅ **ODP** (`application/vnd.oasis.opendocument.presentation`) - OpenDocument Presentation

### Implementation Notes
- **HTML/CSS**: Implemented using specialized parsers (scraper, cssparser)
- **Legacy Office formats (.doc, .xls, .ppt)**: Use OLE2 structure parsing with calamine for XLS
- **Spreadsheets (XLSX, ODS)**: Full cell extraction with formula value support
- **Presentations (PPTX, ODP)**: Complete slide and speaker notes extraction
- **RTF**: Custom RTF control word parser

---

## Planned Future Support

The following formats are prioritized for future releases:

### High Priority
- **Markdown** (`text/markdown`) - Markdown documents
- **EPUB** - E-book format
- **Email formats** (EML, MSG) - Email messages

### Images
- GIF
- WebP
- BMP
- SVG (currently detected but not parsed)

### Archives
- GZIP
- BZIP2
- 7-Zip
- RAR

### Other
- MOBI - Kindle e-book format
- Additional specialized formats based on user demand

---

## Contributing New Format Support

To add support for a new file format:

1. Create a new parser in the appropriate category directory
2. Implement the `Parser` trait
3. Register the parser in `ParserRegistry::default()`
4. Add magic bytes detection in `src/detection/magic.rs`
5. Add tests in `tests/parser_tests.rs`
6. Update this documentation

See the [Contributing Guide](CONTRIBUTING.md) for detailed instructions.

---

## Format Compatibility Notes

### DOCX and ZIP
DOCX files are ZIP archives internally. Omniparse correctly identifies them as DOCX rather than ZIP based on their internal structure.

### TAR Compression
TAR files can be compressed with GZIP (.tar.gz, .tgz) or BZIP2 (.tar.bz2). Currently, only uncompressed TAR files are fully supported. Compressed TAR files may require decompression first.

### XML-based Formats
Many modern document formats (DOCX, ODT, SVG) are XML-based. Omniparse handles these with format-specific parsers rather than the generic XML parser.

### Image Content
Image parsers extract metadata only, not pixel data. Use the `image` crate directly if you need to process image pixels.

---

## Performance Characteristics

| Format Category | Typical Processing Time | Memory Usage |
|----------------|------------------------|--------------|
| Text (< 1MB) | < 10ms | Low |
| JSON/CSV | 10-50ms | Low |
| XML | 20-100ms | Medium |
| PDF | 200-500ms | Medium-High |
| DOCX/ODT | 50-200ms | Medium |
| Images | 20-100ms | Low |
| Archives | 50-200ms | Low-Medium |

*Times measured on standard hardware with typical file sizes*

---

## API Reference

For detailed API documentation, see:
- [Main README](README.md)
- [API Documentation](https://docs.rs/omniparse)
- [Examples](examples/)
