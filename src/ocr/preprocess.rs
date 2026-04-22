//! Stage 1 of the OCR pipeline: clean raster input before recognition.
//!
//! Operations:
//! - grayscale conversion
//! - Otsu binarization (adaptive threshold)
//! - median-filter despeckle (removes isolated dark pixels)
//! - optional deskew (Hough-line angle detection + affine rotation)
//!
//! The defaults are tuned for scanned printed text. For clean screenshots,
//! skip preprocessing entirely and pass the raw `DynamicImage` to the layout
//! stage.

use crate::ocr::error::OcrResult;
use image::{DynamicImage, GrayImage, Luma};

/// Contract every preprocessor must satisfy.
pub trait Preprocessor: Send + Sync {
    fn process(&self, img: DynamicImage) -> OcrResult<GrayImage>;
}

/// Binarization algorithm choice.
///
/// - `Otsu`: global threshold from image histogram. Fast, works on uniform
///   lighting. Fails on gradients and shadows.
/// - `Sauvola`: window-local threshold per pixel. Robust to non-uniform
///   lighting. Slower. Typical `k` ≈ 0.2, `window` ≈ 25.
/// - `AdaptiveMean`: window-local mean minus a small offset. Cheaper than
///   Sauvola, similar behavior on many inputs.
#[derive(Clone, Debug)]
pub enum BinarizeMode {
    Otsu,
    Sauvola { window: u32, k: f32, r: f32 },
    AdaptiveMean { window: u32, offset: i32 },
    Disabled,
}

impl Default for BinarizeMode {
    fn default() -> Self {
        BinarizeMode::Otsu
    }
}

/// Preprocessor options.
#[derive(Clone, Debug)]
pub struct PreprocessConfig {
    /// Binarization mode. Use [`BinarizeMode::Sauvola`] for scans/screenshots
    /// with uneven lighting.
    pub binarize: BinarizeMode,
    /// Median filter radius (0 disables). Small values (1) are plenty for scans.
    pub despeckle_radius: u32,
    /// Attempt Hough-line-based deskew (small angles).
    pub deskew: bool,
    /// Reject deskew rotations smaller than this (radians) to avoid noise.
    pub deskew_min_radians: f32,
    /// Contrast-limited adaptive histogram equalization. Applied before
    /// binarization.
    pub clahe: bool,
    /// CLAHE tile count per axis (8 = 8×8 grid). Ignored when `clahe = false`.
    pub clahe_grid: u32,
    /// CLAHE contrast clip limit (higher = more contrast, noisier).
    pub clahe_clip: f32,
    /// Morphological top-hat background subtraction radius (0 disables).
    pub tophat_radius: u32,
    /// Bilateral filter radius for edge-preserving denoise (0 disables).
    pub bilateral_radius: u32,
    /// Bilateral filter spatial σ (larger = more blur).
    pub bilateral_sigma_spatial: f32,
    /// Bilateral filter range σ (larger = crosses more edges).
    pub bilateral_sigma_range: f32,
    /// Unsharp mask strength (0 disables). Typical 0.5..=2.0.
    pub unsharp_amount: f32,
    /// Unsharp mask radius (Gaussian blur radius, pixels).
    pub unsharp_radius: u32,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            binarize: BinarizeMode::Otsu,
            despeckle_radius: 1,
            deskew: true,
            deskew_min_radians: 0.005,
            clahe: false,
            clahe_grid: 8,
            clahe_clip: 2.0,
            tophat_radius: 0,
            bilateral_radius: 0,
            bilateral_sigma_spatial: 3.0,
            bilateral_sigma_range: 25.0,
            unsharp_amount: 0.0,
            unsharp_radius: 2,
        }
    }
}

/// Pure-Rust preprocessor built on the `imageproc` crate.
pub struct ImageprocPreprocessor {
    cfg: PreprocessConfig,
}

impl ImageprocPreprocessor {
    pub fn new() -> Self {
        Self {
            cfg: PreprocessConfig::default(),
        }
    }

    pub fn with_config(cfg: PreprocessConfig) -> Self {
        Self { cfg }
    }
}

impl Default for ImageprocPreprocessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Preprocessor for ImageprocPreprocessor {
    fn process(&self, img: DynamicImage) -> OcrResult<GrayImage> {
        let mut gray = img.into_luma8();

        if self.cfg.clahe {
            gray = clahe(&gray, self.cfg.clahe_grid, self.cfg.clahe_clip);
        }

        if self.cfg.bilateral_radius > 0 {
            gray = bilateral_filter(
                &gray,
                self.cfg.bilateral_radius,
                self.cfg.bilateral_sigma_spatial,
                self.cfg.bilateral_sigma_range,
            );
        }

        if self.cfg.unsharp_amount > 0.0 {
            gray = unsharp_mask(&gray, self.cfg.unsharp_radius, self.cfg.unsharp_amount);
        }

        if self.cfg.tophat_radius > 0 {
            gray = tophat(&gray, self.cfg.tophat_radius);
        }

        if self.cfg.despeckle_radius > 0 {
            gray = imageproc::filter::median_filter(
                &gray,
                self.cfg.despeckle_radius,
                self.cfg.despeckle_radius,
            );
        }

        gray = match &self.cfg.binarize {
            BinarizeMode::Otsu => {
                let t = imageproc::contrast::otsu_level(&gray);
                imageproc::contrast::threshold(&gray, t)
            }
            BinarizeMode::Sauvola { window, k, r } => sauvola(&gray, *window, *k, *r),
            BinarizeMode::AdaptiveMean { window, offset } => {
                adaptive_mean(&gray, *window, *offset)
            }
            BinarizeMode::Disabled => gray,
        };

        if self.cfg.deskew {
            if let Some(angle) = estimate_skew(&gray) {
                if angle.abs() > self.cfg.deskew_min_radians {
                    gray = rotate_gray(&gray, -angle)?;
                }
            }
        }

        Ok(gray)
    }
}

/// Sauvola adaptive thresholding.
///
/// For each pixel, compute the local mean `m` and standard deviation `s` over
/// a `window × window` neighborhood. Threshold t = m · (1 + k · (s / r - 1))
/// where `r` is the dynamic range (128 for 8-bit images) and `k` tunes how
/// aggressively dark regions are segmented (typical 0.2). Pixels below t
/// become ink; others become background.
pub fn sauvola(img: &GrayImage, window: u32, k: f32, r: f32) -> GrayImage {
    let (w, h) = img.dimensions();
    let half = (window / 2) as i32;
    let (ii, ii2) = integral_images(img);
    let iw = (w + 1) as usize;

    let mut out = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let x0 = (x as i32 - half).max(0) as u32;
            let y0 = (y as i32 - half).max(0) as u32;
            let x1 = ((x as i32 + half + 1) as u32).min(w);
            let y1 = ((y as i32 + half + 1) as u32).min(h);
            let area = ((x1 - x0) * (y1 - y0)) as f32;
            if area == 0.0 {
                continue;
            }
            let sum = rect_sum(&ii, iw, x0, y0, x1, y1);
            let sum_sq = rect_sum(&ii2, iw, x0, y0, x1, y1);
            let mean = sum as f32 / area;
            let var = (sum_sq as f32 / area) - mean * mean;
            let std = var.max(0.0).sqrt();
            let threshold = mean * (1.0 + k * (std / r - 1.0));
            let px = img.get_pixel(x, y)[0] as f32;
            let val = if px < threshold { 0u8 } else { 255u8 };
            out.put_pixel(x, y, Luma([val]));
        }
    }
    out
}

/// Adaptive mean thresholding — cheaper than Sauvola, similar behavior.
pub fn adaptive_mean(img: &GrayImage, window: u32, offset: i32) -> GrayImage {
    let (w, h) = img.dimensions();
    let half = (window / 2) as i32;
    let (ii, _) = integral_images(img);
    let iw = (w + 1) as usize;

    let mut out = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let x0 = (x as i32 - half).max(0) as u32;
            let y0 = (y as i32 - half).max(0) as u32;
            let x1 = ((x as i32 + half + 1) as u32).min(w);
            let y1 = ((y as i32 + half + 1) as u32).min(h);
            let area = ((x1 - x0) * (y1 - y0)) as i64;
            if area == 0 {
                continue;
            }
            let sum = rect_sum(&ii, iw, x0, y0, x1, y1) as i64;
            let mean = sum / area;
            let threshold = (mean - offset as i64).clamp(0, 255);
            let px = img.get_pixel(x, y)[0] as i64;
            let val = if px < threshold { 0u8 } else { 255u8 };
            out.put_pixel(x, y, Luma([val]));
        }
    }
    out
}

/// Morphological top-hat: `img - opening(img)`. Removes slow-varying
/// background, preserves narrow dark strokes.
pub fn tophat(img: &GrayImage, radius: u32) -> GrayImage {
    // Erode then dilate = opening. Use imageproc's morphology on a binary
    // mask won't work here since we want grayscale. Approximate via median
    // min/max filters which imageproc doesn't ship; implement naive boxes.
    let eroded = morphological_erode(img, radius);
    let opened = morphological_dilate(&eroded, radius);

    let (w, h) = img.dimensions();
    let mut out = GrayImage::new(w, h);
    for (x, y, px) in img.enumerate_pixels() {
        let bg = opened.get_pixel(x, y)[0] as i32;
        let fg = px[0] as i32;
        // Bright text on dark bg: top-hat. Dark text on bright bg: bottom-hat.
        // We want dark strokes highlighted, so use `img - opening` on the
        // inverted image (equivalent to bottom-hat).
        let val = (fg - bg).clamp(0, 255) as u8;
        out.put_pixel(x, y, Luma([255u8.saturating_sub(val)]));
        let _ = (w, h);
    }
    out
}

fn morphological_erode(img: &GrayImage, radius: u32) -> GrayImage {
    box_filter(img, radius, |a, b| a.min(b), 255u8)
}
fn morphological_dilate(img: &GrayImage, radius: u32) -> GrayImage {
    box_filter(img, radius, |a, b| a.max(b), 0u8)
}

fn box_filter(
    img: &GrayImage,
    radius: u32,
    combine: fn(u8, u8) -> u8,
    init: u8,
) -> GrayImage {
    let (w, h) = img.dimensions();
    let r = radius as i32;
    let mut out = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let mut acc = init;
            let x0 = (x as i32 - r).max(0) as u32;
            let y0 = (y as i32 - r).max(0) as u32;
            let x1 = ((x as i32 + r + 1) as u32).min(w);
            let y1 = ((y as i32 + r + 1) as u32).min(h);
            for yy in y0..y1 {
                for xx in x0..x1 {
                    acc = combine(acc, img.get_pixel(xx, yy)[0]);
                }
            }
            out.put_pixel(x, y, Luma([acc]));
        }
    }
    out
}

/// Contrast-limited adaptive histogram equalization.
///
/// Splits the image into a `grid × grid` tile grid, computes a clipped-
/// histogram equalization mapping per tile, and bilinearly interpolates
/// between tile centers at each output pixel to avoid blocky artifacts.
pub fn clahe(img: &GrayImage, grid: u32, clip_limit: f32) -> GrayImage {
    let (w, h) = img.dimensions();
    let grid = grid.max(2);
    let tile_w = ((w + grid - 1) / grid).max(1);
    let tile_h = ((h + grid - 1) / grid).max(1);

    let mut maps: Vec<[u8; 256]> = Vec::with_capacity((grid * grid) as usize);
    for ty in 0..grid {
        for tx in 0..grid {
            let x0 = tx * tile_w;
            let y0 = ty * tile_h;
            let x1 = (x0 + tile_w).min(w);
            let y1 = (y0 + tile_h).min(h);
            let mut hist = [0u32; 256];
            for yy in y0..y1 {
                for xx in x0..x1 {
                    hist[img.get_pixel(xx, yy)[0] as usize] += 1;
                }
            }
            let pixel_count = ((x1 - x0) * (y1 - y0)).max(1) as f32;
            let clip = (clip_limit * pixel_count / 256.0) as u32;
            let mut excess: u32 = 0;
            for c in hist.iter_mut() {
                if *c > clip {
                    excess += *c - clip;
                    *c = clip;
                }
            }
            let redistribute = excess / 256;
            let leftover = excess % 256;
            for (i, c) in hist.iter_mut().enumerate() {
                *c += redistribute;
                if (i as u32) < leftover {
                    *c += 1;
                }
            }
            // CDF → lookup table
            let mut cdf: u32 = 0;
            let total = pixel_count as u32;
            let mut map = [0u8; 256];
            for i in 0..256 {
                cdf += hist[i];
                map[i] = ((cdf as f32 / total as f32) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
            maps.push(map);
        }
    }

    let mut out = GrayImage::new(w, h);
    let map_at = |gx: i32, gy: i32| -> &[u8; 256] {
        let gx = gx.clamp(0, grid as i32 - 1) as u32;
        let gy = gy.clamp(0, grid as i32 - 1) as u32;
        &maps[(gy * grid + gx) as usize]
    };

    for y in 0..h {
        for x in 0..w {
            let val = img.get_pixel(x, y)[0] as usize;
            // Fractional tile coordinates.
            let tx = (x as f32 / tile_w as f32) - 0.5;
            let ty = (y as f32 / tile_h as f32) - 0.5;
            let tx0 = tx.floor() as i32;
            let ty0 = ty.floor() as i32;
            let fx = tx - tx0 as f32;
            let fy = ty - ty0 as f32;
            let v00 = map_at(tx0, ty0)[val] as f32;
            let v10 = map_at(tx0 + 1, ty0)[val] as f32;
            let v01 = map_at(tx0, ty0 + 1)[val] as f32;
            let v11 = map_at(tx0 + 1, ty0 + 1)[val] as f32;
            let a = v00 * (1.0 - fx) + v10 * fx;
            let b = v01 * (1.0 - fx) + v11 * fx;
            let v = a * (1.0 - fy) + b * fy;
            out.put_pixel(x, y, Luma([v.round().clamp(0.0, 255.0) as u8]));
        }
    }
    out
}

/// Bilateral filter — edge-preserving smoother. Each output pixel is a
/// weighted average of its neighbors, where weights combine a spatial
/// Gaussian (distance) with a range Gaussian (intensity difference). Strokes
/// stay sharp while noise inside flat regions gets averaged out.
pub fn bilateral_filter(
    img: &GrayImage,
    radius: u32,
    sigma_spatial: f32,
    sigma_range: f32,
) -> GrayImage {
    let (w, h) = img.dimensions();
    let r = radius as i32;
    let spatial_denom = 2.0 * sigma_spatial * sigma_spatial;
    let range_denom = 2.0 * sigma_range * sigma_range;

    let mut out = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let center = img.get_pixel(x, y)[0] as f32;
            let mut weight_sum = 0.0f32;
            let mut value_sum = 0.0f32;
            for dy in -r..=r {
                for dx in -r..=r {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let neighbor = img.get_pixel(nx as u32, ny as u32)[0] as f32;
                    let spatial_sq = (dx * dx + dy * dy) as f32;
                    let range_sq = (neighbor - center).powi(2);
                    let weight = (-spatial_sq / spatial_denom - range_sq / range_denom).exp();
                    weight_sum += weight;
                    value_sum += weight * neighbor;
                }
            }
            let val = if weight_sum > 0.0 {
                (value_sum / weight_sum).clamp(0.0, 255.0) as u8
            } else {
                center as u8
            };
            out.put_pixel(x, y, Luma([val]));
        }
    }
    out
}

/// Unsharp mask: `output = original + amount × (original − blur)`.
/// Sharpens edges by amplifying high-frequency content. `radius` controls
/// the blur kernel size.
pub fn unsharp_mask(img: &GrayImage, radius: u32, amount: f32) -> GrayImage {
    let blurred = box_blur(img, radius);
    let (w, h) = img.dimensions();
    let mut out = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let orig = img.get_pixel(x, y)[0] as f32;
            let blur = blurred.get_pixel(x, y)[0] as f32;
            let sharpened = orig + amount * (orig - blur);
            out.put_pixel(x, y, Luma([sharpened.clamp(0.0, 255.0) as u8]));
        }
    }
    out
}

fn box_blur(img: &GrayImage, radius: u32) -> GrayImage {
    let (w, h) = img.dimensions();
    let r = radius as i32;
    let mut out = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let mut sum: u32 = 0;
            let mut count: u32 = 0;
            for dy in -r..=r {
                for dx in -r..=r {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    sum += img.get_pixel(nx as u32, ny as u32)[0] as u32;
                    count += 1;
                }
            }
            let v = if count > 0 { (sum / count) as u8 } else { 0 };
            out.put_pixel(x, y, Luma([v]));
        }
    }
    out
}

fn integral_images(img: &GrayImage) -> (Vec<u64>, Vec<u64>) {
    let (w, h) = img.dimensions();
    let iw = w as usize + 1;
    let ih = h as usize + 1;
    let mut ii = vec![0u64; iw * ih];
    let mut ii2 = vec![0u64; iw * ih];
    for y in 0..h as usize {
        let mut row_sum: u64 = 0;
        let mut row_sum_sq: u64 = 0;
        for x in 0..w as usize {
            let px = img.get_pixel(x as u32, y as u32)[0] as u64;
            row_sum += px;
            row_sum_sq += px * px;
            ii[(y + 1) * iw + x + 1] = ii[y * iw + x + 1] + row_sum;
            ii2[(y + 1) * iw + x + 1] = ii2[y * iw + x + 1] + row_sum_sq;
        }
    }
    (ii, ii2)
}

fn rect_sum(ii: &[u64], iw: usize, x0: u32, y0: u32, x1: u32, y1: u32) -> u64 {
    let (x0, y0, x1, y1) = (x0 as usize, y0 as usize, x1 as usize, y1 as usize);
    let a = ii[y1 * iw + x1];
    let b = ii[y0 * iw + x1];
    let c = ii[y1 * iw + x0];
    let d = ii[y0 * iw + x0];
    a + d - b - c
}

/// Estimate skew angle in radians from dominant near-horizontal Hough lines.
///
/// Returns `None` when no reliable near-horizontal structure is detected, which
/// lets the caller skip rotation rather than apply a noisy angle.
fn estimate_skew(gray: &GrayImage) -> Option<f32> {
    use imageproc::hough::{detect_lines, LineDetectionOptions};

    // Work on a binarized copy so Hough sees clean edges.
    let threshold = imageproc::contrast::otsu_level(gray);
    let bin = imageproc::contrast::threshold(gray, threshold);

    // Invert so text pixels (which become background after threshold) stand out as edges.
    let edges = imageproc::edges::canny(&bin, 50.0, 100.0);

    let options = LineDetectionOptions {
        vote_threshold: 80,
        suppression_radius: 8,
    };
    let lines = detect_lines(&edges, options);
    if lines.is_empty() {
        return None;
    }

    // Collect angles close to horizontal (theta ≈ 90°) into radians relative to horizontal.
    let mut deltas: Vec<f32> = lines
        .iter()
        .filter_map(|l| {
            let theta = l.angle_in_degrees as f32;
            let delta = (theta - 90.0).abs();
            if delta < 15.0 {
                // Convert "deviation from horizontal" in degrees to radians.
                Some((theta - 90.0).to_radians())
            } else {
                None
            }
        })
        .collect();
    if deltas.is_empty() {
        return None;
    }
    deltas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(deltas[deltas.len() / 2])
}

fn rotate_gray(gray: &GrayImage, radians: f32) -> OcrResult<GrayImage> {
    use imageproc::geometric_transformations::{rotate_about_center, Interpolation};
    let white = Luma([255u8]);
    Ok(rotate_about_center(
        gray,
        radians,
        Interpolation::Bilinear,
        white,
    ))
}
