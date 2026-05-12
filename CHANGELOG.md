# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-05-12

### Added

#### OCR model management
- New `omniparse models` CLI subcommand with four actions: `download [--force]`,
  `verify`, `path`, `list`. Pre-fetch ML OCR models in CI / containers / air-
  gapped installs instead of relying on the silent first-run download. Requires
  the `ocr-ml` feature.
- Public API in `omniparse::ocr::ml`: `ModelSpec`, `MODELS` constant slice with
  pinned SHA-256 hashes, `prefetch_all`, `verify_all`, `list_models`,
  `ModelStatus`. The previously-unused `sha2` dependency is now wired into the
  download path: streamed hashes are compared to the pinned digest, and
  cached files are re-verified on every `MlOcrEngine::new()` call.
- `OcrError::ChecksumMismatch` now actually fires (was declared but never
  returned).

#### Unified OCR runtime gate
- `OMNIPARSE_OCR` is now tri-state: `off` / `classical` / `ml`. New
  `omniparse::ocr::OcrMode` enum and `omniparse::ocr::ocr_mode()` reader.
- Legacy `OMNIPARSE_OCR=1` + `OMNIPARSE_OCR_ML=1` is still honored but emits a
  one-shot stderr deprecation warning. Will be removed in 0.5.

#### Web service + container
- `examples/web_service.rs` now reads `OMNIPARSE_BIND` (default
  `127.0.0.1:3000`) so the example can be containerized without a code edit.
- New project-root `Dockerfile`: cargo-chef-cached multi-stage build (planner →
  cook → builder → models → distroless runtime). ML OCR models are downloaded
  and SHA-256-verified at build time, then copied into the final image at
  `/opt/omniparse/models`. Final image runs distroless as UID 65532.
- `.dockerignore` and `docker-compose.yml` to make `docker compose up --build`
  the one-command local-dev path.
- `.github/workflows/release-docker.yml`: tag-triggered (`v*.*.*`) +
  `workflow_dispatch` multi-arch (`linux/amd64`, `linux/arm64`) build that
  publishes to `ghcr.io/<owner>/omniparse-web`. Uses GitHub Actions build cache.

### Changed
- README OCR section condensed to a 30-second quickstart; the long-form
  guide is now the single canonical reference in `OCR_GUIDE.md`.
- `OCR_GUIDE.md` restructured: "30-second start" block at the top, dedicated
  "Managing the model cache" section documenting the new CLI, all examples
  updated to the unified `OMNIPARSE_OCR=ml|classical` env var.

## [0.3.0] - 2026-04-22

### Added

#### Core improvements
- Cached default `ParserRegistry` via `std::sync::OnceLock`. New free function
  `parsers::default_registry()` returns a `&'static ParserRegistry` — hot paths
  no longer rebuild the registry on every call.
- Real EXIF extraction via `kamadak-exif` for JPEG and TIFF. New metadata keys:
  `camera_make`, `camera_model`, `software`, `artist`, `copyright`,
  `image_description`, `lens_model`, `orientation`, `iso`, `focal_length_mm`,
  `f_number`, `exposure_time_sec`, `exposure_bias`, `focal_length_35mm`,
  `datetime`, `datetime_original`, `datetime_digitized`, `gps_latitude`,
  `gps_longitude`.
- PNG zTXt and compressed iTXt now zlib-inflated via `flate2`; values are the
  decompressed text instead of a placeholder.
- HTML parser now extracts OpenGraph (`og_*`), Twitter Card (`twitter_*`),
  canonical URL (`canonical_url`), viewport, robots, http-equiv
  (`http_equiv_*`) meta tags, and heading-tag counts
  (`heading_h1_count` … `heading_h6_count`).
- PDF parser emits `pdf_version`, `encrypted`, `form_fields_count`,
  `annotations_count`, `attachments_count`, `page_layout`, `page_mode`,
  `modification_date`, `keywords`.
- ZIP/TAR parsers report `contains_unsafe_paths` after scanning entries for
  `..`, absolute paths, and Windows drive prefixes. New helper
  `utils::security::is_safe_archive_path` and constant `MAX_ARCHIVE_DEPTH`.
- ZIP parser now walks the archive in a single pass instead of three.

#### OCR subsystem (`ocr` Cargo feature, default off)
- New `omniparse::ocr` module implementing a pure-Rust classical OCR pipeline:
  preprocess (imageproc Otsu / median / Hough deskew) → layout
  (connected-component analysis) → recognize (hand-crafted 29-dim feature
  vector + 1-NN classifier over labeled prototypes) → postprocess (SymSpell
  dictionary correction).
- Bundled bitmap glyph prototypes for uppercase A–Z and digits 0–9.
- `OcrEngine`, `OcrEngineBuilder`, `OcrConfig`, `OcrOutput`, `RecognizedLine`,
  `TextRegion`, `Prototype`, and per-stage traits (`Preprocessor`,
  `LayoutAnalyzer`, `Recognizer`, `PostProcessor`).
- Standalone entry point `omniparse::ocr::extract_text_from_image(path)`.
- Image parsers (JPEG, PNG, TIFF) optionally run OCR at parse time when both
  the `ocr` feature is enabled and the `OMNIPARSE_OCR=1` environment variable
  is set. Populates `Content::Text` and `ocr_applied` / `ocr_confidence`
  metadata. When either condition is absent, image parsers behave identically
  to prior releases.
- Explicitly **no machine-learning runtime and no downloaded models**. Every
  reference is shipped in-crate.
- Runtime confidence filter: `OcrConfig::min_confidence` defaults to `0.15`,
  overridable via `OMNIPARSE_OCR_MIN_CONFIDENCE=<float>`. Lines below the
  threshold are dropped — noisy inputs yield empty text rather than garbage.
- New optional `ocr-train` Cargo feature adds a TTF/OTF glyph trainer
  (`omniparse::ocr::train`) backed by `ab_glyph`. `FeatureRecognizer` can now
  be seeded from a user-supplied font. `prototypes::save_prototypes_json` and
  `prototypes::load_prototypes_json` round-trip a labeled set to disk; the
  shared engine honors `OMNIPARSE_OCR_PROTOTYPES=<json-path>` to swap the
  runtime prototype set without rebuilding.
- New example `examples/train_prototypes.rs` wraps the end-to-end workflow:
  font + char set + pixel size → JSON prototype file.
- Line/word spatial grouping in `OcrEngine::recognize`: glyph regions are
  now clustered into lines by vertical overlap and ordered left-to-right
  within each line; word boundaries are inserted when horizontal gaps exceed
  40% of the line's median glyph width.
- Multi-scale training (`omniparse::ocr::train::train_multiscale` +
  `train_multiscale_from_path`) concatenates prototype sets across several
  pixel sizes. The `train_prototypes` example now accepts a comma-separated
  size list (e.g. `24,48,96`).
- New `omniparse::ocr::swt` module implementing the Stroke-Width Transform
  (Epshtein et al., CVPR 2010) as an alternative `LayoutAnalyzer`. Useful
  for photographic inputs where text-on-image segmentation floods the
  default connected-component analyzer.
- `OMNIPARSE_OCR_LAYOUT=cca|swt` env var picks the runtime layout analyzer
  on the shared engine.

#### New format parsers
- **Markdown** (`markdown` feature, on by default): `pulldown-cmark` 0.10
  driven. Strips formatting into plain text and emits title (first H1),
  heading counts per level, code-block count, link/image counts.
- **SVG** (`svg` feature, on by default): extracts `<title>`, `<desc>`,
  `viewBox`, root `width`/`height`/`xmlns`, all `<text>` nodes, and per-shape
  element counts (path, rect, circle, ellipse, line, polyline, polygon, g,
  use, image). Magic-byte detector tightened to require a literal `<svg`
  (the prior `<?xml` pattern swallowed every XML file).
- **WebP** (`webp` feature, on by default): dimensions, color type, and EXIF
  via the shared `kamadak-exif` helper. Integrates with the OCR hookup path.
- **EPUB** (`epub` feature, on by default): OPF metadata (title, author,
  publisher, language, identifier, publication_date, keywords, rights,
  description), spine/resource counts, and concatenated chapter text walked
  in reading order. Extension-based detection (`.epub`).
- **MP3** (`mp3` feature, on by default): ID3v1 / ID3v2 tags — title,
  artist, album, album_artist, genre, year, track, total_tracks, disc,
  duration_ms. Comment frames surface as `Content::Text`.
- New parser category `parsers::audio` scaffolded for future audio formats.

#### OCR robustness improvements
- **Sauvola adaptive binarization** (`BinarizeMode::Sauvola { window, k, r }`)
  plus `AdaptiveMean` mode. Integral-image based, O(W·H) per pass. Picks a
  per-pixel threshold from its local neighborhood — dramatically better than
  global Otsu on gradient-lit or shadowed inputs.
- **CLAHE** (contrast-limited adaptive histogram equalization) gated via
  `PreprocessConfig::clahe`. Bilinear-interpolated tile grid, configurable
  grid size and clip limit.
- **Morphological top-hat** background subtraction (`tophat_radius`),
  optional pre-binarization step.
- **Auto-rotation 90/180/270°** via `OcrConfig::auto_rotate` or
  `OMNIPARSE_OCR_AUTO_ROTATE=1`. Runs the pipeline four times and keeps the
  orientation with the highest `text_length × mean_confidence` score.
- **Color-based foreground segmentation** (`omniparse::ocr::color_seg`):
  pixel k-means, auto-picks the text cluster by size + luminance heuristics,
  returns a binary mask the layout stage can consume directly. Aimed at
  text-over-photo inputs.
- **k-NN voting** in `FeatureRecognizer` via `with_k(k)`. Top-k nearest
  neighbors vote with inverse-distance weights; still defaults to `k = 1`
  for backward compatibility.
- **Extended feature vector** (`FEATURE_COUNT` bumped from 29 to 32):
  average row crossings, average column crossings, approximate hole count
  (Euler number). **Breaking**: prototype JSON files generated against the
  old count fail to deserialize — retrain with the updated
  `train_prototypes` example.
- **Pre-recognition line grouping** utility
  (`layout::group_regions_into_lines`, `line_median_height`) exposed for
  downstream callers that want to normalize glyph size before classification.
- **Diagnostic image dumps**: set `OMNIPARSE_OCR_DEBUG_DIR=/path/to/dir` to
  write `01_input.png`, `02_preprocessed.png`, and `03_layout.png` (with
  detected region bounding boxes overlaid) on every `recognize()` call.
  Invaluable for tuning.

#### OCR: ML backend via `ocrs` + `rten`
- New optional `ocr-ml` Cargo feature pulls pure-Rust ML runtime (ocrs 0.12
  + rten 0.24). Backed by downloadable pre-trained detection + recognition
  models (~30 MB, one-time fetch to the user cache directory).
- `omniparse::ocr::ml::MlOcrEngine` wraps `ocrs::OcrEngine` and exposes the
  same `recognize(DynamicImage) -> OcrOutput` API as the classical path.
- Runtime dispatch: when `OMNIPARSE_OCR=1` AND `OMNIPARSE_OCR_ML=1` are
  both set and the crate is built with `--features ocr-ml`, image parsers
  route through the ML backend instead of the classical pipeline.
- Model cache directory defaults to `dirs::cache_dir()/omniparse/ocrs-models/`;
  override with `OMNIPARSE_OCR_MODELS=<path>`. Atomic `.part` → rename on
  download. No models are bundled in the crate (preserves crates.io size
  budget).
- Classical pipeline remains fully available and is still the default when
  the ML opt-in env var is unset, so existing deployments are unaffected.

#### OCR: neighbor-density region filter
- `layout::filter_by_neighbor_density` rejects isolated regions that lack
  enough co-linear similarly-sized neighbors (real text always appears in
  dense clusters). Wired via `OcrConfig::neighbor_density_min` /
  `OMNIPARSE_OCR_NEIGHBOR_MIN=<int>`.

#### OCR: scale normalization, advanced preprocess, script detection, caching
- **Per-line scale normalization** on the recognizer
  (`FeatureRecognizer::with_normalize_height` / `OMNIPARSE_OCR_NORMALIZE_HEIGHT=<px>`).
  Each region crop is resized to a canonical height (aspect-preserving)
  before feature extraction, compensating for the feature vector's residual
  scale sensitivity.
- **Bilateral filter** (`preprocess::bilateral_filter`,
  `OMNIPARSE_OCR_BILATERAL=<radius>`) — edge-preserving denoise. Keeps
  strokes sharp while smoothing flat noise.
- **Unsharp mask** (`preprocess::unsharp_mask`, `OMNIPARSE_OCR_UNSHARP=<amount>`)
  — amplifies edge contrast before binarization.
- **Script detection** (`omniparse::ocr::script`) — classifies characters
  into Latin / Cyrillic / Greek / Arabic / Hebrew / Han / Hiragana /
  Katakana / Hangul / Devanagari / Digit / Other. `OcrOutput::detected_script`
  reports the dominant script of the recognized text (excluding digits and
  punctuation).
- **Parallel per-region recognition** behind a new `ocr-parallel` Cargo
  feature (implies `ocr` + `parallel`). Regions get dispatched through
  rayon instead of a serial loop.
- **Result cache** (`omniparse::ocr::cache::OcrCache`, shared instance via
  `OMNIPARSE_OCR_CACHE_SIZE=<int>`, disabled via `OMNIPARSE_OCR_CACHE=0`).
  LRU cache keyed on a FNV-style 32-byte digest of the input bytes. Repeat
  calls with identical bytes reuse the prior result.

#### OCR: engine-integrated post-processing + prototype dedup
- **Bigram re-ranking wired into the engine** — `OcrConfig::bigram_rerank`
  flag + `OMNIPARSE_OCR_BIGRAM=1` env. When on, per-line character candidates
  run through `BigramRanker` before final text assembly.
- **Word-level beam search** (`postprocess::beam_search_line`) — Viterbi-like
  beam over per-glyph top-k candidates scored by recognition confidence +
  bigram log-prob + dictionary bonus. Produces the jointly best dictionary-
  aligned string across the line. Wired via `OcrConfig::beam_search` /
  `OMNIPARSE_OCR_BEAM=1`, `OMNIPARSE_OCR_BEAM_WIDTH=<int>`.
- **Stroke-width constancy filter** now applied automatically after layout
  analysis when `OcrConfig::stroke_width_cv_max` or `OMNIPARSE_OCR_SW_CV_MAX`
  is set. Drops layout regions whose ink has inconsistent stroke widths
  (photographic edges).
- **Text-line heuristic filter** (`layout::filter_text_lines`,
  `OcrConfig::text_line_filter` / `OMNIPARSE_OCR_LINE_FILTER=1`) — groups
  regions into lines and rejects lines with high glyph-height variance or
  abnormal inter-glyph spacing.
- **Prototype deduplication** (`prototypes::dedupe_prototypes`) via k-medoids
  per label. Shrinks multi-font × multi-scale prototype sets without
  materially hurting accuracy.

#### OCR: feature vector expansion + language-model re-ranking
- **FEATURE_COUNT bumped to 55** (breaking — retrain JSON prototypes). New
  features:
  - 7 Hu moment invariants (log-magnitude, scale/rotation/translation
    invariant)
  - 12 HOG-lite cells (3 horizontal zones × 4 gradient-orientation bins,
    L1-normalized per zone)
  - 4 skeleton topology values (endpoint ratio, junction ratio, skeleton
    length / area, longest horizontal skeleton run / width) via Zhang-Suen
    thinning
- `FeatureVec` type switched from `[f32; N]` to `Vec<f32>` so serde can
  derive Serialize/Deserialize for the larger vector. Length invariant
  enforced by every constructor.
- `FeatureRecognizer::distance_scale` default raised from 0.5 to 1.0 to
  compensate for the expanded dimensionality.
- **Per-glyph top-k alternatives** now surfaced on
  `RecognizedLine::alternatives` (dedupe by label, sort by distance). Used
  downstream by language-model re-ranking.
- **Character bigram re-ranker** (`omniparse::ocr::bigram::BigramRanker`).
  Bundled English bigram log-prob table, Viterbi over per-glyph candidate
  lists, combined score `α·recognition + β·bigram`. No external data
  files, no ML.
- **Stroke-width constancy filter**
  (`layout::filter_by_stroke_width_constancy`) — L1 distance transform over
  a binarized glyph image, rejects regions whose per-pixel stroke-width
  coefficient-of-variation exceeds a threshold. Useful post-CCA cleanup for
  photographic inputs.

#### OCR: training + validation tooling
- **Multi-font training** (`train::train_multifont_multiscale` +
  `train_multifont_multiscale_from_paths`). `train_prototypes` example now
  accepts a colon-separated font path list; the concatenated prototype set
  covers several typefaces in one JSON file.
- **Pipeline validator** (`examples/ocr_validate.rs`) — renders a known
  string with the supplied font at a user-chosen size, trains prototypes
  from the same font, runs the full pipeline, and reports per-character
  accuracy against the target. Use it to distinguish "pipeline bug" from
  "font/input mismatch" when real-world images produce garbage.

#### OCR: env wiring for every preprocess/recognizer knob
- `OMNIPARSE_OCR_BINARIZE=otsu|sauvola|adaptive_mean|disabled`
- `OMNIPARSE_OCR_BIN_WINDOW=<int>` (Sauvola/AdaptiveMean window)
- `OMNIPARSE_OCR_CLAHE=1`
- `OMNIPARSE_OCR_TOPHAT=<radius>`
- `OMNIPARSE_OCR_DESPECKLE=<radius>`
- `OMNIPARSE_OCR_K=<int>` (k-NN vote)
- `OMNIPARSE_OCR_POLARITY=1` (try both polarities per region)
- `OMNIPARSE_OCR_KDTREE=1` (build kd-tree over prototypes)
- `OMNIPARSE_OCR_LAYOUT` now accepts `mser` in addition to `cca` / `swt`.
- `OMNIPARSE_OCR_PROTOTYPES` now terminates the process with a descriptive
  error when the file is missing or fails to deserialize (previously fell
  back silently to bundled bitmaps, producing garbage-indistinguishable
  output).

#### OCR: new analyzers and algorithms
- **MSER** (`omniparse::ocr::mser::MserLayoutAnalyzer`) — Maximally Stable
  Extremal Regions. Two polarity passes by default. Typical alternative to
  SWT on photographic inputs.
- **Per-region polarity detection** — `FeatureRecognizer::with_both_polarities`
  extracts features from both the crop and its inversion, keeps the label
  with the smaller nearest-neighbor distance.
- **Layout NMS and filters** — `layout::iou`, `layout::nms_regions`,
  `layout::filter_text_regions`, `layout::merge_regions` for combining
  candidates across CCA / SWT / MSER into a single deduplicated set.
- **k-d tree nearest-neighbor** (`omniparse::ocr::kdtree::KdTree`).
  `FeatureRecognizer::build_kdtree` swaps the linear scan for a kd-tree —
  recommended when prototype counts exceed a few hundred.

#### OCR diagnostics and PDF hookup
- New `ocr::OcrAttempt` enum (`Disabled` / `NoTextFound` / `Error` /
  `Recognized`) surfaced via image parsers as structured metadata:
  `ocr_status`, `ocr_applied`, `ocr_confidence`, `ocr_regions`, `ocr_error`.
  Callers can now distinguish "OCR didn't run" from "OCR ran and found
  nothing" from "OCR errored out" — previously every non-success path
  silently produced `Content::None` with no trace.
- `ocr::run_ocr(bytes)` exposed as the structured-outcome API;
  `ocr::maybe_ocr` kept as a back-compat shim.
- PDF OCR Tier 1: when the PDF text layer is empty, the parser now walks
  every page's `/XObject` dictionary, extracts `Subtype = Image` streams
  whose filter is `DCTDecode` (JPEG), and OCRs each embedded image.
  Recognized text is concatenated with `[image N of M]` headers; diagnostic
  metadata includes `ocr_images_total` and `ocr_images_recognized`. Gated
  on the same `OMNIPARSE_OCR=1` runtime switch as image OCR. Unsupported
  filters (`JPXDecode`, `FlateDecode`, `CCITTFaxDecode`) are silently
  skipped for now.

#### Tests
- New `tests/enhancements_test.rs` and `tests/ocr_test.rs` covering the new
  behavior end-to-end.

### Changed
- **MSRV bumped to 1.88** — the internal codebase now uses stabilized
  `if let … && let` chains.
- `tests/parser_registration_test.rs` rewritten to check a required
  MIME-type list rather than a stale fixed count.
- `tests/cli_test.rs::test_cli_help_flag` assertion aligned with the clap
  `long_about` string.

### Breaking changes
- **TIFF parser**: the `tiff_ImageWidth`, `tiff_Make`, `tiff_DateTime`, … keys
  that previously emitted literal `"tag_{id}"` placeholder values have been
  removed. Real values are now available under the shared EXIF keys listed
  under "Added". Consumers reading the old keys will see `None`.
- **PNG parser**: `ztext_*` and compressed `itext_*` values changed from the
  literal string `"[compressed text]"` to the decompressed text. Consumers
  pattern-matching the placeholder must update.
- **JPEG parser**: `exif_present` is now only set when EXIF actually parses;
  malformed APP1 payloads no longer yield `exif_present = true`.

### Dependencies
- Added `kamadak-exif` 0.5 (direct).
- Added `flate2` 1.0 (direct; previously reached only transitively via `zip`).
- Added optional `imageproc` 0.23, `symspell` 0.5, `log` 0.4 (gated by the
  new `ocr` feature).

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

[0.3.0]: https://github.com/omniparse/omniparse/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/omniparse/omniparse/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/omniparse/omniparse/releases/tag/v0.1.0
