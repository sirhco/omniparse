//! Maximally Stable Extremal Regions (Matas et al., 2002).
//!
//! Classical blob detector — finds image regions whose connected-component
//! size is stable across a range of intensity thresholds. Text glyphs
//! naturally produce MSERs because their stroke pixels form stable regions
//! over a wide threshold window between the paper and ink intensities.
//!
//! Strategy for this implementation:
//! 1. For every intensity threshold `t` in `[delta, 255-delta]` stepped by
//!    `delta`, compute the set of connected components of pixels ≤ t
//!    (dark-on-light) or ≥ t (light-on-dark).
//! 2. Track how component bounding-box area changes across thresholds.
//! 3. A component is *stable* if its area change `|A(t+δ) − A(t−δ)| / A(t)`
//!    stays below `max_variation` over a span of thresholds.
//!
//! This is an approximation of the original union-find-based MSER — it runs
//! in O(T · W · H) where T is the number of thresholds. For OCR input sizes
//! (< 2000 px wide) and a coarse `delta=8`, that's fine.

use crate::ocr::error::OcrResult;
use crate::ocr::layout::{LayoutAnalyzer, TextRegion};
use image::{GrayImage, Luma};

#[derive(Clone, Debug)]
pub struct MserConfig {
    /// Threshold step size. Smaller = more thresholds = slower but more
    /// regions picked up. 8 is a reasonable default for 8-bit images.
    pub delta: u32,
    /// Maximum allowed relative area variation across `delta` for a region
    /// to be considered stable. Typical values: 0.25 (strict) – 0.6 (loose).
    pub max_variation: f32,
    /// Minimum bounding-box area (px²).
    pub min_area: u32,
    /// Maximum bounding-box area (px²).
    pub max_area: u32,
    /// Aspect ratio bounds (w/h).
    pub aspect_min: f32,
    pub aspect_max: f32,
    /// Run dark-on-light pass.
    pub dark_on_light: bool,
    /// Run light-on-dark pass.
    pub light_on_dark: bool,
}

impl Default for MserConfig {
    fn default() -> Self {
        Self {
            delta: 8,
            max_variation: 0.5,
            min_area: 60,
            max_area: 200_000,
            aspect_min: 0.1,
            aspect_max: 10.0,
            dark_on_light: true,
            light_on_dark: true,
        }
    }
}

pub struct MserLayoutAnalyzer {
    pub cfg: MserConfig,
}

impl MserLayoutAnalyzer {
    pub fn new() -> Self {
        Self {
            cfg: MserConfig::default(),
        }
    }
    pub fn with_config(cfg: MserConfig) -> Self {
        Self { cfg }
    }
}

impl Default for MserLayoutAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutAnalyzer for MserLayoutAnalyzer {
    fn detect_regions(&self, img: &GrayImage) -> OcrResult<Vec<TextRegion>> {
        let mut regions = Vec::new();
        if self.cfg.dark_on_light {
            regions.extend(mser_pass(img, &self.cfg, false));
        }
        if self.cfg.light_on_dark {
            regions.extend(mser_pass(img, &self.cfg, true));
        }
        // Deduplicate overlapping regions via NMS.
        regions = crate::ocr::layout::nms_regions(regions, 0.7);
        Ok(regions)
    }
}

fn mser_pass(img: &GrayImage, cfg: &MserConfig, invert: bool) -> Vec<TextRegion> {
    let (w, h) = img.dimensions();
    let working = if invert {
        invert_image(img)
    } else {
        img.clone()
    };

    // For each threshold t, compute bounding-box areas of every labeled
    // connected component. We then find, per-component, stable thresholds.
    // To keep memory manageable we hash component signatures — a "component"
    // across thresholds is identified by the pixel at its darkest point.

    #[derive(Clone, Default)]
    struct Entry {
        bboxes: Vec<(u32, u32, u32, u32, u32)>, // (threshold, x0, y0, x1, y1)
    }
    let mut table: std::collections::HashMap<(u32, u32), Entry> =
        std::collections::HashMap::new();

    let mut t = cfg.delta;
    while t <= 255u32.saturating_sub(cfg.delta) {
        let threshold = t as u8;
        // Mask pixels ≤ threshold.
        let mut mask = GrayImage::new(w, h);
        for (x, y, px) in working.enumerate_pixels() {
            let v = if px[0] <= threshold { 255u8 } else { 0u8 };
            mask.put_pixel(x, y, Luma([v]));
        }
        let labelled = imageproc::region_labelling::connected_components(
            &mask,
            imageproc::region_labelling::Connectivity::Eight,
            Luma([0u8]),
        );

        // Per-label bounding boxes + "seed" pixel = first encountered.
        #[derive(Default)]
        struct Box {
            x0: u32,
            y0: u32,
            x1: u32,
            y1: u32,
            seed: (u32, u32),
            initialized: bool,
        }
        let mut boxes: std::collections::HashMap<u32, Box> = std::collections::HashMap::new();
        for (x, y, px) in labelled.enumerate_pixels() {
            let label = px[0];
            if label == 0 {
                continue;
            }
            let b = boxes.entry(label).or_default();
            if !b.initialized {
                b.seed = (x, y);
                b.x0 = x;
                b.y0 = y;
                b.x1 = x;
                b.y1 = y;
                b.initialized = true;
            } else {
                b.x0 = b.x0.min(x);
                b.y0 = b.y0.min(y);
                b.x1 = b.x1.max(x);
                b.y1 = b.y1.max(y);
            }
        }

        for b in boxes.values() {
            // Skip components touching every edge — those are almost
            // certainly the outer background.
            if b.x0 == 0 && b.y0 == 0 && b.x1 == w - 1 && b.y1 == h - 1 {
                continue;
            }
            let entry = table.entry(b.seed).or_default();
            entry
                .bboxes
                .push((t, b.x0, b.y0, b.x1, b.y1));
        }

        t += cfg.delta;
    }

    // For each tracked component (indexed by seed), find the most stable
    // bounding box — the one whose neighbors at ±δ in threshold have the
    // smallest area variation.
    let mut out = Vec::new();
    for entry in table.values() {
        if entry.bboxes.len() < 3 {
            continue;
        }
        let mut best_variation = f32::INFINITY;
        let mut best_box: Option<(u32, u32, u32, u32)> = None;
        for i in 1..entry.bboxes.len() - 1 {
            let (_, _x0a, _y0a, _x1a, _y1a) = (
                entry.bboxes[i - 1].0,
                entry.bboxes[i - 1].1,
                entry.bboxes[i - 1].2,
                entry.bboxes[i - 1].3,
                entry.bboxes[i - 1].4,
            );
            let area_prev = area_of(&entry.bboxes[i - 1]);
            let area_cur = area_of(&entry.bboxes[i]);
            let area_next = area_of(&entry.bboxes[i + 1]);
            if area_cur == 0 {
                continue;
            }
            let variation = ((area_next as f32 - area_prev as f32).abs()) / area_cur as f32;
            if variation < best_variation {
                best_variation = variation;
                best_box = Some((
                    entry.bboxes[i].1,
                    entry.bboxes[i].2,
                    entry.bboxes[i].3,
                    entry.bboxes[i].4,
                ));
            }
        }
        let Some((x0, y0, x1, y1)) = best_box else {
            continue;
        };
        if best_variation > cfg.max_variation {
            continue;
        }
        let width = x1 - x0 + 1;
        let height = y1 - y0 + 1;
        let area = width * height;
        if area < cfg.min_area || area > cfg.max_area {
            continue;
        }
        let aspect = width as f32 / height.max(1) as f32;
        if aspect < cfg.aspect_min || aspect > cfg.aspect_max {
            continue;
        }
        out.push(TextRegion {
            x: x0,
            y: y0,
            width,
            height,
        });
    }
    out
}

fn area_of(bbox: &(u32, u32, u32, u32, u32)) -> u32 {
    let (_t, x0, y0, x1, y1) = *bbox;
    (x1 - x0 + 1) * (y1 - y0 + 1)
}

fn invert_image(img: &GrayImage) -> GrayImage {
    let (w, h) = img.dimensions();
    let mut out = GrayImage::new(w, h);
    for (x, y, px) in img.enumerate_pixels() {
        out.put_pixel(x, y, Luma([255 - px[0]]));
    }
    out
}
