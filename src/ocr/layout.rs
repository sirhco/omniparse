//! Stage 2 of the OCR pipeline: find text regions and per-character glyph boxes.

use crate::ocr::error::OcrResult;
use image::GrayImage;

/// A rectangular region in image coordinates. Used both for multi-glyph text
/// lines and for single-character bounding boxes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl TextRegion {
    pub fn area(&self) -> u32 {
        self.width.saturating_mul(self.height)
    }
}

pub trait LayoutAnalyzer: Send + Sync {
    fn detect_regions(&self, img: &GrayImage) -> OcrResult<Vec<TextRegion>>;
}

/// Group raw `TextRegion`s into text lines by vertical overlap.
///
/// Two regions belong to the same line if their vertical extents overlap by
/// more than 50% of the smaller region's height. Within a line, regions are
/// sorted left-to-right. Lines are returned top-to-bottom. A per-line median
/// height lets downstream stages normalize glyph size before recognition.
pub fn group_regions_into_lines(mut regions: Vec<TextRegion>) -> Vec<Vec<TextRegion>> {
    regions.sort_by_key(|r| (r.y, r.x));
    let mut lines: Vec<Vec<TextRegion>> = Vec::new();
    for r in regions {
        let r_top = r.y;
        let r_bot = r.y + r.height;
        let r_h = r.height.max(1);
        let mut placed = false;
        for line in lines.iter_mut() {
            let (top, bot, h) = line_vspan(line);
            let overlap = r_bot.min(bot).saturating_sub(r_top.max(top));
            let smaller = r_h.min(h.max(1));
            if overlap as f32 / smaller as f32 >= 0.5 {
                line.push(r.clone());
                placed = true;
                break;
            }
        }
        if !placed {
            lines.push(vec![r]);
        }
    }
    for line in lines.iter_mut() {
        line.sort_by_key(|g| g.x);
    }
    lines.sort_by_key(|line| line.iter().map(|g| g.y).min().unwrap_or(0));
    lines
}

fn line_vspan(line: &[TextRegion]) -> (u32, u32, u32) {
    let top = line.iter().map(|r| r.y).min().unwrap_or(0);
    let bot = line.iter().map(|r| r.y + r.height).max().unwrap_or(0);
    (top, bot, bot.saturating_sub(top))
}

/// Median glyph height in a grouped line. Returns 0 for empty lines.
pub fn line_median_height(line: &[TextRegion]) -> u32 {
    if line.is_empty() {
        return 0;
    }
    let mut hs: Vec<u32> = line.iter().map(|r| r.height).collect();
    hs.sort_unstable();
    hs[hs.len() / 2]
}

/// Intersection-over-union between two rectangles.
pub fn iou(a: &TextRegion, b: &TextRegion) -> f32 {
    let ax1 = a.x + a.width;
    let ay1 = a.y + a.height;
    let bx1 = b.x + b.width;
    let by1 = b.y + b.height;
    let ix0 = a.x.max(b.x);
    let iy0 = a.y.max(b.y);
    let ix1 = ax1.min(bx1);
    let iy1 = ay1.min(by1);
    if ix1 <= ix0 || iy1 <= iy0 {
        return 0.0;
    }
    let inter = ((ix1 - ix0) * (iy1 - iy0)) as f32;
    let union = (a.area() + b.area()) as f32 - inter;
    if union <= 0.0 {
        0.0
    } else {
        inter / union
    }
}

/// Non-max suppression over region candidates. Keeps the larger of each
/// pair with IoU above `iou_threshold`. Simple O(n²) sweep — fine for
/// n ≈ 500 typical region counts.
pub fn nms_regions(regions: Vec<TextRegion>, iou_threshold: f32) -> Vec<TextRegion> {
    let mut sorted = regions;
    sorted.sort_by(|a, b| b.area().cmp(&a.area()));
    let mut kept: Vec<TextRegion> = Vec::with_capacity(sorted.len());
    for r in sorted {
        if kept.iter().any(|k| iou(k, &r) > iou_threshold) {
            continue;
        }
        kept.push(r);
    }
    kept
}

/// Aspect-ratio + absolute-size filter. Drops regions that are too tall-and-
/// narrow, too wide-and-short, or outside a min/max height range. Useful as
/// a post-MSER/SWT cleanup step.
pub fn filter_text_regions(
    regions: Vec<TextRegion>,
    min_aspect: f32,
    max_aspect: f32,
    min_height: u32,
    max_height: u32,
    min_area: u32,
) -> Vec<TextRegion> {
    regions
        .into_iter()
        .filter(|r| {
            let aspect = r.width as f32 / r.height.max(1) as f32;
            aspect >= min_aspect
                && aspect <= max_aspect
                && r.height >= min_height
                && r.height <= max_height
                && r.area() >= min_area
        })
        .collect()
}

/// Merge two region lists with NMS. Use to combine the output of multiple
/// analyzers (CCA + SWT + MSER) into a de-duplicated candidate set.
pub fn merge_regions(a: Vec<TextRegion>, b: Vec<TextRegion>, iou_threshold: f32) -> Vec<TextRegion> {
    let mut combined = a;
    combined.extend(b);
    nms_regions(combined, iou_threshold)
}

/// Keep only regions that look like characters in a text line — each kept
/// region must have at least `min_neighbors` other regions that:
///   - sit on roughly the same baseline (y-center within `y_tolerance × h`)
///   - are within `x_reach × h` horizontal pixels
///   - are within `size_ratio..(1/size_ratio)` of the same height
///
/// Real text produces dense clusters of similar-sized co-linear glyphs.
/// Photographic edges produce isolated blobs of varying heights. This
/// filter rejects the latter without needing semantic information.
pub fn filter_by_neighbor_density(
    regions: Vec<TextRegion>,
    min_neighbors: usize,
    y_tolerance: f32,
    x_reach: f32,
    size_ratio: f32,
) -> Vec<TextRegion> {
    let n = regions.len();
    if n == 0 {
        return regions;
    }
    let mut keep = vec![false; n];
    for i in 0..n {
        let a = &regions[i];
        let a_cy = a.y as f32 + a.height as f32 * 0.5;
        let a_h = a.height.max(1) as f32;
        let mut neighbors = 0usize;
        for j in 0..n {
            if i == j {
                continue;
            }
            let b = &regions[j];
            let b_cy = b.y as f32 + b.height as f32 * 0.5;
            if (b_cy - a_cy).abs() > y_tolerance * a_h {
                continue;
            }
            let b_h = b.height.max(1) as f32;
            let ratio = b_h / a_h;
            if ratio < size_ratio || ratio > 1.0 / size_ratio {
                continue;
            }
            let dx = if b.x > a.x + a.width {
                (b.x - (a.x + a.width)) as f32
            } else if a.x > b.x + b.width {
                (a.x - (b.x + b.width)) as f32
            } else {
                0.0
            };
            if dx > x_reach * a_h {
                continue;
            }
            neighbors += 1;
            if neighbors >= min_neighbors {
                break;
            }
        }
        if neighbors >= min_neighbors {
            keep[i] = true;
        }
    }
    regions
        .into_iter()
        .zip(keep.into_iter())
        .filter_map(|(r, k)| if k { Some(r) } else { None })
        .collect()
}

/// Heuristic text-line rejection. Groups regions into lines (see
/// `group_regions_into_lines`) and keeps only lines that look text-like:
/// - at least 2 regions
/// - glyph-height coefficient of variation ≤ 0.5 (text has uniform heights)
/// - median spacing between glyph centers ≤ median width × 3 (no huge gaps)
///
/// Singleton regions pass through unchanged — too little data to judge.
pub fn filter_text_lines(regions: Vec<TextRegion>) -> Vec<TextRegion> {
    let lines = group_regions_into_lines(regions);
    let mut kept: Vec<TextRegion> = Vec::new();
    for line in lines {
        if line.len() <= 1 {
            kept.extend(line);
            continue;
        }
        let heights: Vec<f32> = line.iter().map(|r| r.height as f32).collect();
        let mean = heights.iter().sum::<f32>() / heights.len() as f32;
        let var = heights.iter().map(|h| (h - mean).powi(2)).sum::<f32>() / heights.len() as f32;
        let std = var.sqrt();
        let cv = if mean > 0.0 { std / mean } else { f32::INFINITY };
        if cv > 0.5 {
            continue;
        }
        // Spacing sanity: median horizontal gap ≤ median width × 3.
        let mut sorted = line.clone();
        sorted.sort_by_key(|r| r.x);
        let gaps: Vec<f32> = sorted
            .windows(2)
            .map(|w| {
                let a = &w[0];
                let b = &w[1];
                (b.x as f32 - (a.x + a.width) as f32).max(0.0)
            })
            .collect();
        let mut widths: Vec<f32> = line.iter().map(|r| r.width as f32).collect();
        widths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_w = widths[widths.len() / 2];
        let mut sorted_gaps = gaps.clone();
        sorted_gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_gap = sorted_gaps
            .get(sorted_gaps.len() / 2)
            .copied()
            .unwrap_or(0.0);
        if median_gap > median_w * 3.0 {
            continue;
        }
        kept.extend(line);
    }
    kept
}

/// Keep only regions whose ink pixels have a coefficient-of-variation in
/// stroke width below `max_cv`. True text glyphs have nearly constant
/// stroke width; photographic edges do not. Operates on an already-
/// binarized grayscale image (ink < 128, background ≥ 128).
///
/// Stroke width per pixel is approximated by the distance to the nearest
/// background pixel — the classical distance transform. A fast L1 two-pass
/// variant is used here (Rosenfeld & Pfaltz, 1966).
pub fn filter_by_stroke_width_constancy(
    img: &image::GrayImage,
    regions: Vec<TextRegion>,
    max_cv: f32,
) -> Vec<TextRegion> {
    let dt = l1_distance_transform(img);
    let (w, _h) = img.dimensions();
    regions
        .into_iter()
        .filter(|r| {
            let mut count = 0u32;
            let mut sum = 0.0f64;
            let mut sum_sq = 0.0f64;
            for y in r.y..(r.y + r.height).min(img.height()) {
                for x in r.x..(r.x + r.width).min(img.width()) {
                    let d = dt[(y * w + x) as usize];
                    if d > 0 {
                        count += 1;
                        sum += d as f64;
                        sum_sq += (d as f64) * (d as f64);
                    }
                }
            }
            if count < 10 {
                return false; // too few ink pixels to judge
            }
            let mean = sum / count as f64;
            let var = (sum_sq / count as f64) - mean * mean;
            let std = var.max(0.0).sqrt();
            let cv = if mean > 0.0 { (std / mean) as f32 } else { f32::INFINITY };
            cv <= max_cv
        })
        .collect()
}

fn l1_distance_transform(img: &image::GrayImage) -> Vec<u32> {
    let (w, h) = img.dimensions();
    let n = (w * h) as usize;
    // Initialize: 0 for background, ∞ for ink.
    let big = w + h;
    let mut dt = vec![big; n];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if img.get_pixel(x, y)[0] >= 128 {
                dt[i] = 0;
            }
        }
    }
    // Forward pass.
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if y > 0 {
                dt[i] = dt[i].min(dt[i - w as usize] + 1);
            }
            if x > 0 {
                dt[i] = dt[i].min(dt[i - 1] + 1);
            }
        }
    }
    // Backward pass.
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = (y * w + x) as usize;
            if y + 1 < h {
                dt[i] = dt[i].min(dt[i + w as usize] + 1);
            }
            if x + 1 < w {
                dt[i] = dt[i].min(dt[i + 1] + 1);
            }
        }
    }
    dt
}

/// Trivial analyzer — one region covering the entire input. Useful for clean
/// single-line fixtures and smoke tests.
pub struct WholeImageAnalyzer;

impl LayoutAnalyzer for WholeImageAnalyzer {
    fn detect_regions(&self, img: &GrayImage) -> OcrResult<Vec<TextRegion>> {
        Ok(vec![TextRegion {
            x: 0,
            y: 0,
            width: img.width(),
            height: img.height(),
        }])
    }
}

/// Connected-component based analyzer. Each dark component on a light
/// background is treated as a glyph candidate; components are then grouped
/// into lines by y-overlap. The recognizer receives per-glyph regions.
///
/// Expects a binarized image (background = 255, ink = 0). Feed it the output
/// of `ImageprocPreprocessor` configured with `binarize = true`.
pub struct ConnectedComponentAnalyzer {
    /// Reject components smaller than this many pixels on either axis.
    /// Filters out scanner dust without dropping periods.
    pub min_dimension: u32,
    /// Reject components taller than this fraction of image height (likely
    /// borders or rules).
    pub max_height_fraction: f32,
}

impl Default for ConnectedComponentAnalyzer {
    fn default() -> Self {
        Self {
            min_dimension: 2,
            max_height_fraction: 0.9,
        }
    }
}

impl LayoutAnalyzer for ConnectedComponentAnalyzer {
    fn detect_regions(&self, img: &GrayImage) -> OcrResult<Vec<TextRegion>> {
        let (w, h) = img.dimensions();
        let max_height = (h as f32 * self.max_height_fraction) as u32;

        // Label connected components over the "ink" pixels. We treat any pixel
        // below 128 as ink. `regionlabelling` expects a background value, so we
        // first produce a binary ink mask.
        let mut ink_mask: GrayImage = GrayImage::new(w, h);
        for (x, y, px) in img.enumerate_pixels() {
            let ink = if px[0] < 128 { 255 } else { 0 };
            ink_mask.put_pixel(x, y, image::Luma([ink]));
        }

        let labelled = imageproc::region_labelling::connected_components(
            &ink_mask,
            imageproc::region_labelling::Connectivity::Eight,
            image::Luma([0u8]),
        );

        // Walk labelled image, compute bounding box per label.
        let mut boxes: std::collections::HashMap<u32, Bbox> = std::collections::HashMap::new();
        for (x, y, px) in labelled.enumerate_pixels() {
            let label = px[0];
            if label == 0 {
                continue;
            }
            let entry = boxes.entry(label).or_insert_with(|| Bbox::point(x, y));
            entry.extend(x, y);
        }

        let mut regions: Vec<TextRegion> = boxes
            .into_values()
            .map(|b| b.into_region())
            .filter(|r| {
                r.width >= self.min_dimension
                    && r.height >= self.min_dimension
                    && r.height <= max_height
            })
            .collect();

        // Sort reading order: top-to-bottom, then left-to-right. Line grouping
        // is done inline: if two consecutive regions overlap vertically by
        // more than half their height, they belong to the same line.
        regions.sort_by(|a, b| a.y.cmp(&b.y).then_with(|| a.x.cmp(&b.x)));
        Ok(regions)
    }
}

struct Bbox {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

impl Bbox {
    fn point(x: u32, y: u32) -> Self {
        Self {
            x0: x,
            y0: y,
            x1: x,
            y1: y,
        }
    }
    fn extend(&mut self, x: u32, y: u32) {
        if x < self.x0 {
            self.x0 = x;
        }
        if y < self.y0 {
            self.y0 = y;
        }
        if x > self.x1 {
            self.x1 = x;
        }
        if y > self.y1 {
            self.y1 = y;
        }
    }
    fn into_region(self) -> TextRegion {
        TextRegion {
            x: self.x0,
            y: self.y0,
            width: self.x1 - self.x0 + 1,
            height: self.y1 - self.y0 + 1,
        }
    }
}
