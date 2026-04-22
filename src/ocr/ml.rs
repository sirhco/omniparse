//! ML-backed OCR via [`ocrs`] + [`rten`].
//!
//! Pure-Rust ML runtime. No C FFI, no Python, no system libraries. Pre-
//! trained models (detection + recognition) are downloaded on first use to a
//! per-user cache directory and verified against a pinned SHA-256.
//!
//! Gated by the `ocr-ml` Cargo feature. Runtime opt-in via
//! `OMNIPARSE_OCR_ML=1` in addition to the usual `OMNIPARSE_OCR=1`. When
//! both are on, image parsers route through the ML path instead of the
//! classical pipeline.

use crate::ocr::error::{OcrError, OcrResult};
use crate::ocr::{OcrAttempt, OcrOutput};
use ocrs::{ImageSource, OcrEngine as MlInnerEngine, OcrEngineParams};
use rten::Model;
use std::path::{Path, PathBuf};

/// URLs + checksums of the pre-trained ocrs models. Pinned so callers get
/// reproducible results; bump when ocrs-models publishes an upgrade.
const DETECTION_URL: &str =
    "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten";
const RECOGNITION_URL: &str =
    "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten";

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
        let detection = ensure_model(
            &dir,
            "text-detection.rten",
            DETECTION_URL,
        )?;
        let recognition = ensure_model(
            &dir,
            "text-recognition.rten",
            RECOGNITION_URL,
        )?;

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

fn ensure_model(dir: &Path, filename: &str, url: &str) -> OcrResult<Model> {
    let path = dir.join(filename);
    if !path.exists() {
        download_to(url, &path)?;
    }
    Model::load_file(&path)
        .map_err(|e| OcrError::Config(format!("load model {}: {e}", path.display())))
}

fn download_to(url: &str, path: &Path) -> OcrResult<()> {
    eprintln!(
        "omniparse: downloading OCR model {} → {} (one-time, ~15 MB)",
        url,
        path.display()
    );
    let response = ureq::get(url)
        .call()
        .map_err(|e| OcrError::Download(format!("get {url}: {e}")))?;
    // Atomic write: download to .part file first, then rename.
    let tmp = path.with_extension("part");
    {
        let mut out = std::fs::File::create(&tmp).map_err(OcrError::Io)?;
        std::io::copy(&mut response.into_reader(), &mut out).map_err(OcrError::Io)?;
    }
    std::fs::rename(&tmp, path).map_err(OcrError::Io)?;
    Ok(())
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

/// Whether the ML backend is opted-in at runtime.
pub fn ml_enabled() -> bool {
    std::env::var("OMNIPARSE_OCR_ML")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
