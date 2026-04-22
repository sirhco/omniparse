//! Pure-Rust classical OCR pipeline. **No machine-learning runtime, no
//! downloaded models.** Four stages plug together through traits so callers
//! can swap any stage:
//!
//! 1. [`preprocess`] — clean raster input (deskew, binarize, despeckle).
//! 2. [`layout`]     — locate per-glyph regions via connected components.
//! 3. [`recognize`]  — classify each glyph via hand-crafted features + 1-NN.
//! 4. [`postprocess`]— dictionary-correct the concatenated string.
//!
//! # Accuracy caveat
//!
//! A from-scratch classical recognizer is dramatically less accurate than
//! Tesseract or a neural OCR pipeline. Expect useful results only on clean,
//! well-exposed printed Latin text. Handwriting, rotated scans, low DPI, and
//! unusual fonts will fail.
//!
//! # Example
//!
//! ```no_run
//! # #[cfg(feature = "ocr")]
//! # fn main() -> Result<(), omniparse::Error> {
//! use omniparse::ocr::OcrEngine;
//! let engine = OcrEngine::new();
//! let image = image::open("page.png")
//!     .map_err(|e| omniparse::Error::ParseError(e.to_string()))?;
//! let output = engine.recognize(image)?;
//! println!("{}", output.text);
//! # Ok(()) }
//! # #[cfg(not(feature = "ocr"))] fn main() {}
//! ```

pub mod bigram;
pub mod cache;
pub mod color_seg;
pub mod error;
pub mod features;
pub mod kdtree;
pub mod layout;
pub mod mser;
#[cfg(feature = "ocr-ml")]
pub mod ml;
pub mod postprocess;
pub mod preprocess;
pub mod prototypes;
pub mod recognize;
pub mod script;
pub mod swt;
#[cfg(feature = "ocr-train")]
pub mod train;

use crate::core::Result;
use error::OcrError;
use image::DynamicImage;
use layout::{ConnectedComponentAnalyzer, LayoutAnalyzer};
use postprocess::{NoopCorrector, PostProcessor};
use preprocess::{ImageprocPreprocessor, Preprocessor};
use recognize::{FeatureRecognizer, RecognizedLine, Recognizer};

/// Engine configuration.
#[derive(Clone, Debug)]
pub struct OcrConfig {
    /// Whether image parsers should invoke OCR on decode. Defaults to `false`
    /// so flipping the `ocr` Cargo feature doesn't silently slow every image
    /// parse; callers opt in via `OcrConfig::enabled = true` or the
    /// `OMNIPARSE_OCR=1` env var.
    pub enabled: bool,
    /// Drop recognized lines below this mean confidence.
    pub min_confidence: f32,
    /// Apply dictionary post-processing. The default is off because the
    /// recognizer's raw output is more useful for debugging early on.
    pub spellcheck: bool,
    /// Try rotating the image 90/180/270° and pick the orientation that
    /// produces the highest `recognized_text_length × mean_confidence`
    /// score. Useful for photos of pages held sideways. 4× runtime cost when
    /// enabled; off by default.
    pub auto_rotate: bool,
    /// Run character-bigram re-ranking over per-line candidates. Requires the
    /// recognizer to populate `RecognizedLine::alternatives` (the default
    /// `FeatureRecognizer` does). Off by default.
    pub bigram_rerank: bool,
    /// Run word-level beam search over per-line candidates using the
    /// bundled wordlist. Implies `bigram_rerank` is skipped (beam search is
    /// a strict superset).
    pub beam_search: bool,
    /// Width of the beam-search beam. Default 8.
    pub beam_width: usize,
    /// Reject post-layout regions whose stroke-width coefficient-of-variation
    /// exceeds this threshold. `None` disables the filter.
    pub stroke_width_cv_max: Option<f32>,
    /// Reject text lines whose glyph height or spacing variance exceeds
    /// reasonable bounds (heuristic rejection of photographic edge clusters).
    /// Off by default.
    pub text_line_filter: bool,
    /// Reject individual regions lacking at least this many similar-sized
    /// co-linear neighbors. Strong noise suppressor on photographic inputs:
    /// real text glyphs always appear in clusters, photo edges don't.
    /// Suggested 2–4 for line-dense documents, 1 for short captions,
    /// `None` to disable.
    pub neighbor_density_min: Option<usize>,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_confidence: default_min_confidence_from_env().unwrap_or(0.15),
            spellcheck: false,
            auto_rotate: env_flag("OMNIPARSE_OCR_AUTO_ROTATE"),
            bigram_rerank: env_flag("OMNIPARSE_OCR_BIGRAM"),
            beam_search: env_flag("OMNIPARSE_OCR_BEAM"),
            beam_width: std::env::var("OMNIPARSE_OCR_BEAM_WIDTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8),
            stroke_width_cv_max: std::env::var("OMNIPARSE_OCR_SW_CV_MAX")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| *v > 0.0 && *v <= 2.0),
            text_line_filter: env_flag("OMNIPARSE_OCR_LINE_FILTER"),
            neighbor_density_min: std::env::var("OMNIPARSE_OCR_NEIGHBOR_MIN")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .filter(|v| *v > 0),
        }
    }
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn default_min_confidence_from_env() -> Option<f32> {
    std::env::var("OMNIPARSE_OCR_MIN_CONFIDENCE")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|v| (0.0..=1.0).contains(v))
}

/// End-to-end OCR output.
#[derive(Clone, Debug)]
pub struct OcrOutput {
    pub text: String,
    pub lines: Vec<RecognizedLine>,
    pub mean_confidence: f32,
    /// Dominant script detected in the recognized text, if any.
    pub detected_script: Option<script::Script>,
}

/// End-to-end OCR engine. Build via [`OcrEngine::new`] for defaults or
/// [`OcrEngine::builder`] for component swaps.
pub struct OcrEngine {
    pre: Box<dyn Preprocessor>,
    layout: Box<dyn LayoutAnalyzer>,
    recog: Box<dyn Recognizer>,
    post: Box<dyn PostProcessor>,
    cfg: OcrConfig,
}

impl OcrEngine {
    /// Build an engine with default stages.
    ///
    /// - Preprocessor: [`ImageprocPreprocessor`]
    /// - Layout: [`ConnectedComponentAnalyzer`]
    /// - Recognizer: [`FeatureRecognizer`] with the bundled (currently empty)
    ///   reference prototypes
    /// - Postprocessor: [`NoopCorrector`]
    ///
    /// The default recognizer ships with no prototypes — attaching a labeled
    /// reference set is a required step for real use. See
    /// [`OcrEngine::builder`].
    pub fn new() -> Self {
        Self {
            pre: Box::new(ImageprocPreprocessor::new()),
            layout: Box::new(ConnectedComponentAnalyzer::default()),
            recog: Box::new(FeatureRecognizer::with_default_prototypes()),
            post: Box::new(NoopCorrector),
            cfg: OcrConfig::default(),
        }
    }

    pub fn builder() -> OcrEngineBuilder {
        OcrEngineBuilder::default()
    }

    pub fn config(&self) -> &OcrConfig {
        &self.cfg
    }

    /// Run the full pipeline on an image.
    pub fn recognize(&self, img: DynamicImage) -> Result<OcrOutput> {
        if self.cfg.auto_rotate {
            return self.recognize_with_auto_rotate(img);
        }
        self.recognize_once(img)
    }

    fn recognize_with_auto_rotate(&self, img: DynamicImage) -> Result<OcrOutput> {
        let orientations = [
            img.clone(),
            image::imageops::rotate90(&img.to_rgba8()).into(),
            image::imageops::rotate180(&img.to_rgba8()).into(),
            image::imageops::rotate270(&img.to_rgba8()).into(),
        ];
        let mut best: Option<OcrOutput> = None;
        let mut best_score = f32::NEG_INFINITY;
        for candidate in orientations {
            let out = self.recognize_once(candidate)?;
            let score = out.text.trim().len() as f32 * out.mean_confidence;
            if score > best_score {
                best_score = score;
                best = Some(out);
            }
        }
        Ok(best.unwrap_or(OcrOutput {
            text: String::new(),
            lines: Vec::new(),
            mean_confidence: 0.0,
            detected_script: None,
        }))
    }

    fn recognize_once(&self, img: DynamicImage) -> Result<OcrOutput> {
        let debug_dir = std::env::var("OMNIPARSE_OCR_DEBUG_DIR")
            .ok()
            .map(std::path::PathBuf::from)
            .filter(|p| std::fs::create_dir_all(p).is_ok());
        if let Some(dir) = debug_dir.as_ref() {
            let _ = img.to_rgb8().save(dir.join("01_input.png"));
        }
        self.recognize_inner(img, debug_dir.as_deref())
    }

    fn recognize_inner(
        &self,
        img: DynamicImage,
        debug_dir: Option<&std::path::Path>,
    ) -> Result<OcrOutput> {
        let gray = self.pre.process(img).map_err(crate::core::Error::from)?;
        if let Some(dir) = debug_dir {
            let _ = gray.save(dir.join("02_preprocessed.png"));
        }
        let mut regions = self.layout.detect_regions(&gray).map_err(crate::core::Error::from)?;
        if let Some(cv_max) = self.cfg.stroke_width_cv_max {
            regions = layout::filter_by_stroke_width_constancy(&gray, regions, cv_max);
        }
        if let Some(min_n) = self.cfg.neighbor_density_min {
            regions = layout::filter_by_neighbor_density(regions, min_n, 0.5, 2.5, 0.5);
        }
        if self.cfg.text_line_filter {
            regions = layout::filter_text_lines(regions);
        }
        if let Some(dir) = debug_dir {
            let _ = draw_region_overlay(&gray, &regions).save(dir.join("03_layout.png"));
        }

        // Region recognition — optionally parallel when the `ocr-parallel`
        // feature enables rayon. Sequential path is used otherwise to keep
        // the non-parallel build fully deterministic.
        #[cfg(feature = "ocr-parallel")]
        let recognized: Vec<RecognizedLine> = {
            use rayon::prelude::*;
            regions
                .par_iter()
                .map(|r| self.recog.recognize(&gray, r))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(crate::core::Error::from)?
        };
        #[cfg(not(feature = "ocr-parallel"))]
        let recognized: Vec<RecognizedLine> = {
            let mut out = Vec::with_capacity(regions.len());
            for region in &regions {
                out.push(
                    self.recog
                        .recognize(&gray, region)
                        .map_err(crate::core::Error::from)?,
                );
            }
            out
        };

        let mut lines: Vec<RecognizedLine> = Vec::with_capacity(recognized.len());
        let mut confidences: Vec<f32> = Vec::with_capacity(recognized.len());
        for line in recognized {
            if line.confidence < self.cfg.min_confidence {
                continue;
            }
            confidences.push(line.confidence);
            lines.push(line);
        }

        let grouped = group_into_lines(lines.clone());
        let raw = if self.cfg.beam_search {
            render_lines_with(&grouped, |line_glyphs| {
                postprocess::beam_search_line(
                    line_glyphs,
                    self.cfg.beam_width,
                    postprocess::DEFAULT_WORDLIST,
                )
            })
        } else if self.cfg.bigram_rerank {
            render_lines_with(&grouped, |line_glyphs| {
                bigram::BigramRanker::english().rerank_line(line_glyphs)
            })
        } else {
            render_lines(&grouped)
        };

        let text = if self.cfg.spellcheck {
            self.post.correct(&raw)
        } else {
            raw
        };

        let mean_confidence = if confidences.is_empty() {
            0.0
        } else {
            confidences.iter().sum::<f32>() / confidences.len() as f32
        };

        let detected_script = script::dominant_script(&text);

        Ok(OcrOutput {
            text,
            lines,
            mean_confidence,
            detected_script,
        })
    }
}

impl Default for OcrEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for composing a custom pipeline.
#[derive(Default)]
pub struct OcrEngineBuilder {
    pre: Option<Box<dyn Preprocessor>>,
    layout: Option<Box<dyn LayoutAnalyzer>>,
    recog: Option<Box<dyn Recognizer>>,
    post: Option<Box<dyn PostProcessor>>,
    cfg: Option<OcrConfig>,
}

impl OcrEngineBuilder {
    pub fn preprocessor<P: Preprocessor + 'static>(mut self, p: P) -> Self {
        self.pre = Some(Box::new(p));
        self
    }
    pub fn layout<L: LayoutAnalyzer + 'static>(mut self, l: L) -> Self {
        self.layout = Some(Box::new(l));
        self
    }
    pub fn recognizer<R: Recognizer + 'static>(mut self, r: R) -> Self {
        self.recog = Some(Box::new(r));
        self
    }
    pub fn postprocessor<P: PostProcessor + 'static>(mut self, p: P) -> Self {
        self.post = Some(Box::new(p));
        self
    }
    pub fn config(mut self, cfg: OcrConfig) -> Self {
        self.cfg = Some(cfg);
        self
    }
    pub fn build(self) -> OcrEngine {
        OcrEngine {
            pre: self.pre.unwrap_or_else(|| Box::new(ImageprocPreprocessor::new())),
            layout: self
                .layout
                .unwrap_or_else(|| Box::new(ConnectedComponentAnalyzer::default())),
            recog: self.recog.unwrap_or_else(|| Box::new(FeatureRecognizer::with_default_prototypes())),
            post: self.post.unwrap_or_else(|| Box::new(NoopCorrector)),
            cfg: self.cfg.unwrap_or_default(),
        }
    }
}

/// Group recognized glyphs into visual lines using vertical-overlap heuristics.
///
/// Two glyphs belong to the same line if the overlap of their vertical
/// extents, divided by the smaller of their heights, exceeds `0.5`. Each line
/// is sorted left-to-right by `region.x`.
fn group_into_lines(mut glyphs: Vec<RecognizedLine>) -> Vec<Vec<RecognizedLine>> {
    glyphs.sort_by(|a, b| a.region.y.cmp(&b.region.y).then_with(|| a.region.x.cmp(&b.region.x)));

    let mut lines: Vec<Vec<RecognizedLine>> = Vec::new();
    for glyph in glyphs {
        let g_top = glyph.region.y;
        let g_bot = glyph.region.y + glyph.region.height;
        let g_h = glyph.region.height.max(1);

        let placed = lines.iter_mut().any(|line| {
            let (top, bot, h) = line_vspan(line);
            let overlap = g_bot.min(bot).saturating_sub(g_top.max(top));
            let smaller = g_h.min(h.max(1));
            overlap as f32 / smaller as f32 >= 0.5
        });
        if placed {
            // Find the line we matched and push (linear scan — lines per image is small).
            for line in lines.iter_mut() {
                let (top, bot, h) = line_vspan(line);
                let overlap = g_bot.min(bot).saturating_sub(g_top.max(top));
                let smaller = g_h.min(h.max(1));
                if overlap as f32 / smaller as f32 >= 0.5 {
                    line.push(glyph);
                    break;
                }
            }
        } else {
            lines.push(vec![glyph]);
        }
    }

    // Sort each line left-to-right, and lines top-to-bottom by the topmost glyph.
    for line in lines.iter_mut() {
        line.sort_by_key(|g| g.region.x);
    }
    lines.sort_by_key(|line| line.iter().map(|g| g.region.y).min().unwrap_or(0));
    lines
}

fn line_vspan(line: &[RecognizedLine]) -> (u32, u32, u32) {
    let top = line.iter().map(|g| g.region.y).min().unwrap_or(0);
    let bot = line
        .iter()
        .map(|g| g.region.y + g.region.height)
        .max()
        .unwrap_or(0);
    (top, bot, bot.saturating_sub(top))
}

/// Concatenate grouped glyphs into a string, inserting spaces between words
/// and newlines between lines.
fn render_lines(lines: &[Vec<RecognizedLine>]) -> String {
    render_lines_with(lines, |line| {
        line.iter().map(|g| g.text.clone()).collect::<String>()
    })
}

/// Render-lines variant that delegates per-line text rendering to `renderer`.
/// Spaces and newlines between lines are still managed here.
fn render_lines_with<F>(lines: &[Vec<RecognizedLine>], mut renderer: F) -> String
where
    F: FnMut(&[RecognizedLine]) -> String,
{
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let raw_chars = renderer(line);
        if raw_chars.is_empty() {
            continue;
        }
        // Space threshold: 0.4× median glyph width on this line. Gaps wider
        // than that are treated as word boundaries.
        let mut widths: Vec<u32> = line.iter().map(|g| g.region.width).collect();
        widths.sort_unstable();
        let median_w = widths.get(widths.len() / 2).copied().unwrap_or(1);
        let space_threshold = (median_w as f32 * 0.4).max(1.0);

        let mut prev_right: Option<u32> = None;
        let mut chars = raw_chars.chars();
        for glyph in line {
            if let Some(right) = prev_right {
                let gap = glyph.region.x.saturating_sub(right);
                if gap as f32 >= space_threshold {
                    out.push(' ');
                }
            }
            // Pull one char from renderer output per glyph; if renderer emits
            // fewer chars than glyphs, fall back to glyph.text.
            match chars.next() {
                Some(c) => out.push(c),
                None => out.push_str(&glyph.text),
            }
            prev_right = Some(glyph.region.x + glyph.region.width);
        }
    }
    out
}

/// Render region bounding boxes on top of the grayscale image for
/// diagnostics.
fn draw_region_overlay(
    gray: &image::GrayImage,
    regions: &[layout::TextRegion],
) -> image::RgbImage {
    let (w, h) = gray.dimensions();
    let mut out = image::RgbImage::new(w, h);
    for (x, y, px) in gray.enumerate_pixels() {
        let v = px[0];
        out.put_pixel(x, y, image::Rgb([v, v, v]));
    }
    let red = image::Rgb([255u8, 0, 0]);
    for r in regions {
        let x1 = (r.x + r.width).min(w);
        let y1 = (r.y + r.height).min(h);
        for x in r.x..x1 {
            if r.y < h {
                out.put_pixel(x, r.y, red);
            }
            if y1 > 0 && y1 - 1 < h {
                out.put_pixel(x, y1 - 1, red);
            }
        }
        for y in r.y..y1 {
            if r.x < w {
                out.put_pixel(r.x, y, red);
            }
            if x1 > 0 && x1 - 1 < w {
                out.put_pixel(x1 - 1, y, red);
            }
        }
    }
    out
}

/// Runtime gate checked by image parsers. `true` when explicitly opted in via
/// `OMNIPARSE_OCR=1`; otherwise `false` so enabling the `ocr` Cargo feature
/// alone does not silently slow every image parse.
pub fn runtime_enabled() -> bool {
    std::env::var("OMNIPARSE_OCR")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Convenience: run OCR against a file path.
pub fn extract_text_from_image(path: impl AsRef<std::path::Path>) -> Result<String> {
    let img = image::open(path.as_ref())
        .map_err(|e| OcrError::ImageDecode(e.to_string()))
        .map_err(crate::core::Error::from)?;
    let engine = OcrEngine::new();
    Ok(engine.recognize(img)?.text)
}

/// Shared engine initialized once per process. Used by image parsers so each
/// parse doesn't rebuild prototypes.
///
/// Env overrides (all optional):
/// - `OMNIPARSE_OCR_PROTOTYPES=<json-path>` — load prototypes from JSON.
/// - `OMNIPARSE_OCR_LAYOUT=cca|swt` — layout analyzer choice.
/// - `OMNIPARSE_OCR_BINARIZE=otsu|sauvola|adaptive_mean|disabled` —
///   binarization method. Sauvola/AdaptiveMean use a 25-pixel window by
///   default; override with `OMNIPARSE_OCR_BIN_WINDOW=<int>`.
/// - `OMNIPARSE_OCR_CLAHE=1` — enable CLAHE before binarization.
/// - `OMNIPARSE_OCR_TOPHAT=<radius>` — morphological top-hat radius (0 off).
/// - `OMNIPARSE_OCR_K=<int>` — k-NN vote count on the default recognizer.
/// - `OMNIPARSE_OCR_AUTO_ROTATE=1` — try 90/180/270° and keep best score.
/// - `OMNIPARSE_OCR_DESPECKLE=<radius>` — median filter radius (0 off).
/// - `OMNIPARSE_OCR_DEBUG_DIR=<path>` — dump input/preprocessed/layout PNGs.
///
/// Missing `OMNIPARSE_OCR_PROTOTYPES` falls back to bundled bitmap
/// prototypes. Pointing it at an unreadable or outdated JSON terminates the
/// process with a descriptive message (see commit log for rationale).
pub fn shared_engine() -> &'static OcrEngine {
    use std::sync::OnceLock;
    static ENGINE: OnceLock<OcrEngine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut builder = OcrEngineBuilder::default();

        // Assemble a custom preprocessor from env overrides. We always go
        // through this path so all envs are honored consistently.
        let mut pre_cfg = preprocess::PreprocessConfig::default();
        let bin_window: u32 = std::env::var("OMNIPARSE_OCR_BIN_WINDOW")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(25);
        match std::env::var("OMNIPARSE_OCR_BINARIZE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "sauvola" => {
                pre_cfg.binarize = preprocess::BinarizeMode::Sauvola {
                    window: bin_window,
                    k: 0.2,
                    r: 128.0,
                };
            }
            "adaptive_mean" | "adaptive-mean" => {
                pre_cfg.binarize = preprocess::BinarizeMode::AdaptiveMean {
                    window: bin_window,
                    offset: 10,
                };
            }
            "disabled" | "off" | "none" => {
                pre_cfg.binarize = preprocess::BinarizeMode::Disabled;
            }
            "otsu" | "" => {}
            other => eprintln!(
                "omniparse: unknown OMNIPARSE_OCR_BINARIZE='{}'. Using Otsu.",
                other
            ),
        }
        if std::env::var("OMNIPARSE_OCR_CLAHE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            pre_cfg.clahe = true;
        }
        if let Ok(v) = std::env::var("OMNIPARSE_OCR_TOPHAT") {
            if let Ok(r) = v.parse::<u32>() {
                pre_cfg.tophat_radius = r;
            }
        }
        if let Ok(v) = std::env::var("OMNIPARSE_OCR_DESPECKLE") {
            if let Ok(r) = v.parse::<u32>() {
                pre_cfg.despeckle_radius = r;
            }
        }
        if let Ok(v) = std::env::var("OMNIPARSE_OCR_BILATERAL") {
            if let Ok(r) = v.parse::<u32>() {
                pre_cfg.bilateral_radius = r;
            }
        }
        if let Ok(v) = std::env::var("OMNIPARSE_OCR_UNSHARP") {
            if let Ok(amt) = v.parse::<f32>() {
                pre_cfg.unsharp_amount = amt;
            }
        }
        builder = builder.preprocessor(preprocess::ImageprocPreprocessor::with_config(pre_cfg));

        let k: usize = std::env::var("OMNIPARSE_OCR_K")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let polarity = std::env::var("OMNIPARSE_OCR_POLARITY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("both"))
            .unwrap_or(false);
        let use_kdtree = std::env::var("OMNIPARSE_OCR_KDTREE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let normalize_height = std::env::var("OMNIPARSE_OCR_NORMALIZE_HEIGHT")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|v| *v >= 4);

        let finalize_recognizer = |r: recognize::FeatureRecognizer| {
            let r = r
                .with_k(k)
                .with_both_polarities(polarity)
                .with_normalize_height(normalize_height);
            if use_kdtree {
                r.build_kdtree()
            } else {
                r
            }
        };

        if let Ok(path) = std::env::var("OMNIPARSE_OCR_PROTOTYPES") {
            match prototypes::load_prototypes_json(&path) {
                Ok(protos) => {
                    builder = builder
                        .recognizer(finalize_recognizer(recognize::FeatureRecognizer::new(protos)));
                }
                Err(e) => {
                    // Hard failure — silently using bundled 7×9 bitmap
                    // prototypes against a user who explicitly asked for a
                    // trained set produces uniformly-garbage output that the
                    // caller has no way to distinguish from "OCR ran fine".
                    // Refuse to initialize so the caller sees the problem
                    // immediately. Unset the env var to opt back into the
                    // bundled bitmap set.
                    eprintln!(
                        "omniparse: OMNIPARSE_OCR_PROTOTYPES='{}' could not be loaded: {}.\n\
                         Either fix the path (retrain with `cargo run --features ocr-train \
                         --example train_prototypes -- <font.ttf> <out.json> <px-sizes>`) \
                         or unset the env var to use the bundled bitmap set.",
                        path, e
                    );
                    std::process::exit(1);
                }
            }
        } else if k > 1 || polarity || use_kdtree || normalize_height.is_some() {
            // No user prototype path but some recognizer knob requested.
            builder = builder.recognizer(finalize_recognizer(
                recognize::FeatureRecognizer::with_default_prototypes(),
            ));
        }

        match std::env::var("OMNIPARSE_OCR_LAYOUT")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "swt" => {
                builder = builder.layout(swt::SwtLayoutAnalyzer::new());
            }
            "mser" => {
                builder = builder.layout(mser::MserLayoutAnalyzer::new());
            }
            "cca" | "" => {}
            other => {
                eprintln!(
                    "omniparse: unknown OMNIPARSE_OCR_LAYOUT='{}'. Using default CCA.",
                    other
                );
            }
        }

        builder.build()
    })
}

/// Outcome of an OCR attempt from an image parser.
#[derive(Clone, Debug)]
pub enum OcrAttempt {
    /// OCR feature not opted-in at runtime (`OMNIPARSE_OCR` env var unset).
    Disabled,
    /// OCR ran but nothing survived the min-confidence filter.
    NoTextFound { mean_confidence: f32, regions: usize },
    /// OCR failed before producing a result (decode error, engine error).
    Error(String),
    /// OCR recognized text.
    Recognized { text: String, mean_confidence: f32 },
}

/// Image-parser helper: run OCR on raw image bytes and return a structured
/// outcome. Unlike a bare `Option`, this variant lets callers distinguish
/// "OCR didn't run" from "OCR ran and found nothing" from "OCR errored out".
///
/// Results are cached by image-bytes hash in the shared [`cache`] when
/// caching is enabled (default), so repeated calls with the same bytes pay
/// the OCR cost only once per process.
pub fn run_ocr(bytes: &[u8]) -> OcrAttempt {
    if !runtime_enabled() {
        return OcrAttempt::Disabled;
    }

    let cache = cache::shared_cache();
    let cache_key = cache.as_ref().map(|_| cache::OcrCache::key(bytes));
    if let (Some(c), Some(key)) = (cache, cache_key) {
        if let Some(snap) = c.get(&key) {
            return cache::snapshot_to_attempt(&snap);
        }
    }

    // ML path: route through ocrs + rten when opted in.
    #[cfg(feature = "ocr-ml")]
    {
        if ml::ml_enabled() {
            let attempt = ml::run_ml_ocr(bytes);
            if let (Some(c), Some(key)) = (cache, cache_key) {
                c.put(key, cache::attempt_to_snapshot(&attempt));
            }
            return attempt;
        }
    }

    let attempt = {
        let img = match image::load_from_memory(bytes) {
            Ok(i) => i,
            Err(e) => return OcrAttempt::Error(format!("image decode: {e}")),
        };
        let out = match shared_engine().recognize(img) {
            Ok(o) => o,
            Err(e) => return OcrAttempt::Error(format!("engine: {e}")),
        };
        if out.text.trim().is_empty() {
            OcrAttempt::NoTextFound {
                mean_confidence: out.mean_confidence,
                regions: out.lines.len(),
            }
        } else {
            OcrAttempt::Recognized {
                text: out.text,
                mean_confidence: out.mean_confidence,
            }
        }
    };

    if let (Some(c), Some(key)) = (cache, cache_key) {
        c.put(key, cache::attempt_to_snapshot(&attempt));
    }

    attempt
}

/// Back-compat shim around `run_ocr` — returns text + confidence only on
/// successful recognition.
pub fn maybe_ocr(bytes: &[u8]) -> Option<(String, f32)> {
    match run_ocr(bytes) {
        OcrAttempt::Recognized { text, mean_confidence } => Some((text, mean_confidence)),
        _ => None,
    }
}
