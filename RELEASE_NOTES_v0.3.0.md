# Omniparse v0.3.0 Release Notes

*Release date: 2026-04-22*

A major release focused on three themes:

1. **OCR** — a complete optical character recognition subsystem with
   classical and ML backends, per-stage traits, and extensive tuning knobs.
2. **New format parsers** — Markdown, SVG, WebP, EPUB, MP3.
3. **Metadata + hardening** — real EXIF extraction, PNG zTXt decompression,
   HTML OpenGraph/Twitter/canonical metadata, PDF encryption + form + annotation
   counts, archive path-traversal detection, `ParserRegistry` caching.

If you're upgrading from v0.2.x, read
[`MIGRATION_v0.3.0.md`](MIGRATION_v0.3.0.md) — a handful of metadata keys
changed and the feature-flag layout expanded.

## Table of contents

1. [Highlights](#highlights)
2. [New format parsers](#new-format-parsers)
3. [OCR subsystem](#ocr-subsystem)
4. [Parser metadata additions](#parser-metadata-additions)
5. [Security, hardening, performance](#security-hardening-performance)
6. [Breaking changes](#breaking-changes)
7. [Cargo feature matrix](#cargo-feature-matrix)
8. [Full environment-variable reference](#full-environment-variable-reference)
9. [Full tooling reference](#full-tooling-reference)

---

## Highlights

- **ML OCR that actually works** — opt-in `ocr-ml` Cargo feature pulls
  pure-Rust `ocrs` + `rten`, downloads pre-trained models on first use,
  recognizes photographic text overlays at ~90% confidence.
- **Classical OCR pipeline** — pure algorithmic, no ML, no downloads. Runs
  end-to-end for clean scans and when model downloads aren't acceptable.
- **Five new format parsers** — Markdown, SVG, WebP, EPUB, MP3 — each
  feature-flagged (all on by default).
- **Real EXIF** for JPEG and TIFF via `kamadak-exif`, replacing the v0.2
  placeholder stubs.
- **Registry caching** — `supported_mime_types()` / `is_mime_supported()`
  are now O(1) instead of rebuilding the full parser registry per call.

---

## New format parsers

### Markdown (`markdown` feature, default on)

`pulldown-cmark`-driven parser. Strips Markdown syntax into plain text and
emits structural metadata.

```rust
let result = omniparse::extract_from_path("post.md")?;
if let omniparse::Content::Text(body) = &result.content {
    println!("{body}");
}
assert_eq!(result.metadata.title(), Some("My Post")); // from first H1
```

Metadata keys: `title`, `heading_h1_count` … `heading_h6_count`,
`link_count`, `image_count`, `code_block_count`.

### SVG (`svg` feature, default on)

Treats SVG as structured XML via `quick-xml`. Pulls `<title>`, `<desc>`,
root attributes, text nodes, and element counts.

Metadata keys: `title`, `description`, `viewbox`, `width`, `height`,
`xmlns`, `element_path_count`, `element_rect_count`,
`element_circle_count`, `element_ellipse_count`, `element_line_count`,
`element_polyline_count`, `element_polygon_count`, `element_g_count`,
`element_use_count`, `element_image_count`.

### WebP (`webp` feature, default on)

Uses the existing `image` crate for decode plus the shared EXIF helper.
Integrates with the OCR hookup path when the `ocr` feature is also on.

### EPUB (`epub` feature, default on)

OPF metadata extraction + spine walk + chapter body text via `epub` 2.1
crate and `scraper` for XHTML stripping.

Metadata keys: `title`, `author`, `publisher`, `language`, `description`,
`rights`, `publication_date`, `keywords`, `identifier`, `spine_count`,
`resource_count`.

### MP3 (`mp3` feature, default on)

ID3v1 / ID3v2 tags via the `id3` crate. New `parsers::audio` submodule —
future audio formats plug in here.

Metadata keys: `title`, `artist`, `album`, `album_artist`, `genre`,
`year`, `track`, `total_tracks`, `disc`, `duration_ms`. Comment frames
appear as the extracted text content.

---

## OCR subsystem

Activated via the `ocr` or `ocr-ml` Cargo feature plus the runtime gate
`OMNIPARSE_OCR=1`. Two backends:

### Classical backend (`ocr` feature)

Pure-algorithm pipeline. No ML runtime, no downloaded models. Stages:

1. **Preprocess** (`omniparse::ocr::preprocess`)
   - Otsu / Sauvola / AdaptiveMean binarization
   - Median despeckle, bilateral denoise, unsharp mask
   - CLAHE contrast enhancement
   - Morphological top-hat background subtraction
   - Hough-based deskew (small angles)
   - Auto-rotation 90° / 180° / 270°
2. **Layout** (`omniparse::ocr::layout`, `swt`, `mser`)
   - Connected-component analyzer
   - Stroke-Width Transform (Epshtein 2010)
   - Maximally Stable Extremal Regions (Matas 2002)
   - Non-maximum suppression across analyzers
   - Stroke-width constancy filter
   - Neighbor-density cluster filter
   - Text-line heuristic filter
3. **Recognize** (`omniparse::ocr::recognize`, `features`, `kdtree`,
   `prototypes`)
   - 55-dim feature vector per region (zones, projections, symmetry,
     crossings, Euler number, Hu moments, HOG-lite, skeleton topology)
   - 1-NN / k-NN with inverse-distance voting
   - Optional k-d tree for NN over large prototype sets
   - Per-region polarity detection
   - Per-line scale normalization
4. **Postprocess** (`omniparse::ocr::postprocess`, `bigram`)
   - SymSpell dictionary correction
   - Character-bigram language-model re-ranking
   - Word-level beam search with dictionary bonus

### ML backend (`ocr-ml` feature)

`ocrs` 0.12 + `rten` 0.24. Pure Rust. Pre-trained detection + recognition
models download once on first use to
`dirs::cache_dir()/omniparse/ocrs-models/` (override via
`OMNIPARSE_OCR_MODELS=<path>`). Atomic download-then-rename so partial
transfers don't poison the cache.

Runtime dispatch: set `OMNIPARSE_OCR_ML=1` alongside `OMNIPARSE_OCR=1`.

### Tooling

- **`examples/ocr_basic.rs`** — smoke test against the bundled bitmap
  prototypes or a user-supplied image.
- **`examples/ocr_validate.rs`** — render a known string with a user-
  supplied font, run the full classical pipeline, report per-character
  accuracy. Use this to distinguish pipeline bugs from font/input mismatch.
- **`examples/train_prototypes.rs`** — generate JSON prototypes from one
  or more TTF fonts at one or more pixel sizes. Format:
  `<font1.ttf[:font2.ttf:…]> <out.json> <px1,px2,px3> [chars]`.

### Diagnostic image dumps

Set `OMNIPARSE_OCR_DEBUG_DIR=<path>` and every `OcrEngine::recognize` call
writes `01_input.png`, `02_preprocessed.png`, `03_layout.png`
(with red region bounding boxes) to that directory. Essential for tuning.

### PDF OCR (Tier 1)

When a PDF's text layer is empty and OCR is enabled, the PDF parser walks
every page's `/XObject` dictionary, extracts `Subtype = Image` streams
with `DCTDecode` (JPEG) filter, and OCRs each. Concatenates recognized
text with `[image N of M]` headers. Metadata includes `ocr_images_total`
and `ocr_images_recognized`.

### OCR result caching

Repeated calls with the same bytes reuse the prior result via an in-
memory LRU cache keyed on a 32-byte digest. Default capacity 64 entries.
Disable with `OMNIPARSE_OCR_CACHE=0`; size via `OMNIPARSE_OCR_CACHE_SIZE`.

### OCR output diagnostics

Image parsers populate a structured `ocr_status` metadata field so
callers can distinguish the three non-success paths:

| `ocr_status`      | Meaning                                                |
| ----------------- | ------------------------------------------------------ |
| absent            | `ocr` feature off OR `OMNIPARSE_OCR` runtime gate off  |
| `no_text_found`   | Pipeline ran, nothing passed confidence filter         |
| `error`           | Engine error (see `ocr_error` metadata)                |
| `recognized`      | Text extracted; also see `ocr_confidence`              |

---

## Parser metadata additions

### PDF

- `pdf_version`, `encrypted`, `form_fields_count`, `annotations_count`,
  `attachments_count`, `page_layout`, `page_mode`, `modification_date`,
  `keywords`.

### JPEG + TIFF

Real EXIF via `kamadak-exif` (v0.2 emitted placeholders or just the
`exif_present` flag). New keys include `camera_make`, `camera_model`,
`software`, `artist`, `copyright`, `image_description`, `lens_model`,
`orientation`, `iso`, `focal_length_mm`, `f_number`,
`exposure_time_sec`, `exposure_bias`, `focal_length_35mm`, `datetime`,
`datetime_original`, `datetime_digitized`, `gps_latitude`,
`gps_longitude`.

### PNG

zTXt and compressed iTXt payloads now properly zlib-inflated via
`flate2`; previously returned the literal placeholder string
`"[compressed text]"`.

### HTML

OpenGraph (`og_*`), Twitter Card (`twitter_*`), canonical URL
(`canonical_url`), viewport, robots, http-equiv headers
(`http_equiv_*`), and heading-tag counts `heading_h1_count` through
`heading_h6_count`.

### ZIP + TAR

New boolean `contains_unsafe_paths` metadata flag derived from scanning
entries for `..`, absolute paths, and Windows drive prefixes. ZIP parser
also refactored from three passes to one.

### Images (with `ocr` feature)

`ocr_applied: bool`, `ocr_confidence: f64`, `ocr_status: Text`, plus
`ocr_regions` / `ocr_error` for non-success cases.

---

## Security, hardening, performance

- `utils::security::is_safe_archive_path` — detects `..`, absolute, and
  Windows-drive-prefixed entry paths. Applied automatically by ZIP and
  TAR parsers.
- `utils::security::MAX_ARCHIVE_DEPTH` constant (default 8) anchors a
  future recursive-extraction cap.
- `parsers::default_registry()` — `OnceLock`-cached shared registry.
- HTML parser no longer panics on malformed selectors; `.unwrap()` calls
  on `Selector::parse` replaced with graceful fallbacks.
- PNG `zTXt` / compressed `iTXt` no longer silently stored as a
  placeholder.
- TIFF: removed stale `tiff_*` keys that emitted literal `"tag_{id}"`
  strings — real tag values now come from the shared EXIF helper.

---

## Breaking changes

See [`MIGRATION_v0.3.0.md`](MIGRATION_v0.3.0.md) for details and migration
steps. Short list:

1. **TIFF** — `tiff_ImageWidth`, `tiff_Make`, `tiff_DateTime`, etc. keys
   removed. Real EXIF keys added in their place.
2. **PNG** — `ztext_*` values changed from `"[compressed text]"` to the
   decompressed text.
3. **JPEG** — `exif_present` now only true when EXIF actually parses.
4. **MSRV** — minimum Rust version is now 1.88 (stabilized
   `if let … && let` chains used internally).

---

## Cargo feature matrix

| Feature        | Default | Description                                    |
| -------------- | ------- | ---------------------------------------------- |
| `async`        | off     | Tokio-based `extract_from_path_async`          |
| `parallel`     | off     | Rayon-based `process_files_parallel`           |
| `markdown`     | **on**  | Markdown parser                                |
| `svg`          | **on**  | SVG parser                                     |
| `webp`         | **on**  | WebP parser                                    |
| `epub`         | **on**  | EPUB parser                                    |
| `mp3`          | **on**  | MP3 parser                                     |
| `ocr`          | off     | Classical OCR pipeline                         |
| `ocr-train`    | off     | TTF → prototype trainer (implies `ocr`)        |
| `ocr-parallel` | off     | Parallel per-region recognition (implies `ocr` + `parallel`) |
| `ocr-ml`       | off     | ML OCR via `ocrs` + `rten` (implies `ocr`)     |

Disable all default formats:
```toml
omniparse = { version = "0.3", default-features = false }
```

---

## Full environment-variable reference

### Runtime gates

| Variable                          | Purpose                              |
| --------------------------------- | ------------------------------------ |
| `OMNIPARSE_OCR=1`                 | Master OCR on/off gate               |
| `OMNIPARSE_OCR_ML=1`              | Use ML backend (requires `ocr-ml`)   |
| `OMNIPARSE_OCR_CACHE=0`           | Disable result cache                 |
| `OMNIPARSE_OCR_CACHE_SIZE=<int>`  | Result-cache LRU capacity (default 64) |

### Classical pipeline — preprocess

| Variable                             | Default     | Description                                 |
| ------------------------------------ | ----------- | ------------------------------------------- |
| `OMNIPARSE_OCR_BINARIZE`             | `otsu`      | `otsu` / `sauvola` / `adaptive_mean` / `disabled` |
| `OMNIPARSE_OCR_BIN_WINDOW=<int>`     | 25          | Sauvola / AdaptiveMean window               |
| `OMNIPARSE_OCR_CLAHE=1`              | off         | Contrast-limited histogram equalization     |
| `OMNIPARSE_OCR_TOPHAT=<radius>`      | 0           | Morphological top-hat background subtract   |
| `OMNIPARSE_OCR_BILATERAL=<radius>`   | 0           | Bilateral filter (edge-preserving denoise)  |
| `OMNIPARSE_OCR_UNSHARP=<amount>`     | 0.0         | Unsharp mask strength                       |
| `OMNIPARSE_OCR_DESPECKLE=<radius>`   | 1           | Median filter radius                        |
| `OMNIPARSE_OCR_AUTO_ROTATE=1`        | off         | Try 4 orientations, keep highest score      |

### Classical pipeline — layout

| Variable                             | Default | Description                                    |
| ------------------------------------ | ------- | ---------------------------------------------- |
| `OMNIPARSE_OCR_LAYOUT`               | `cca`   | `cca` / `swt` / `mser`                         |
| `OMNIPARSE_OCR_SW_CV_MAX=<float>`    | unset   | Stroke-width CV threshold                      |
| `OMNIPARSE_OCR_LINE_FILTER=1`        | off     | Reject text lines with high height variance    |
| `OMNIPARSE_OCR_NEIGHBOR_MIN=<int>`   | unset   | Minimum co-linear similar-sized neighbors      |

### Classical pipeline — recognize

| Variable                                   | Default | Description                        |
| ------------------------------------------ | ------- | ---------------------------------- |
| `OMNIPARSE_OCR_PROTOTYPES=<json-path>`     | unset   | JSON prototype file (strict load)  |
| `OMNIPARSE_OCR_K=<int>`                    | 1       | k-NN vote count                    |
| `OMNIPARSE_OCR_POLARITY=1`                 | off     | Try both ink polarities            |
| `OMNIPARSE_OCR_KDTREE=1`                   | off     | k-d tree for NN                    |
| `OMNIPARSE_OCR_NORMALIZE_HEIGHT=<px>`      | unset   | Resize each region to canonical height |
| `OMNIPARSE_OCR_MIN_CONFIDENCE=<0..=1>`     | 0.15    | Drop lines below this confidence   |

### Classical pipeline — postprocess

| Variable                             | Default | Description                               |
| ------------------------------------ | ------- | ----------------------------------------- |
| `OMNIPARSE_OCR_BIGRAM=1`             | off     | Character-bigram re-ranking               |
| `OMNIPARSE_OCR_BEAM=1`               | off     | Word-level beam search (dictionary-aware) |
| `OMNIPARSE_OCR_BEAM_WIDTH=<int>`     | 8       | Beam width                                |

### ML backend

| Variable                           | Default | Description                    |
| ---------------------------------- | ------- | ------------------------------ |
| `OMNIPARSE_OCR_MODELS=<path>`      | cache   | Override model cache directory |

### Debugging

| Variable                           | Description                                    |
| ---------------------------------- | ---------------------------------------------- |
| `OMNIPARSE_OCR_DEBUG_DIR=<path>`   | Dump input/preprocessed/layout PNGs per call   |

---

## Full tooling reference

### Command-line

```sh
# Install
cargo install omniparse --features "ocr-ml ocr-train ocr-parallel"

# Basic extraction
omniparse document.pdf

# With OCR on a photo
OMNIPARSE_OCR=1 OMNIPARSE_OCR_ML=1 omniparse photo.jpg

# Parallel batch
omniparse --parallel *.pdf *.docx
```

### Library — classical OCR

```rust
use omniparse::ocr::{OcrEngine, OcrEngineBuilder, OcrConfig};
use omniparse::ocr::preprocess::{ImageprocPreprocessor, PreprocessConfig, BinarizeMode};
use omniparse::ocr::layout::ConnectedComponentAnalyzer;
use omniparse::ocr::recognize::FeatureRecognizer;
use omniparse::ocr::prototypes::load_prototypes_json;

let prototypes = load_prototypes_json("/tmp/arial.json")?;
let engine: OcrEngine = OcrEngineBuilder::default()
    .preprocessor(ImageprocPreprocessor::with_config(PreprocessConfig {
        binarize: BinarizeMode::Sauvola { window: 25, k: 0.2, r: 128.0 },
        clahe: true,
        ..Default::default()
    }))
    .layout(ConnectedComponentAnalyzer::default())
    .recognizer(
        FeatureRecognizer::new(prototypes)
            .with_k(5)
            .with_both_polarities(true)
            .build_kdtree(),
    )
    .config(OcrConfig {
        min_confidence: 0.2,
        bigram_rerank: true,
        ..Default::default()
    })
    .build();

let image = image::open("page.png")?;
let out = engine.recognize(image)?;
println!("{}", out.text);
println!("confidence: {:.2}", out.mean_confidence);
println!("script: {:?}", out.detected_script);
```

### Library — ML OCR

```rust
#[cfg(feature = "ocr-ml")]
{
    use omniparse::ocr::ml::MlOcrEngine;
    let engine = MlOcrEngine::new()?;
    let out = engine.recognize(image::open("photo.jpg")?)?;
    println!("{}", out.text);
}
```

### Library — via image parsers (automatic)

```rust
// OMNIPARSE_OCR=1 in the environment activates OCR for image parsers.
let result = omniparse::extract_from_path("photo.jpg")?;
match &result.content {
    omniparse::Content::Text(text) => println!("{text}"),
    _ => {}
}
// Diagnostics for any OCR attempt:
if let Some(status) = result.metadata.get("ocr_status") {
    println!("ocr_status = {status:?}");
}
```

### Training custom prototypes

```sh
# Single font, multiple sizes
cargo run --features ocr-train --example train_prototypes -- \
    /System/Library/Fonts/Supplemental/Arial.ttf \
    ./arial.json \
    24,48,96,128

# Multiple fonts, multiple sizes
cargo run --features ocr-train --example train_prototypes -- \
    Arial.ttf:Helvetica.ttf:Verdana.ttf \
    ./multifont.json \
    24,48,96

# Validation: renders known text, runs pipeline, reports accuracy
cargo run --features ocr-train --example ocr_validate -- \
    Arial.ttf "HELLO WORLD" 48
```

---

## Dependency additions

| Crate            | Version | Optional | Feature     |
| ---------------- | ------- | -------- | ----------- |
| `kamadak-exif`   | 0.5     | no       | —           |
| `flate2`         | 1.0     | no       | —           |
| `pulldown-cmark` | 0.10    | yes      | `markdown`  |
| `epub`           | 2.1     | yes      | `epub`      |
| `id3`            | 1       | yes      | `mp3`       |
| `imageproc`      | 0.23    | yes      | `ocr`       |
| `symspell`       | 0.5     | yes      | `ocr`       |
| `log`            | 0.4     | yes      | `ocr`       |
| `ab_glyph`       | 0.2     | yes      | `ocr-train` |
| `ocrs`           | 0.12    | yes      | `ocr-ml`    |
| `rten`           | 0.24    | yes      | `ocr-ml`    |
| `rten-imageproc` | 0.24    | yes      | `ocr-ml`    |
| `dirs`           | 5       | yes      | `ocr-ml`    |
| `ureq`           | 2       | yes      | `ocr-ml`    |
| `sha2`           | 0.10    | yes      | `ocr-ml`    |
