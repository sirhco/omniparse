//! Stroke-Width Transform (Epshtein et al., CVPR 2010).
//!
//! A classical text-detection algorithm that isolates foreground text from
//! photographic backgrounds by exploiting one structural property of written
//! glyphs: **the stroke width is nearly constant within a single character**.
//!
//! Pipeline:
//! 1. Canny edge detection.
//! 2. Sobel gradient direction at each edge pixel.
//! 3. For each edge pixel, cast a ray along ±gradient until another edge
//!    pixel is hit whose gradient is roughly antiparallel. Ray length is a
//!    stroke-width estimate; every pixel along the ray receives that width
//!    (or the minimum of its current value and the new ray).
//! 4. Connected components over the SWT image, grouped by similar widths.
//! 5. Filter components by stroke-width coefficient of variation, aspect
//!    ratio, and absolute size bounds.
//!
//! One forward pass handles dark-on-light text; a second reversed pass
//! captures light-on-dark. This implementation runs both by default.
//!
//! The module exposes [`SwtLayoutAnalyzer`] which can be plugged into
//! [`crate::ocr::OcrEngineBuilder::layout`] as an alternative to
//! [`crate::ocr::layout::ConnectedComponentAnalyzer`]. Use SWT when input
//! images are photographs with text overlays and the default CCA
//! segmentation floods regions with image content.

use crate::ocr::error::OcrResult;
use crate::ocr::layout::{LayoutAnalyzer, TextRegion};
use image::{GrayImage, Luma};

/// Polarity of the text relative to its background.
#[derive(Clone, Copy, Debug)]
pub enum Polarity {
    /// Text darker than background (most common).
    DarkOnLight,
    /// Text lighter than background (e.g. neon, white-on-black UI).
    LightOnDark,
    /// Try both and union the results.
    Both,
}

/// Configuration for the SWT analyzer.
#[derive(Clone, Debug)]
pub struct SwtConfig {
    /// Canny low threshold.
    pub canny_low: f32,
    /// Canny high threshold.
    pub canny_high: f32,
    /// Reject rays longer than this (pixels).
    pub max_stroke_width: u32,
    /// Accept component only if stroke-width CV (std/mean) ≤ this.
    pub max_cv: f32,
    /// Accept component only if its bounding box aspect ratio (w/h) falls
    /// in this range. Latin glyphs are roughly 0.1..=10.0.
    pub aspect_min: f32,
    pub aspect_max: f32,
    /// Minimum and maximum glyph height (px).
    pub height_min: u32,
    pub height_max: u32,
    /// Minimum pixel count per component.
    pub min_area: u32,
    /// Angular tolerance (radians) when matching opposite-gradient edges.
    pub angle_tolerance: f32,
    /// Which polarity to run.
    pub polarity: Polarity,
}

impl Default for SwtConfig {
    fn default() -> Self {
        Self {
            canny_low: 50.0,
            canny_high: 150.0,
            max_stroke_width: 40,
            max_cv: 0.5,
            aspect_min: 0.1,
            aspect_max: 10.0,
            height_min: 8,
            height_max: 400,
            min_area: 20,
            angle_tolerance: std::f32::consts::FRAC_PI_6, // 30°
            polarity: Polarity::Both,
        }
    }
}

pub struct SwtLayoutAnalyzer {
    pub cfg: SwtConfig,
}

impl SwtLayoutAnalyzer {
    pub fn new() -> Self {
        Self {
            cfg: SwtConfig::default(),
        }
    }
    pub fn with_config(cfg: SwtConfig) -> Self {
        Self { cfg }
    }
}

impl Default for SwtLayoutAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutAnalyzer for SwtLayoutAnalyzer {
    fn detect_regions(&self, img: &GrayImage) -> OcrResult<Vec<TextRegion>> {
        let mut regions = Vec::new();
        match self.cfg.polarity {
            Polarity::DarkOnLight => regions.extend(run_pass(img, &self.cfg, false)),
            Polarity::LightOnDark => regions.extend(run_pass(img, &self.cfg, true)),
            Polarity::Both => {
                regions.extend(run_pass(img, &self.cfg, false));
                regions.extend(run_pass(img, &self.cfg, true));
            }
        }
        dedupe(&mut regions);
        Ok(regions)
    }
}

fn run_pass(img: &GrayImage, cfg: &SwtConfig, invert: bool) -> Vec<TextRegion> {
    let (w, h) = img.dimensions();
    let working = if invert { invert_image(img) } else { img.clone() };

    // Sobel gradients (signed i16 outputs).
    let gx = imageproc::gradients::horizontal_sobel(&working);
    let gy = imageproc::gradients::vertical_sobel(&working);
    let edges = imageproc::edges::canny(&working, cfg.canny_low, cfg.canny_high);

    // SWT map: f32 per pixel, initialized to max. Ray lengths overwrite.
    let mut swt: Vec<f32> = vec![f32::INFINITY; (w * h) as usize];

    for y in 0..h {
        for x in 0..w {
            if edges.get_pixel(x, y)[0] == 0 {
                continue;
            }
            let dx = gx.get_pixel(x, y)[0] as f32;
            let dy = gy.get_pixel(x, y)[0] as f32;
            let mag = (dx * dx + dy * dy).sqrt();
            if mag < 1.0 {
                continue;
            }
            let ux = dx / mag;
            let uy = dy / mag;
            // Dark-on-light: gradient points FROM dark (ink) TOWARD light (bg).
            // Cast ray in the direction of decreasing brightness (−gradient).
            if let Some(stroke_len) =
                cast_ray(&edges, &gx, &gy, x as f32, y as f32, -ux, -uy, cfg)
            {
                let steps = stroke_len.ceil() as u32;
                for s in 0..=steps {
                    let t = s as f32;
                    let px = (x as f32 + -ux * t) as i32;
                    let py = (y as f32 + -uy * t) as i32;
                    if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                        break;
                    }
                    let idx = (py as u32 * w + px as u32) as usize;
                    if stroke_len < swt[idx] {
                        swt[idx] = stroke_len;
                    }
                }
            }
        }
    }

    // Turn SWT into a binary mask of pixels with finite stroke widths.
    let mut mask = GrayImage::from_pixel(w, h, Luma([0]));
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if swt[idx].is_finite() && swt[idx] <= cfg.max_stroke_width as f32 {
                mask.put_pixel(x, y, Luma([255]));
            }
        }
    }

    // Connected components on the mask.
    let labelled = imageproc::region_labelling::connected_components(
        &mask,
        imageproc::region_labelling::Connectivity::Eight,
        Luma([0u8]),
    );

    // Per-label stats.
    #[derive(Default)]
    struct Stats {
        min_x: u32,
        min_y: u32,
        max_x: u32,
        max_y: u32,
        count: u32,
        sum: f32,
        sum_sq: f32,
        initialized: bool,
    }
    let mut stats: std::collections::HashMap<u32, Stats> = std::collections::HashMap::new();
    for y in 0..h {
        for x in 0..w {
            let label = labelled.get_pixel(x, y)[0];
            if label == 0 {
                continue;
            }
            let sw = swt[(y * w + x) as usize];
            if !sw.is_finite() {
                continue;
            }
            let s = stats.entry(label).or_default();
            if !s.initialized {
                s.min_x = x;
                s.min_y = y;
                s.max_x = x;
                s.max_y = y;
                s.initialized = true;
            }
            s.min_x = s.min_x.min(x);
            s.min_y = s.min_y.min(y);
            s.max_x = s.max_x.max(x);
            s.max_y = s.max_y.max(y);
            s.count += 1;
            s.sum += sw;
            s.sum_sq += sw * sw;
        }
    }

    let mut out = Vec::new();
    for s in stats.values() {
        if s.count < cfg.min_area {
            continue;
        }
        let bw = s.max_x - s.min_x + 1;
        let bh = s.max_y - s.min_y + 1;
        if bh < cfg.height_min || bh > cfg.height_max {
            continue;
        }
        let aspect = bw as f32 / bh.max(1) as f32;
        if aspect < cfg.aspect_min || aspect > cfg.aspect_max {
            continue;
        }
        let mean = s.sum / s.count as f32;
        let var = (s.sum_sq / s.count as f32) - mean * mean;
        let std = var.max(0.0).sqrt();
        let cv = if mean > 0.0 { std / mean } else { f32::INFINITY };
        if cv > cfg.max_cv {
            continue;
        }
        out.push(TextRegion {
            x: s.min_x,
            y: s.min_y,
            width: bw,
            height: bh,
        });
    }
    out
}

fn cast_ray(
    edges: &GrayImage,
    gx: &image::ImageBuffer<Luma<i16>, Vec<i16>>,
    gy: &image::ImageBuffer<Luma<i16>, Vec<i16>>,
    start_x: f32,
    start_y: f32,
    ux: f32,
    uy: f32,
    cfg: &SwtConfig,
) -> Option<f32> {
    let (w, h) = edges.dimensions();
    let mut t = 1.0f32;
    while t <= cfg.max_stroke_width as f32 {
        let px = (start_x + ux * t).round() as i32;
        let py = (start_y + uy * t).round() as i32;
        if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
            return None;
        }
        let xp = px as u32;
        let yp = py as u32;
        if xp as f32 == start_x && yp as f32 == start_y {
            t += 1.0;
            continue;
        }
        if edges.get_pixel(xp, yp)[0] != 0 {
            // Opposite-edge test: gradient at this edge pixel should roughly
            // antiparallel the ray direction. Using dot-product on normalized
            // vectors: want dot ≈ +1 between (−grad_here_unit) and (ux,uy)
            // (because the stroke's other side has opposite outward gradient).
            let dx = gx.get_pixel(xp, yp)[0] as f32;
            let dy = gy.get_pixel(xp, yp)[0] as f32;
            let mag = (dx * dx + dy * dy).sqrt();
            if mag < 1.0 {
                t += 1.0;
                continue;
            }
            let nx = dx / mag;
            let ny = dy / mag;
            // Expect gradient at the far edge to point roughly against ray
            // direction (i.e. the dot product of +grad_far and +ray should be
            // near −1 for dark-on-light, because the gradient flips sign
            // across the stroke).
            // Accept strongly-aligned (parallel or antiparallel) gradients.
            // `imageproc`'s Sobel sign convention doesn't match the text-book
            // orientation consistently, so we allow either polarity and rely
            // on downstream stroke-width CV filtering to reject noise.
            let dot = nx * ux + ny * uy;
            if dot.abs() >= cfg.angle_tolerance.cos() {
                return Some(t);
            }
        }
        t += 1.0;
    }
    None
}

fn invert_image(img: &GrayImage) -> GrayImage {
    let (w, h) = img.dimensions();
    let mut out = GrayImage::new(w, h);
    for (x, y, px) in img.enumerate_pixels() {
        out.put_pixel(x, y, Luma([255 - px[0]]));
    }
    out
}

fn dedupe(regions: &mut Vec<TextRegion>) {
    regions.sort_by_key(|r| (r.y, r.x, r.width, r.height));
    regions.dedup();
}
