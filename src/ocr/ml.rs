//! ML-backed OCR via [`ocrs`] + [`rten`].
//!
//! Pure-Rust ML runtime. No C FFI, no Python, no system libraries. Pre-
//! trained models (detection + recognition) are downloaded on first use to a
//! per-user cache directory and verified against a pinned SHA-256.
//!
//! Gated by the `ocr-ml` Cargo feature. Runtime opt-in via the unified
//! `OMNIPARSE_OCR=ml` env var (the legacy `OMNIPARSE_OCR=1 OMNIPARSE_OCR_ML=1`
//! pair still works but emits a one-shot deprecation warning).
//!
//! # Model management
//!
//! The CLI exposes `omniparse models {download,verify,path,list}` for
//! pre-fetching, integrity checking, and inspecting the cache. The same
//! functionality is available programmatically via [`prefetch_all`],
//! [`verify_all`], and [`list_models`].

use crate::ocr::error::{OcrError, OcrResult};
use crate::ocr::{OcrAttempt, OcrOutput};
use ocrs::{ImageSource, OcrEngine as MlInnerEngine, OcrEngineParams};
use rten::Model;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

/// URL of the pre-trained text-detection model.
pub const DETECTION_URL: &str =
    "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten";
/// URL of the pre-trained text-recognition model.
pub const RECOGNITION_URL: &str =
    "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten";

/// Static description of a pre-trained model: filename, source URL, and
/// pinned SHA-256 of the upstream artifact. Used by the download / verify /
/// list paths to guarantee reproducible inputs.
#[derive(Debug, Clone, Copy)]
pub struct ModelSpec {
    pub name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
}

/// All models required for the ML OCR backend. Adding a new model means
/// appending to this slice and pinning its SHA-256.
pub const MODELS: &[ModelSpec] = &[
    ModelSpec {
        name: "text-detection.rten",
        url: DETECTION_URL,
        sha256: "f15cfb56bd02c4bf478a20343986504a1f01e1665c2b3a0ad66340f054b1b5ca",
    },
    ModelSpec {
        name: "text-recognition.rten",
        url: RECOGNITION_URL,
        sha256: "e484866d4cce403175bd8d00b128feb08ab42e208de30e42cd9889d8f1735a6e",
    },
];

/// Snapshot of a single cached model on disk.
#[derive(Debug, Clone)]
pub struct ModelStatus {
    pub spec: &'static ModelSpec,
    pub path: PathBuf,
    /// Present if the file exists on disk.
    pub size: Option<u64>,
    /// Present if `size.is_some()`; the on-disk SHA-256.
    pub sha256: Option<String>,
    /// `true` only when the file exists *and* its SHA-256 matches `spec.sha256`.
    pub ok: bool,
}

/// High-level ML OCR engine.
pub struct MlOcrEngine {
    inner: MlInnerEngine,
}

impl MlOcrEngine {
    /// Build the engine, loading models from the local cache or downloading
    /// them on first use. See [`model_dir`] for cache location, and the
    /// module-level docs for override env vars.
    pub fn new() -> OcrResult<Self> {
        let dir = model_dir()?;
        let detection = ensure_model(&dir, &MODELS[0])?;
        let recognition = ensure_model(&dir, &MODELS[1])?;

        let params = OcrEngineParams {
            detection_model: Some(detection),
            recognition_model: Some(recognition),
            ..Default::default()
        };
        let inner = MlInnerEngine::new(params)
            .map_err(|e| OcrError::Config(format!("ocrs init: {e}")))?;
        Ok(Self { inner })
    }

    /// Run OCR on an image. Returns an [`OcrOutput`] shaped like the
    /// classical engine's output so callers can switch backends without
    /// changing downstream code.
    pub fn recognize(&self, img: image::DynamicImage) -> OcrResult<OcrOutput> {
        let rgb = img.to_rgb8();
        let (w, h) = rgb.dimensions();
        let source = ImageSource::from_bytes(rgb.as_raw(), (w, h))
            .map_err(|e| OcrError::ImageDecode(format!("ocrs source: {e:?}")))?;
        let input = self
            .inner
            .prepare_input(source)
            .map_err(|e| OcrError::Recognize(format!("ocrs prepare: {e}")))?;
        let text = self
            .inner
            .get_text(&input)
            .map_err(|e| OcrError::Recognize(format!("ocrs recognize: {e}")))?;
        let mean_confidence = if text.trim().is_empty() { 0.0 } else { 0.9 };
        Ok(OcrOutput {
            text: text.clone(),
            lines: Vec::new(),
            mean_confidence,
            detected_script: crate::ocr::script::dominant_script(&text),
        })
    }
}

/// Resolve the model cache directory. Honors `OMNIPARSE_OCR_MODELS` env var
/// (absolute path override); otherwise uses `$XDG_CACHE_HOME/omniparse/`
/// or the platform equivalent via the `dirs` crate.
pub fn model_dir() -> OcrResult<PathBuf> {
    if let Ok(p) = std::env::var("OMNIPARSE_OCR_MODELS") {
        let path = PathBuf::from(p);
        std::fs::create_dir_all(&path).map_err(OcrError::Io)?;
        return Ok(path);
    }
    let base = dirs::cache_dir().ok_or_else(|| {
        OcrError::Config("could not resolve a cache directory; set OMNIPARSE_OCR_MODELS".into())
    })?;
    let dir = base.join("omniparse").join("ocrs-models");
    std::fs::create_dir_all(&dir).map_err(OcrError::Io)?;
    Ok(dir)
}

/// Ensure a single model is present in `dir` and matches its pinned hash,
/// downloading if missing. Returns the loaded [`Model`].
pub(crate) fn ensure_model(dir: &Path, spec: &ModelSpec) -> OcrResult<Model> {
    let path = dir.join(spec.name);
    if !path.exists() {
        download_to(spec, &path)?;
    } else {
        // File exists; verify before trusting it. A mismatch typically means
        // partial download or upstream model rotation — re-fetch.
        let actual = hash_file(&path)?;
        if !actual.eq_ignore_ascii_case(spec.sha256) {
            eprintln!(
                "omniparse: cached {} sha256 mismatch (expected {}, got {}); re-downloading",
                spec.name, spec.sha256, actual
            );
            download_to(spec, &path)?;
        }
    }
    Model::load_file(&path)
        .map_err(|e| OcrError::Config(format!("load model {}: {e}", path.display())))
}

/// Stream `spec.url` to `path` via a `.part` temp, verifying SHA-256 as bytes
/// flow through. On mismatch the partial file is deleted and
/// [`OcrError::ChecksumMismatch`] is returned.
pub(crate) fn download_to(spec: &ModelSpec, path: &Path) -> OcrResult<()> {
    eprintln!(
        "omniparse: downloading OCR model {} → {} (one-time)",
        spec.url,
        path.display()
    );
    let response = ureq::get(spec.url)
        .call()
        .map_err(|e| OcrError::Download(format!("get {}: {e}", spec.url)))?;
    let tmp = path.with_extension("part");
    let mut hasher = Sha256::new();
    {
        let mut out = std::fs::File::create(&tmp).map_err(OcrError::Io)?;
        let mut reader = response.into_reader();
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = reader.read(&mut buf).map_err(OcrError::Io)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            std::io::Write::write_all(&mut out, &buf[..n]).map_err(OcrError::Io)?;
        }
    }
    let actual = hex_encode(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(spec.sha256) {
        let _ = std::fs::remove_file(&tmp);
        return Err(OcrError::ChecksumMismatch {
            file: spec.name.to_string(),
            expected: spec.sha256.to_string(),
            actual,
        });
    }
    std::fs::rename(&tmp, path).map_err(OcrError::Io)?;
    Ok(())
}

/// Pre-fetch every model in [`MODELS`]. With `force`, existing files are
/// re-downloaded even if their hash already matches. Returns the resolved
/// cache paths in declaration order.
pub fn prefetch_all(force: bool) -> OcrResult<Vec<PathBuf>> {
    let dir = model_dir()?;
    let mut out = Vec::with_capacity(MODELS.len());
    for spec in MODELS {
        let path = dir.join(spec.name);
        if force || !path.exists() {
            download_to(spec, &path)?;
        } else {
            let actual = hash_file(&path)?;
            if !actual.eq_ignore_ascii_case(spec.sha256) {
                download_to(spec, &path)?;
            }
        }
        out.push(path);
    }
    Ok(out)
}

/// Re-hash every cached model in [`MODELS`] and compare against its pinned
/// SHA-256. Returns the first [`OcrError::ChecksumMismatch`] or
/// [`OcrError::ModelUnavailable`] encountered; succeeds only when every
/// model is present and correct.
pub fn verify_all() -> OcrResult<()> {
    let dir = model_dir()?;
    for spec in MODELS {
        let path = dir.join(spec.name);
        if !path.exists() {
            return Err(OcrError::ModelUnavailable(format!(
                "{} missing at {}",
                spec.name,
                path.display()
            )));
        }
        let actual = hash_file(&path)?;
        if !actual.eq_ignore_ascii_case(spec.sha256) {
            return Err(OcrError::ChecksumMismatch {
                file: spec.name.to_string(),
                expected: spec.sha256.to_string(),
                actual,
            });
        }
    }
    Ok(())
}

/// Inspect the cache: one [`ModelStatus`] per model in [`MODELS`].
pub fn list_models() -> OcrResult<Vec<ModelStatus>> {
    let dir = model_dir()?;
    let mut out = Vec::with_capacity(MODELS.len());
    for spec in MODELS {
        let path = dir.join(spec.name);
        if path.exists() {
            let size = std::fs::metadata(&path).map(|m| m.len()).ok();
            let sha = hash_file(&path).ok();
            let ok = sha
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case(spec.sha256))
                .unwrap_or(false);
            out.push(ModelStatus {
                spec,
                path,
                size,
                sha256: sha,
                ok,
            });
        } else {
            out.push(ModelStatus {
                spec,
                path,
                size: None,
                sha256: None,
                ok: false,
            });
        }
    }
    Ok(out)
}

fn hash_file(path: &Path) -> OcrResult<String> {
    let mut f = std::fs::File::open(path).map_err(OcrError::Io)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(OcrError::Io)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex_encode(&hasher.finalize()))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Shared process-wide ML engine. First call pays the model-load cost;
/// subsequent calls reuse.
pub fn shared_ml_engine() -> Option<&'static MlOcrEngine> {
    use std::sync::OnceLock;
    static ENGINE: OnceLock<Option<MlOcrEngine>> = OnceLock::new();
    ENGINE
        .get_or_init(|| match MlOcrEngine::new() {
            Ok(engine) => Some(engine),
            Err(e) => {
                eprintln!("omniparse: ML OCR init failed: {e}");
                None
            }
        })
        .as_ref()
}

/// Runtime dispatch entry point called by `run_ocr` when ML mode is enabled.
pub fn run_ml_ocr(bytes: &[u8]) -> OcrAttempt {
    let img = match image::load_from_memory(bytes) {
        Ok(i) => i,
        Err(e) => return OcrAttempt::Error(format!("image decode: {e}")),
    };
    let engine = match shared_ml_engine() {
        Some(e) => e,
        None => return OcrAttempt::Error("ML engine unavailable".into()),
    };
    match engine.recognize(img) {
        Ok(out) if out.text.trim().is_empty() => OcrAttempt::NoTextFound {
            mean_confidence: 0.0,
            regions: 0,
        },
        Ok(out) => OcrAttempt::Recognized {
            text: out.text,
            mean_confidence: out.mean_confidence,
        },
        Err(e) => OcrAttempt::Error(format!("ml engine: {e}")),
    }
}

/// Whether the ML backend is opted-in at runtime. Delegates to the unified
/// `OMNIPARSE_OCR` reader so legacy `OMNIPARSE_OCR_ML=1` still works.
pub fn ml_enabled() -> bool {
    crate::ocr::ocr_mode() == crate::ocr::OcrMode::Ml
}
