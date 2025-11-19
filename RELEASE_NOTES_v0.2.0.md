# Omniparse v0.2.0 Release Notes

**Release Date**: November 18, 2025

We're excited to announce Omniparse v0.2.0, a major update that significantly expands format support and enhances the toolkit's capabilities!

## 🎉 What's New

### 9 New Document Formats

This release adds comprehensive support for 9 additional document formats, bringing Omniparse's total format coverage to **35+ MIME types**:

#### Web Formats
- **HTML** - Extract visible text and metadata from web pages and HTML documents
- **CSS** - Analyze stylesheets with rule and selector counting

#### Modern Office Formats
- **XLSX** - Microsoft Excel spreadsheets with multi-sheet support
- **PPTX** - Microsoft PowerPoint presentations with speaker notes

#### OpenDocument Formats
- **ODS** - OpenDocument Spreadsheets
- **ODP** - OpenDocument Presentations

#### Legacy Office Formats
- **XLS** - Legacy Microsoft Excel spreadsheets
- **DOC** - Legacy Microsoft Word documents
- **PPT** - Legacy Microsoft PowerPoint presentations

#### Rich Text
- **RTF** - Rich Text Format documents

## ✨ Key Features

### Comprehensive Extraction
- **Multi-sheet spreadsheets**: Extract data from all sheets in XLSX, XLS, and ODS files
- **Presentation slides**: Extract text from all slides including speaker notes
- **HTML metadata**: Extract title, description, author, keywords from meta tags
- **CSS analysis**: Count rules and selectors, extract @import statements
- **RTF text**: Clean plain text extraction from rich text documents

### Enhanced Detection
- Improved magic bytes detection for all new formats
- Automatic format identification with high confidence (0.60-0.95)
- OLE2 structure detection for legacy Office files
- OpenXML detection within ZIP archives

### Performance Excellence
All new formats meet or exceed performance targets:
- HTML (1 MB): < 100ms (actual: ~0.6ms) ⚡
- XLSX (10K cells): < 500ms (actual: ~0.9ms) ⚡
- PPTX (100 slides): < 1000ms (actual: ~0.6ms) ⚡

### Security Hardening
- ZIP bomb protection for archive-based formats
- XML bomb protection with entity expansion limits
- Password-protected file detection
- File structure validation before parsing
- Maximum file size limits per format

### Rich Metadata
Consistent metadata extraction across all formats:
- Document properties (title, author, subject)
- Creation and modification dates
- Format-specific metadata (sheet names, slide counts, rule counts)
- Standardized field names for consistency

## 📚 Documentation

This release includes extensive new documentation:

- **[CLI_NEW_FORMATS_GUIDE.md](CLI_NEW_FORMATS_GUIDE.md)** - Comprehensive CLI examples for all new formats
- **[MIGRATION_GUIDE.md](MIGRATION_GUIDE.md)** - Upgrade guide from v0.1.0 to v0.2.0
- **[SECURITY_HARDENING.md](SECURITY_HARDENING.md)** - Security measures and best practices
- **[FINAL_PERFORMANCE_SUMMARY.md](FINAL_PERFORMANCE_SUMMARY.md)** - Detailed performance benchmarks
- Updated **[SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md)** - Complete format reference

## 🚀 Quick Start

### Installation

Update your `Cargo.toml`:

```toml
[dependencies]
omniparse = "0.2.0"
```

Or install the CLI:

```bash
cargo install omniparse
```

### Usage Examples

#### Extract from HTML
```bash
omniparse webpage.html
omniparse --format json --metadata-only page.html
```

#### Extract from Spreadsheets
```bash
omniparse data.xlsx spreadsheet.xls budget.ods
omniparse --parallel *.xlsx *.xls *.ods
```

#### Extract from Presentations
```bash
omniparse slides.pptx presentation.ppt deck.odp
omniparse --metadata-only quarterly-review.pptx
```

#### Library API
```rust
use omniparse::extract_from_path;

fn main() -> Result<(), omniparse::Error> {
    // Extract from any supported format
    let result = extract_from_path("document.xlsx")?;
    
    println!("MIME type: {}", result.mime_type);
    println!("Content: {}", result.content);
    
    // Access metadata
    if let Some(author) = result.metadata.author() {
        println!("Author: {}", author);
    }
    
    Ok(())
}
```

## 🔧 Technical Details

### New Dependencies
- `scraper` 0.18 - HTML parsing
- `cssparser` 0.31 - CSS parsing
- `calamine` 0.24 - Excel/spreadsheet support

### API Consistency
- All parsers implement the same `Parser` trait
- Consistent `ExtractionResult` structure
- Standardized metadata field names
- Uniform error handling patterns

### Error Handling
Enhanced error messages for:
- Password-protected files
- Corrupted documents
- Unsupported file versions
- Partial extraction scenarios

## 📊 Performance Benchmarks

Comprehensive benchmarks show excellent performance:

| Format | File Size | Target | Actual | Status |
|--------|-----------|--------|--------|--------|
| HTML | 1 MB | < 100ms | ~0.6ms | ✅ Exceeded |
| CSS | 100 KB | < 50ms | ~0.3ms | ✅ Exceeded |
| RTF | 500 KB | < 100ms | ~1.2ms | ✅ Exceeded |
| XLSX | 10K cells | < 500ms | ~0.9ms | ✅ Exceeded |
| PPTX | 100 slides | < 1000ms | ~0.6ms | ✅ Exceeded |
| ODS | 5K cells | < 500ms | ~0.8ms | ✅ Exceeded |
| ODP | 50 slides | < 500ms | ~0.5ms | ✅ Exceeded |

See [FINAL_PERFORMANCE_SUMMARY.md](FINAL_PERFORMANCE_SUMMARY.md) for complete results.

## 🧪 Testing

- 80%+ code coverage for all new parsers
- Comprehensive unit tests for each format
- Integration tests for end-to-end extraction
- Performance benchmarks
- Security tests for malicious files

## 🔄 Migration from v0.1.0

The v0.2.0 release is **fully backward compatible** with v0.1.0. All existing code will continue to work without changes.

New formats are automatically available:
- No configuration changes needed
- Same API patterns
- Consistent behavior

See [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) for details.

## 🐛 Bug Fixes

- Improved error handling for encrypted and corrupted files
- Better memory management for large files
- Fixed edge cases in type detection
- Enhanced partial extraction handling

## 🙏 Acknowledgments

This release represents a significant expansion of Omniparse's capabilities, bringing it closer to feature parity with Apache Tika while maintaining the performance and safety benefits of Rust.

Special thanks to all contributors and users who provided feedback and suggestions!

## 📝 Full Changelog

See [CHANGELOG.md](CHANGELOG.md) for the complete list of changes.

## 🔗 Links

- **Repository**: https://github.com/omniparse/omniparse
- **Documentation**: Run `cargo doc --open`
- **Issues**: https://github.com/omniparse/omniparse/issues
- **Crates.io**: https://crates.io/crates/omniparse

## 🚦 What's Next

Future releases will focus on:
- Additional format support (e.g., EPUB, Markdown, more image formats)
- Enhanced OCR capabilities
- Improved streaming for very large files
- Additional metadata extraction
- Performance optimizations

---

**Upgrade today and unlock support for 9 new document formats!** 🎊
