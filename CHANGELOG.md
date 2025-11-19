# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-11-18

### Added

#### New Format Support (9 formats)
- **HTML Parser**: Extract visible text and metadata from HTML documents
  - Extracts title, description, author, keywords from meta tags
  - Filters out script and style content
  - Preserves paragraph boundaries
  - Confidence: 0.70+

- **CSS Parser**: Analyze and extract metadata from CSS stylesheets
  - Counts CSS rules and selectors
  - Extracts @import statements
  - Returns raw CSS content
  - Confidence: 0.60+

- **RTF Parser**: Extract plain text from Rich Text Format documents
  - Strips RTF control words
  - Extracts metadata (title, author, subject, creation date)
  - Handles embedded objects gracefully
  - Confidence: 0.90+

- **XLSX Parser**: Extract data from modern Excel spreadsheets
  - Supports multiple sheets
  - Extracts calculated values (not formulas)
  - CSV-formatted output per sheet
  - Rich metadata (sheet names, counts, document properties)
  - Handles encrypted files with appropriate errors
  - Confidence: 0.95+

- **PPTX Parser**: Extract text from modern PowerPoint presentations
  - Extracts text from all slides
  - Includes speaker notes
  - Clear slide boundaries
  - Rich metadata (slide count, title, author, dates)
  - Confidence: 0.95+

- **ODS Parser**: Extract data from OpenDocument Spreadsheets
  - Supports multiple tables
  - Handles repeated cells and rows
  - CSV-formatted output per table
  - Metadata extraction (table names, counts, properties)
  - Confidence: 0.95+

- **ODP Parser**: Extract text from OpenDocument Presentations
  - Extracts text from all slides
  - Clear slide boundaries
  - Metadata extraction (slide count, title, author)
  - Confidence: 0.95+

- **XLS Parser**: Extract data from legacy Excel spreadsheets
  - Full sheet extraction using calamine library
  - CSV-formatted output
  - Metadata extraction
  - Confidence: 0.90+

- **DOC Parser**: Extract content from legacy Word documents
  - Basic text extraction from OLE2 structure
  - Metadata extraction (title, author, subject, dates)
  - Handles corrupted files with descriptive errors
  - Confidence: 0.90+

- **PPT Parser**: Extract text from legacy PowerPoint presentations
  - Basic text extraction from OLE2 structure
  - Metadata extraction (slide count, title, author)
  - Confidence: 0.90+

#### Enhanced Detection
- Improved magic bytes detection for all new formats
- HTML detection: `<!DOCTYPE html>`, `<html>`, `<HTML>` patterns
- RTF detection: `{\rtf` signature
- OLE2 detection with format-specific differentiation (DOC/XLS/PPT)
- OpenXML detection for XLSX/PPTX within ZIP archives
- OpenDocument detection for ODS/ODP

#### Performance Improvements
- All formats meet or exceed performance targets:
  - HTML (1 MB): < 100ms (actual: ~0.6ms)
  - XLSX (10K cells): < 500ms (actual: ~0.9ms)
  - PPTX (100 slides): < 1000ms (actual: ~0.6ms)
- Streaming support for large files
- Memory-efficient processing (< 100 MB for files under 50 MB)

#### Security Enhancements
- ZIP bomb protection for ZIP-based formats
- XML bomb protection (limited entity expansion)
- Maximum file size limits per format
- File structure validation before parsing
- Password-protected file detection with clear error messages

#### Documentation
- New [CLI_NEW_FORMATS_GUIDE.md](CLI_NEW_FORMATS_GUIDE.md) with examples for all new formats
- New [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) for upgrading to v0.2.0
- Updated [SUPPORTED_FORMATS.md](SUPPORTED_FORMATS.md) with comprehensive format details
- New [SECURITY_HARDENING.md](SECURITY_HARDENING.md) documenting security measures
- Performance reports and benchmarks
- Code coverage reports (80%+ coverage for new parsers)

#### Examples
- Added examples for all new formats in `examples/` directory
- HTML extraction example
- CSS extraction example
- RTF extraction example
- Spreadsheet extraction example (XLSX, XLS, ODS)
- Presentation extraction example (PPTX, PPT, ODP)
- Legacy Office format examples

### Changed
- Updated README.md with new format support and examples
- Enhanced error handling with format-specific error messages
- Improved API consistency across all parsers
- Standardized metadata field names (e.g., "author" consistently used)

### Dependencies
- Added `scraper` 0.18 for HTML parsing
- Added `cssparser` 0.31 for CSS parsing
- Added `calamine` 0.24 for Excel/spreadsheet support (XLS and XLSX)

### Fixed
- Consistent error handling for encrypted and corrupted files
- Proper handling of partial extraction scenarios
- Memory usage optimization for large files

## [0.1.0] - 2024-XX-XX

### Added
- Initial release
- Support for text formats (TXT, JSON, CSV, XML)
- Support for document formats (PDF, DOCX, ODT)
- Support for image formats (JPEG, PNG, TIFF)
- Support for archive formats (ZIP, TAR)
- CLI interface with multiple output formats
- Library API for Rust applications
- Async support (optional feature)
- Parallel processing (optional feature)
- Automatic type detection using magic bytes
- Rich metadata extraction

[0.2.0]: https://github.com/omniparse/omniparse/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/omniparse/omniparse/releases/tag/v0.1.0
