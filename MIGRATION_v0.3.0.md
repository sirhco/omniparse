# Migrating from omniparse 0.2.x to 0.3.0

This release adds a substantial amount of new capability but also breaks a
handful of narrow APIs. Most v0.2 deployments upgrade with a single
`cargo update` and no code changes. This document is an exhaustive map for
the minority that hit an edge case.

## 1. Cargo.toml

### Version bump

```toml
[dependencies]
-omniparse = "0.2"
+omniparse = "0.3"
```

### Minimum Rust version

v0.3.0 requires **Rust 1.88+** (stabilized `if let … && let` chain syntax
is used internally).

```sh
rustup update stable
```

### Feature flags

Default features added:

```
markdown, svg, webp, epub, mp3
```

If you want the smallest build, opt out explicitly:

```toml
omniparse = { version = "0.3", default-features = false }
```

New optional features:

```
ocr, ocr-train, ocr-parallel, ocr-ml
```

None of these are on by default; nothing changes for you unless you
enable them.

## 2. Behavioral changes (no code changes required, but output differs)

### PNG compressed-text chunks now contain real text

Before (v0.2):

```json
{
  "ztext_Description": "[compressed text]"
}
```

After (v0.3):

```json
{
  "ztext_Description": "A photograph taken on holiday in Prague"
}
```

If you had code that looked for the literal placeholder to skip processing
compressed chunks, remove that branch — the value is now the real content.

### JPEG `exif_present` is stricter

Before: set to `true` whenever the JPEG had an APP1 marker with the `Exif`
identifier, even if the payload was malformed.

After: only set when `kamadak-exif` successfully parses the EXIF block.

### TIFF `tiff_*` keys removed

Before (v0.2) you could read placeholder metadata like:

```rust
metadata.get("tiff_Make")      // → Some(Text("tag_271"))
metadata.get("tiff_DateTime")  // → Some(Text("tag_306"))
```

Those were never real values — they were the tag ID in string form. They
are now **removed entirely**. Replace with the EXIF-derived keys:

```rust
// Before
metadata.get("tiff_Make")

// After
metadata.get("camera_make")
```

Full replacement map:

| Old `tiff_*` key       | New key                    |
| ---------------------- | -------------------------- |
| `tiff_Make`            | `camera_make`              |
| `tiff_Model`           | `camera_model`             |
| `tiff_Software`        | `software`                 |
| `tiff_Artist`          | `artist`                   |
| `tiff_DateTime`        | `datetime`                 |
| `tiff_Orientation`     | `orientation`              |
| `tiff_ImageWidth`      | (use top-level `width`)    |
| `tiff_ImageLength`     | (use top-level `height`)   |
| `tiff_ImageDescription`| `image_description`        |

### Detection: `<?xml` alone no longer triggers `image/svg+xml`

v0.2's magic table mapped the bare `<?xml` header to SVG, which caused
every XML document to be misrouted. v0.3 requires the literal `<svg` bytes.
Custom parsers registered for XML-like formats will now see their MIME
types correctly.

## 3. Source-level breaks

### `FeatureVec` (OCR internals)

Only relevant if you built something on the OCR module directly.

Before:

```rust
pub type FeatureVec = [f32; 29];
```

After:

```rust
pub type FeatureVec = Vec<f32>;
pub const FEATURE_COUNT: usize = 55;
```

Construct via `zero_features()`:

```rust
let mut f = omniparse::ocr::features::zero_features();
f[0] = 1.0;
```

### `ParserRegistry` default is now cached

Before (still supported, still valid):

```rust
let registry = omniparse::parsers::ParserRegistry::default();  // fresh copy
```

New path for callers that want the shared instance:

```rust
let registry: &'static _ = omniparse::parsers::default_registry();
```

No breaking change — the old form still compiles. The new helper just
avoids rebuilding on every call.

### OCR: prototype JSON format

`FEATURE_COUNT` grew from 29 → 32 → 55 during v0.3 development. Any JSON
prototype file created against an older build **will fail to load** with a
descriptive error:

```
prototype 'A' has 32 features but this build expects 55. The feature vector
was extended in a recent release — retrain with `cargo run --features
ocr-train --example train_prototypes`.
```

Regenerate with:

```sh
cargo run --features ocr-train --example train_prototypes -- \
    /path/to/Font.ttf prototypes.json 24,48,96
```

### OCR: `OMNIPARSE_OCR_PROTOTYPES` now fails loudly

Before (v0.3 pre-release): missing / unreadable path printed a warning and
silently fell back to the bundled bitmap prototypes. v0.3 GA terminates
the process with a descriptive error. Unset the env var to opt into the
bundled set intentionally.

## 4. What's *not* broken

- All existing public APIs from v0.2 (`extract_from_path`,
  `extract_from_bytes`, `extract_from_path_async`, `supported_mime_types`,
  `is_mime_supported`, `core::Extractor`, `Parser` trait,
  `ParserRegistry::register`, `TypeDetector`, `Content`, `Metadata`,
  `MetadataValue`) are source-compatible.
- The CLI binary accepts the same flags.
- All v0.2 metadata keys that corresponded to real values still exist with
  the same meanings.

## 5. Quick upgrade checklist

1. `cargo update -p omniparse` (or bump manually in `Cargo.toml`).
2. `rustup update stable` if below 1.88.
3. If you read `tiff_*` metadata keys → switch to the new EXIF-derived
   keys (table above).
4. If you pattern-matched on `"[compressed text]"` → drop the branch.
5. If you persist OCR prototype JSON → regenerate with the training
   example.
6. If you had code routing bare XML through `image/svg+xml` → the MIME
   detector now returns the right type; remove the workaround.

## 6. Optional additions

If you want the new capabilities:

- **OCR**: enable `--features ocr` (classical) or `--features ocr-ml` (ML).
- **Five new formats**: no action needed — on by default.
- **Registry caching**: replace `ParserRegistry::default()` hot-loop calls
  with `default_registry()`.

---

For the full list of additions and enhancements, see
[`RELEASE_NOTES_v0.3.0.md`](RELEASE_NOTES_v0.3.0.md).
