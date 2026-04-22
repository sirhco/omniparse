//! Color-based foreground segmentation. Takes a color image, clusters
//! pixel colors via k-means, then returns one grayscale mask per cluster
//! (255 where pixels belong to the cluster, 0 elsewhere).
//!
//! For text-over-image inputs, one of the clusters usually captures the
//! overlay text color. Binarizing the cluster mask instead of the original
//! grayscale gives the OCR pipeline a much cleaner foreground signal than
//! Otsu on the whole photo.

use image::{DynamicImage, GrayImage, Luma, RgbImage};

/// K-means on pixel RGB values. `k` clusters, `max_iter` iterations, sampling
/// at most `sample_cap` pixels to keep large images fast.
pub fn kmeans_pixels(
    img: &DynamicImage,
    k: usize,
    max_iter: usize,
    sample_cap: usize,
) -> Vec<[f32; 3]> {
    let rgb: RgbImage = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let total = (w as usize) * (h as usize);
    let step = (total / sample_cap.max(1)).max(1);

    let mut samples: Vec<[f32; 3]> = Vec::with_capacity(sample_cap);
    for idx in (0..total).step_by(step) {
        let x = (idx % w as usize) as u32;
        let y = (idx / w as usize) as u32;
        let p = rgb.get_pixel(x, y);
        samples.push([p[0] as f32, p[1] as f32, p[2] as f32]);
    }

    // Deterministic init: spread initial centroids along the sampled sequence.
    let mut centroids: Vec<[f32; 3]> = (0..k)
        .map(|i| samples[(i * samples.len()) / k.max(1)])
        .collect();

    for _ in 0..max_iter {
        let mut sums = vec![[0.0f32; 3]; k];
        let mut counts = vec![0u32; k];
        for s in &samples {
            let mut best = 0usize;
            let mut best_d = f32::INFINITY;
            for (i, c) in centroids.iter().enumerate() {
                let d = sq_dist(s, c);
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            sums[best][0] += s[0];
            sums[best][1] += s[1];
            sums[best][2] += s[2];
            counts[best] += 1;
        }
        let mut changed = false;
        for i in 0..k {
            if counts[i] == 0 {
                continue;
            }
            let new_c = [
                sums[i][0] / counts[i] as f32,
                sums[i][1] / counts[i] as f32,
                sums[i][2] / counts[i] as f32,
            ];
            if sq_dist(&new_c, &centroids[i]) > 1e-3 {
                changed = true;
                centroids[i] = new_c;
            }
        }
        if !changed {
            break;
        }
    }

    centroids
}

/// Build a binary mask (ink = 0, bg = 255) selecting pixels closest to
/// `centroid`. Pass the centroid believed to match the text color.
pub fn mask_for_centroid(img: &DynamicImage, centroid: &[f32; 3]) -> GrayImage {
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let mut out = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let p = rgb.get_pixel(x, y);
            let v = [p[0] as f32, p[1] as f32, p[2] as f32];
            out.put_pixel(x, y, Luma([0])); // placeholder set below
            let _ = &v;
        }
    }
    // Two-pass: compute nearest-cluster per pixel by distance to centroid
    // relative to other candidates is done by caller via `mask_text_cluster`.
    // This helper assumes caller already picked the winning centroid.
    for y in 0..h {
        for x in 0..w {
            let p = rgb.get_pixel(x, y);
            let v = [p[0] as f32, p[1] as f32, p[2] as f32];
            let d = sq_dist(&v, centroid);
            // A pixel is "text" if it's closer to the centroid than a fixed
            // tolerance (≈ 40 in Euclidean RGB, so 40² = 1600 squared).
            let is_text = d < 1600.0;
            out.put_pixel(x, y, Luma([if is_text { 0 } else { 255 }]));
        }
    }
    out
}

/// Pick the centroid most likely to be text. Heuristic: the cluster whose
/// population is smallest but non-negligible, because body text usually
/// occupies much less area than the background it sits on. Falls back to the
/// darkest centroid if all populations are similar.
pub fn pick_text_centroid(img: &DynamicImage, centroids: &[[f32; 3]]) -> Option<[f32; 3]> {
    if centroids.is_empty() {
        return None;
    }
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let total = (w * h) as u64;

    let mut counts = vec![0u64; centroids.len()];
    for y in 0..h {
        for x in 0..w {
            let p = rgb.get_pixel(x, y);
            let v = [p[0] as f32, p[1] as f32, p[2] as f32];
            let (best, _) = centroids
                .iter()
                .enumerate()
                .map(|(i, c)| (i, sq_dist(&v, c)))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap();
            counts[best] += 1;
        }
    }

    // Candidates: centroids with population between 0.5% and 30% of the
    // image. Anything bigger is probably background; anything tinier is
    // probably noise.
    let min_pop = total / 200; // 0.5%
    let max_pop = total * 30 / 100; // 30%
    let mut candidates: Vec<(usize, u64)> = counts
        .iter()
        .enumerate()
        .filter(|(_, c)| **c >= min_pop && **c <= max_pop)
        .map(|(i, c)| (i, *c))
        .collect();

    // Among candidates, prefer the darkest (text is usually dark-on-light or
    // light-on-dark; caller handles polarity).
    if !candidates.is_empty() {
        candidates.sort_by(|a, b| {
            let la = luma(&centroids[a.0]);
            let lb = luma(&centroids[b.0]);
            la.partial_cmp(&lb).unwrap()
        });
        return Some(centroids[candidates[0].0]);
    }

    // Fallback: darkest centroid overall.
    let darkest_idx = (0..centroids.len())
        .min_by(|&a, &b| luma(&centroids[a]).partial_cmp(&luma(&centroids[b])).unwrap())
        .unwrap();
    Some(centroids[darkest_idx])
}

/// Full helper: cluster colors, auto-pick text cluster, return binary mask
/// suitable for feeding the OCR pipeline's layout stage.
pub fn segment_text_mask(img: &DynamicImage, k: usize) -> Option<GrayImage> {
    let centroids = kmeans_pixels(img, k.max(2), 10, 20_000);
    let text_centroid = pick_text_centroid(img, &centroids)?;
    Some(mask_for_centroid(img, &text_centroid))
}

fn sq_dist(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

fn luma(c: &[f32; 3]) -> f32 {
    0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
}
