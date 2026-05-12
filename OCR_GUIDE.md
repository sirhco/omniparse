# Omniparse OCR Guide

Single canonical reference for the OCR subsystem. The README points here.

## 30-second start

ML backend (recommended for photos, screenshots, unknown typography):

```sh
cargo install omniparse --features ocr-ml
omniparse models download                 # one-time, ~12 MB to cache
OMNIPARSE_OCR=ml omniparse photo.jpg      # text appears in stdout
```

Classical backend (pure-Rust, no downloads, fast — but only good on clean
printed scans with a matched font):

```sh
cargo install omniparse --features ocr
OMNIPARSE_OCR=classical omniparse scan.png
```

That's it. Skip to [PDF OCR](#pdf-ocr) for scanned-PDF specifics, or read on
for the backend-chooser table and tuning knobs.

## Contents

1. [The OMNIPARSE_OCR env var](#the-omniparse_ocr-env-var)
2. [Classical vs ML backend](#classical-vs-ml-backend)
3. [Managing the model cache](#managing-the-model-cache)
4. [Quickstart: ML backend](#quickstart-ml-backend)
5. [Quickstart: classical backend](#quickstart-classical-backend)
6. [Training custom prototypes](#training-custom-prototypes)
7. [Validating the pipeline](#validating-the-pipeline)
8. [Tuning the classical pipeline](#tuning-the-classical-pipeline)
9. [Debugging OCR output](#debugging-ocr-output)
10. [PDF OCR](#pdf-ocr)
11. [Library API](#library-api)
12. [FAQ](#faq)

## The OMNIPARSE_OCR env var

A single switch controls which backend runs:

| Value           | Effect                                                  |
| --------------- | ------------------------------------------------------- |
| unset / `off`   | OCR disabled (image parsers extract EXIF only)          |
| `classical`     | Pure-Rust classical pipeline (requires `ocr` feature)   |
| `ml`            | ocrs + rten ML backend (requires `ocr-ml` feature)      |

> The previous two-variable scheme (`OMNIPARSE_OCR=1` plus
> `OMNIPARSE_OCR_ML=1`) still works but prints a one-shot deprecation
> warning. It will be removed in 0.5.

## When to enable OCR

OCR is **off by default**. Enable it when:

- You're extracting from images that contain text but have no embedded
  text layer (photographs, screenshots, scanned PDFs).
- You're extracting from image/PDF inputs where text-layer extraction
  returns empty.

Skip it when:

- Your inputs are digital PDFs with intact text layers (`lopdf` already
  handles those — OCR adds no value and costs time).
- Your inputs are text-based formats (HTML, Markdown, Office documents).

## Classical vs ML backend

| Aspect               | Classical (`ocr`)               | ML (`ocr-ml`)                       |
| -------------------- | ------------------------------- | ----------------------------------- |
| Accuracy (clean scan)| Good                            | Very good                           |
| Accuracy (photo text)| Poor                            | Very good                           |
| Handwriting          | Useless                         | Limited                             |
| Non-Latin scripts    | Needs custom prototypes         | English model only out-of-box       |
| First-run cost       | Zero (algorithms only)          | ~30 MB model download               |
| Per-image cost       | Milliseconds                    | Seconds                             |
| Build size           | +imageproc / symspell           | +rten + rten-imageproc              |
| Offline-friendly     | Yes (pure algorithms)           | Only after first download           |
| Customization        | Extensive (train your own font) | Fixed models                        |

**Pick the classical backend when** you need full control, have matched-
font prototypes, are running on minimal infrastructure, or can't accept a
network download.

**Pick the ML backend when** your inputs are photographs, screenshots of
real applications, or anything with unknown typography — and you can accept
a one-time model download.

## Managing the model cache

The CLI ships dedicated subcommands for the ML model cache. They're useful
for pre-fetching in CI, baking models into a container image, debugging
"why isn't OCR working", and air-gapped installs.

```sh
omniparse models path        # print cache directory
omniparse models download    # fetch any missing models (~12 MB)
omniparse models list        # show name / size / sha256 / status per model
omniparse models verify      # re-hash cached files; exit 0 only if all match
```

All four commands require the `ocr-ml` feature; without it they exit with
a clear error message.

Override the cache directory with the `OMNIPARSE_OCR_MODELS` env var:

```sh
OMNIPARSE_OCR_MODELS=/opt/omniparse/models omniparse models download
```

Useful for shared installs or air-gapped environments where you want to
pre-populate the cache manually, or for Docker images that bake models
into a known path (see the project `Dockerfile`).

Programmatic equivalents live on [`omniparse::ocr::ml`]:
[`prefetch_all`][prefetch_all], [`verify_all`][verify_all],
[`list_models`][list_models], [`model_dir`][model_dir].

[prefetch_all]: https://docs.rs/omniparse/latest/omniparse/ocr/ml/fn.prefetch_all.html
[verify_all]: https://docs.rs/omniparse/latest/omniparse/ocr/ml/fn.verify_all.html
[list_models]: https://docs.rs/omniparse/latest/omniparse/ocr/ml/fn.list_models.html
[model_dir]: https://docs.rs/omniparse/latest/omniparse/ocr/ml/fn.model_dir.html

## Quickstart: ML backend

### Install

```toml
[dependencies]
omniparse = { version = "0.4", features = ["ocr-ml"] }
```

### Command line

```sh
# One-time setup
omniparse models download

# Every recognition run
OMNIPARSE_OCR=ml omniparse photo.jpg
```

Models live at `~/Library/Caches/omniparse/ocrs-models/` (macOS) or the
platform-appropriate cache dir. Override with
`OMNIPARSE_OCR_MODELS=<path>`.

### Library

```rust
use omniparse::ocr::ml::MlOcrEngine;
let engine = MlOcrEngine::new()?;          // downloads + sha256-verifies on first call
let image = image::open("photo.jpg")?;
let output = engine.recognize(image)?;
println!("{}", output.text);
```

## Quickstart: classical backend

### Install

```toml
[dependencies]
omniparse = { version = "0.4", features = ["ocr", "ocr-train"] }
```

(`ocr-train` is only needed for prototype generation; drop it for runtime-
only deployments that ship their own prototype JSON.)

### Command line with bundled prototypes

```sh
OMNIPARSE_OCR=classical omniparse image.png
```

The bundled prototypes are hand-authored 7×9 bitmap glyphs for uppercase
A–Z and digits 0–9. They're a smoke-test set — good enough to verify
the pipeline runs, **not good enough for real-world OCR**. Train custom
prototypes before production use.

### Command line with trained prototypes

```sh
# One-time training
cargo run --features ocr-train --example train_prototypes -- \
    /System/Library/Fonts/Supplemental/Arial.ttf ./arial.json 24,48,96

# Every recognition run
OMNIPARSE_OCR=classical OMNIPARSE_OCR_PROTOTYPES=./arial.json \
    omniparse image.png
```

## Training custom prototypes

### Single font, multiple sizes

```sh
cargo run --features ocr-train --example train_prototypes -- \
    Arial.ttf out.json 24,48,96,128
```

Produces one prototype per character × per pixel size. Include sizes that
match the heights of glyphs in your real images. Too-small or too-large
sizes are harmless but waste prototype entries.

### Multiple fonts

```sh
cargo run --features ocr-train --example train_prototypes -- \
    Arial.ttf:ArialBold.ttf:Helvetica.ttf:Verdana.ttf \
    out.json 24,48,96
```

Colon-separated font paths. Useful when the target input may contain any
of several similar typefaces.

### Custom character set

```sh
cargo run --features ocr-train --example train_prototypes -- \
    Arial.ttf out.json 48 "0123456789.,:"
```

Fourth argument is the literal string of characters to train. Defaults to
uppercase + lowercase + digits + common punctuation.

### Deduplicating a large prototype set

Multi-font × multi-scale sets can balloon into thousands of entries.
Reduce them via k-medoids per label:

```rust
use omniparse::ocr::prototypes::{load_prototypes_json, save_prototypes_json,
                                  dedupe_prototypes};
let protos = load_prototypes_json("multifont.json")?;
let reduced = dedupe_prototypes(protos, 4); // at most 4 per label
save_prototypes_json(&reduced, "multifont_small.json")?;
```

## Validating the pipeline

Before debugging a failing OCR run on a real image, confirm the pipeline
works end-to-end on a controlled input.

```sh
cargo run --features ocr-train --example ocr_validate -- \
    Arial.ttf "HELLO WORLD" 48
```

Expected output:

```
validator: font=Arial.ttf text="HELLO WORLD" px_size=48
trained 90 prototypes
recognized: "HELLO WORLD"
mean_confidence: 0.56
accuracy: 10/10 (100.0%)
```

100% accuracy confirms the pipeline is correct. Poor accuracy on real
images after this check is a font/input mismatch, not a code bug.

## Tuning the classical pipeline

Every stage has sensible defaults; override via env vars (CLI-friendly)
or the `OcrConfig` struct + `OcrEngineBuilder` (library-friendly).

### When the image has non-uniform lighting

```sh
export OMNIPARSE_OCR_BINARIZE=sauvola
export OMNIPARSE_OCR_CLAHE=1
```

Sauvola thresholds each pixel against its local neighborhood. CLAHE
normalizes contrast in overlapping tiles before binarization.

### When the image is a photograph with text overlay

```sh
export OMNIPARSE_OCR_LAYOUT=mser
export OMNIPARSE_OCR_SW_CV_MAX=0.5
export OMNIPARSE_OCR_NEIGHBOR_MIN=2
export OMNIPARSE_OCR_LINE_FILTER=1
```

MSER detects blob-like structures better than connected components on
photographic input. Stroke-width constancy rejects photo edges. Neighbor
density rejects isolated components. Line filter rejects clusters with
mismatched heights.

### When the image might be rotated

```sh
export OMNIPARSE_OCR_AUTO_ROTATE=1
```

Runs the pipeline four times (original + 90° + 180° + 270°) and keeps
the orientation with the highest `text_length × mean_confidence` score.
4× runtime cost.

### When the recognizer's top label is sometimes wrong

```sh
export OMNIPARSE_OCR_K=5
export OMNIPARSE_OCR_BIGRAM=1
```

k-NN voting uses the 5 nearest prototypes with inverse-distance weights.
Bigram re-ranking picks the character per position that maximizes English
character-bigram probability given the preceding character.

### When you want dictionary-constrained output

```sh
export OMNIPARSE_OCR_BEAM=1
export OMNIPARSE_OCR_BEAM_WIDTH=12
```

Word-level beam search. Picks the line-global string that jointly
maximizes recognition confidence + bigram fluency + dictionary membership.
Trade-off: will force-fit unknown words to the closest dictionary entry.

### When inputs are mixed-scale

```sh
export OMNIPARSE_OCR_NORMALIZE_HEIGHT=32
```

Resizes each detected region to 32px tall (aspect-preserving) before
feature extraction. Compensates for residual scale sensitivity in the
55-dim feature vector.

### When you have thousands of prototypes

```sh
export OMNIPARSE_OCR_KDTREE=1
```

Swaps the linear NN scan for a k-d tree. Recommended above ~500
prototypes.

### When input polarity is unknown

```sh
export OMNIPARSE_OCR_POLARITY=1
```

Extracts features from both the crop and its inverse; keeps whichever
produced the smaller nearest-neighbor distance.

## Debugging OCR output

### The three diagnostic fields

Every image parser output (with OCR on) includes:

```json
{
  "ocr_applied": true,
  "ocr_status": "recognized",
  "ocr_confidence": 0.82
}
```

Possible `ocr_status` values:

- `recognized` — text extracted. Content field is populated.
- `no_text_found` — pipeline ran, nothing passed the confidence filter.
  Also see `ocr_regions` (how many candidates the layout stage found).
- `error` — engine error. See `ocr_error` metadata.
- (field absent) — OCR didn't run. Check `OMNIPARSE_OCR=classical` (or
  `=ml`) and that the `ocr` or `ocr-ml` feature is compiled in.

### Visual debugging

```sh
export OMNIPARSE_OCR_DEBUG_DIR=/tmp/omniparse_debug
OMNIPARSE_OCR=classical omniparse image.jpg

open /tmp/omniparse_debug/01_input.png         # original (grayscale)
open /tmp/omniparse_debug/02_preprocessed.png  # after binarize + despeckle
open /tmp/omniparse_debug/03_layout.png        # red bboxes on detected regions
```

Interpretation:

- `02_preprocessed.png` shows muddy grey → binarization is failing.
  Switch to `sauvola` or enable `CLAHE`.
- `03_layout.png` shows red bboxes scattered across the photo background
  → layout analyzer is firing on image edges. Switch to `mser`, add
  `SW_CV_MAX` + `NEIGHBOR_MIN` filters.
- `03_layout.png` shows no bboxes at all → layout stage rejected every
  candidate. Relax filter thresholds or try a different layout analyzer.

## PDF OCR

When a PDF's text layer extraction returns empty, the PDF parser
automatically OCRs every embedded `DCTDecode` (JPEG) image. Requires the
same `OMNIPARSE_OCR=classical|ml` gate as image parsers. No extra
configuration.

```sh
OMNIPARSE_OCR=ml omniparse scanned.pdf
```

Output concatenates per-image recognized text with `[image N of M]`
headers. Metadata includes `ocr_images_total` and
`ocr_images_recognized`.

Limitations:

- `FlateDecode`, `JPXDecode`, `CCITTFaxDecode` image filters are not yet
  supported (require pixel reconstruction from the ColorSpace +
  BitsPerComponent dictionary). Affected images are silently skipped.
- No per-page rasterization of vector-only scanned PDFs. Pure-Rust PDF
  rasterization is not currently feasible.

## Library API

### OcrEngine — classical

```rust
use omniparse::ocr::{OcrEngine, OcrEngineBuilder, OcrConfig};
use omniparse::ocr::preprocess::{ImageprocPreprocessor, PreprocessConfig, BinarizeMode};
use omniparse::ocr::layout::ConnectedComponentAnalyzer;
use omniparse::ocr::recognize::FeatureRecognizer;
use omniparse::ocr::prototypes::load_prototypes_json;

let engine: OcrEngine = OcrEngineBuilder::default()
    .preprocessor(ImageprocPreprocessor::with_config(PreprocessConfig {
        binarize: BinarizeMode::Sauvola { window: 25, k: 0.2, r: 128.0 },
        clahe: true,
        bilateral_radius: 2,
        ..Default::default()
    }))
    .layout(ConnectedComponentAnalyzer::default())
    .recognizer(
        FeatureRecognizer::new(load_prototypes_json("/tmp/arial.json")?)
            .with_k(5)
            .with_both_polarities(true)
            .with_normalize_height(Some(32))
            .build_kdtree(),
    )
    .config(OcrConfig {
        min_confidence: 0.2,
        bigram_rerank: true,
        auto_rotate: true,
        stroke_width_cv_max: Some(0.5),
        neighbor_density_min: Some(2),
        ..Default::default()
    })
    .build();

let output = engine.recognize(image::open("page.png")?)?;
```

### MlOcrEngine — ML

```rust
#[cfg(feature = "ocr-ml")]
let engine = omniparse::ocr::ml::MlOcrEngine::new()?;
let output = engine.recognize(image::open("photo.jpg")?)?;
```

### Plug a custom stage

Every stage is a trait:

```rust
use omniparse::ocr::preprocess::Preprocessor;
use omniparse::ocr::error::OcrResult;
use image::{DynamicImage, GrayImage};

struct MyPreprocessor;
impl Preprocessor for MyPreprocessor {
    fn process(&self, img: DynamicImage) -> OcrResult<GrayImage> {
        Ok(img.into_luma8())
    }
}

let engine = OcrEngineBuilder::default()
    .preprocessor(MyPreprocessor)
    .build();
```

Same pattern for `LayoutAnalyzer`, `Recognizer`, `PostProcessor`.

### Standalone OCR helper

```rust
#[cfg(feature = "ocr")]
let text = omniparse::ocr::extract_text_from_image("image.png")?;
```

Convenience wrapper that builds the default engine and returns the
recognized text string.

## FAQ

**Q: Do I need the `ocr-train` feature at runtime?**
A: No. Prototype generation is a one-off build step. Production binaries
only need the `ocr` or `ocr-ml` feature plus a JSON prototype file (if
classical) or the auto-downloaded models (if ML).

**Q: Can I use the ML backend without internet access?**
A: Yes. On a machine that does have internet, run `omniparse models
download` (or `omniparse models download --force` to refresh) and then
copy the resulting cache directory to the offline host. Point the offline
binary at the copy via `OMNIPARSE_OCR_MODELS=/some/path`. The pre-built
Docker image (see project `Dockerfile`) already bakes the models in.

**Q: Why does the classical pipeline produce garbage on my photograph?**
A: Likely font mismatch. The classical recognizer shape-matches input
glyphs against trained prototypes. If the prototypes were trained from
Arial but the image is in Helvetica Neue, Futura, or the site's custom
brand font, shape distances won't match well. Identify the real font
(using WhatFont, Font Squirrel Matcherator, etc.) and retrain.

**Q: How much slower is ML OCR?**
A: Roughly 10–100× slower per image than the classical pipeline, but
still sub-second for most inputs on modern hardware. Enable
`ocr-parallel` if you're batching many images.

**Q: Can I train prototypes for non-Latin scripts?**
A: The training pipeline accepts any Unicode characters as the character
set argument. The feature extractor is script-agnostic. In practice the
bundled bigram table is English-only, so disable bigram rerank / beam
search for non-Latin scripts. The ML backend's bundled models are English-
only; multi-lingual models are on the ocrs-models roadmap.

**Q: Can I bundle models with my binary?**
A: Not on crates.io (10 MB package cap). For a private build, point
`OMNIPARSE_OCR_MODELS` at a known location and ship the `.rten` files
alongside your binary.

**Q: How do I disable the result cache?**

```sh
export OMNIPARSE_OCR_CACHE=0
```

**Q: Is there a version that runs in the browser / WASM?**
A: Not currently. The `image` crate decode path is WASM-compatible, and
`rten` has WASM support upstream, but the full omniparse build has not
been exercised against WASM targets. Try `cargo build --target wasm32-*
--features ocr-ml` and report issues.
