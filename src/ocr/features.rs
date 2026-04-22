//! Glyph feature extraction for the classical recognizer.
//!
//! Given a binarized glyph image (ink = 0, background = 255), produce a fixed-
//! length feature vector suitable for nearest-neighbor classification.
//!
//! Features emitted (total 55 floats):
//! - aspect_ratio (width / height)
//! - fill_ratio  (ink_pixels / total)
//! - 9 zone densities (image split into 3×3 grid, each cell's ink density)
//! - 8 horizontal-projection bins (ink-per-row, resampled to 8 bins)
//! - 8 vertical-projection bins (ink-per-column, resampled to 8 bins)
//! - horizontal_symmetry (1.0 - mean abs diff between left and right halves)
//! - vertical_symmetry   (1.0 - mean abs diff between top and bottom halves)
//! - avg row crossings (black↔white transitions per row, normalized)
//! - avg column crossings (black↔white transitions per column, normalized)
//! - hole count (approx Euler number via flood fill)
//! - 7 Hu moments (scale/rotation/translation invariants, log-magnitude)
//! - 12 HOG-lite cells (3×1 zone grid × 4 gradient-orientation bins)
//! - 4 skeleton topology values (endpoint_ratio, junction_ratio,
//!   skeleton_length_ratio, longest_run_ratio)
//!
//! **Breaking**: FEATURE_COUNT bumped again in v0.3. Prototype JSON files
//! generated against the prior count will fail to deserialize; retrain with
//! `examples/train_prototypes.rs`.

use image::GrayImage;

pub const FEATURE_COUNT: usize = 55;

/// Heap-allocated feature vector of length [`FEATURE_COUNT`]. We use a
/// `Vec<f32>` rather than a fixed-size array so that serde can derive
/// `Serialize`/`Deserialize` (default derives don't cover arrays longer than
/// 32 elements). The length invariant is enforced by every constructor in
/// this crate.
pub type FeatureVec = Vec<f32>;

/// Allocate a zero-filled feature vector of the expected length.
pub fn zero_features() -> FeatureVec {
    vec![0.0; FEATURE_COUNT]
}

pub fn extract(img: &GrayImage) -> FeatureVec {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return zero_features();
    }
    let w_f = w as f32;
    let h_f = h as f32;

    let mut v = zero_features();

    let total_pixels = (w * h) as f32;
    let mut ink_total: u32 = 0;
    let mut row_ink = vec![0u32; h as usize];
    let mut col_ink = vec![0u32; w as usize];
    for y in 0..h {
        for x in 0..w {
            if is_ink(img.get_pixel(x, y)[0]) {
                ink_total += 1;
                row_ink[y as usize] += 1;
                col_ink[x as usize] += 1;
            }
        }
    }

    v[0] = w_f / h_f;
    v[1] = ink_total as f32 / total_pixels;

    // 3x3 zone densities — feature indices 2..11.
    for zy in 0..3u32 {
        for zx in 0..3u32 {
            let x0 = (zx * w) / 3;
            let x1 = ((zx + 1) * w) / 3;
            let y0 = (zy * h) / 3;
            let y1 = ((zy + 1) * h) / 3;
            let mut zone_ink: u32 = 0;
            let zone_total = (x1.saturating_sub(x0)) * (y1.saturating_sub(y0));
            if zone_total == 0 {
                continue;
            }
            for y in y0..y1 {
                for x in x0..x1 {
                    if is_ink(img.get_pixel(x, y)[0]) {
                        zone_ink += 1;
                    }
                }
            }
            v[2 + (zy * 3 + zx) as usize] = zone_ink as f32 / zone_total as f32;
        }
    }

    // Horizontal-projection 8 bins — feature indices 11..19.
    resample_to_bins(&row_ink, w, &mut v[11..19]);
    // Vertical-projection 8 bins — feature indices 19..27.
    resample_to_bins(&col_ink, h, &mut v[19..27]);

    // Symmetry features — 27, 28.
    v[27] = horizontal_symmetry(img);
    v[28] = vertical_symmetry(img);

    // Crossings — 29, 30.
    v[29] = avg_row_crossings(img);
    v[30] = avg_col_crossings(img);

    // Hole count (Euler-number approximation) — 31.
    v[31] = hole_count(img) as f32;

    // Hu moments — 32..39 (7 values, log-magnitude for scale)
    let hu = hu_moments(img);
    for (i, m) in hu.iter().enumerate() {
        v[32 + i] = *m;
    }

    // HOG-lite — 39..51 (3 horizontal zones × 4 orientation bins)
    let hog = hog_lite(img);
    for (i, h) in hog.iter().enumerate() {
        v[39 + i] = *h;
    }

    // Skeleton topology — 51..55
    let skel = skeleton_stats(img);
    v[51] = skel.endpoint_ratio;
    v[52] = skel.junction_ratio;
    v[53] = skel.length_ratio;
    v[54] = skel.longest_run_ratio;

    v
}

/// Average number of ink/background transitions per row, normalized by width.
fn avg_row_crossings(img: &GrayImage) -> f32 {
    let (w, h) = img.dimensions();
    if w < 2 || h == 0 {
        return 0.0;
    }
    let mut total: u32 = 0;
    for y in 0..h {
        let mut last = is_ink(img.get_pixel(0, y)[0]);
        for x in 1..w {
            let cur = is_ink(img.get_pixel(x, y)[0]);
            if cur != last {
                total += 1;
            }
            last = cur;
        }
    }
    total as f32 / (h as f32 * (w - 1) as f32)
}

fn avg_col_crossings(img: &GrayImage) -> f32 {
    let (w, h) = img.dimensions();
    if h < 2 || w == 0 {
        return 0.0;
    }
    let mut total: u32 = 0;
    for x in 0..w {
        let mut last = is_ink(img.get_pixel(x, 0)[0]);
        for y in 1..h {
            let cur = is_ink(img.get_pixel(x, y)[0]);
            if cur != last {
                total += 1;
            }
            last = cur;
        }
    }
    total as f32 / (w as f32 * (h - 1) as f32)
}

/// Count holes via connected components of the background. A glyph is treated
/// as "ink" (low) and the OUTSIDE of the glyph bounding box is treated as
/// "reachable background". Any 4-connected background component enclosed by
/// ink is counted as a hole. Rough approximation of the Euler number.
fn hole_count(img: &GrayImage) -> u32 {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return 0;
    }
    // BFS flood-fill from every border background pixel; anything left
    // unvisited and still background is an interior hole.
    let mut visited = vec![false; (w * h) as usize];
    let mut queue: Vec<(u32, u32)> = Vec::new();
    let idx = |x: u32, y: u32| (y * w + x) as usize;

    for x in 0..w {
        for &y in &[0u32, h - 1] {
            if !is_ink(img.get_pixel(x, y)[0]) && !visited[idx(x, y)] {
                visited[idx(x, y)] = true;
                queue.push((x, y));
            }
        }
    }
    for y in 0..h {
        for &x in &[0u32, w - 1] {
            if !is_ink(img.get_pixel(x, y)[0]) && !visited[idx(x, y)] {
                visited[idx(x, y)] = true;
                queue.push((x, y));
            }
        }
    }
    while let Some((x, y)) = queue.pop() {
        for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let (nx, ny) = (nx as u32, ny as u32);
            if visited[idx(nx, ny)] {
                continue;
            }
            if !is_ink(img.get_pixel(nx, ny)[0]) {
                visited[idx(nx, ny)] = true;
                queue.push((nx, ny));
            }
        }
    }

    // Count 4-connected unvisited background components.
    let mut hole_count = 0u32;
    for y in 0..h {
        for x in 0..w {
            if visited[idx(x, y)] || is_ink(img.get_pixel(x, y)[0]) {
                continue;
            }
            hole_count += 1;
            queue.push((x, y));
            while let Some((cx, cy)) = queue.pop() {
                if visited[idx(cx, cy)] {
                    continue;
                }
                visited[idx(cx, cy)] = true;
                for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let (nx, ny) = (nx as u32, ny as u32);
                    if !visited[idx(nx, ny)] && !is_ink(img.get_pixel(nx, ny)[0]) {
                        queue.push((nx, ny));
                    }
                }
            }
        }
    }
    hole_count
}

fn resample_to_bins(source: &[u32], cross_axis_len: u32, out: &mut [f32]) {
    let n = source.len();
    let bins = out.len();
    if n == 0 || bins == 0 || cross_axis_len == 0 {
        return;
    }
    for (i, slot) in out.iter_mut().enumerate() {
        let lo = (i * n) / bins;
        let hi = ((i + 1) * n) / bins;
        let hi = hi.max(lo + 1).min(n);
        let sum: u32 = source[lo..hi].iter().sum();
        let cells = (hi - lo) as u32 * cross_axis_len;
        *slot = if cells == 0 {
            0.0
        } else {
            sum as f32 / cells as f32
        };
    }
}

fn horizontal_symmetry(img: &GrayImage) -> f32 {
    let (w, h) = img.dimensions();
    if w < 2 || h == 0 {
        return 1.0;
    }
    let half = w / 2;
    let mut diff: u64 = 0;
    let mut total: u64 = 0;
    for y in 0..h {
        for x in 0..half {
            let mirror_x = w - 1 - x;
            let a = img.get_pixel(x, y)[0];
            let b = img.get_pixel(mirror_x, y)[0];
            diff += (a as i32 - b as i32).unsigned_abs() as u64;
            total += 255;
        }
    }
    if total == 0 {
        1.0
    } else {
        1.0 - (diff as f32 / total as f32)
    }
}

fn vertical_symmetry(img: &GrayImage) -> f32 {
    let (w, h) = img.dimensions();
    if h < 2 || w == 0 {
        return 1.0;
    }
    let half = h / 2;
    let mut diff: u64 = 0;
    let mut total: u64 = 0;
    for y in 0..half {
        let mirror_y = h - 1 - y;
        for x in 0..w {
            let a = img.get_pixel(x, y)[0];
            let b = img.get_pixel(x, mirror_y)[0];
            diff += (a as i32 - b as i32).unsigned_abs() as u64;
            total += 255;
        }
    }
    if total == 0 {
        1.0
    } else {
        1.0 - (diff as f32 / total as f32)
    }
}

#[inline]
fn is_ink(v: u8) -> bool {
    v < 128
}

/// Hu moment invariants. Returns 7 values, log-transformed so their
/// magnitudes are comparable (`sign * log10(|φ| + ε)`). Scale/rotation/
/// translation-invariant shape descriptors for binary images.
fn hu_moments(img: &GrayImage) -> [f32; 7] {
    let (w, h) = img.dimensions();
    // Raw moments m_pq = Σ x^p y^q I(x,y).
    let mut m00 = 0f64;
    let mut m10 = 0f64;
    let mut m01 = 0f64;
    for y in 0..h {
        for x in 0..w {
            if is_ink(img.get_pixel(x, y)[0]) {
                let xf = x as f64;
                let yf = y as f64;
                m00 += 1.0;
                m10 += xf;
                m01 += yf;
            }
        }
    }
    if m00 < 1.0 {
        return [0.0; 7];
    }
    let cx = m10 / m00;
    let cy = m01 / m00;

    // Central moments μ_pq.
    let mut mu = [[0f64; 4]; 4]; // mu[p][q]
    for y in 0..h {
        for x in 0..w {
            if is_ink(img.get_pixel(x, y)[0]) {
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                for p in 0..4 {
                    for q in 0..4 {
                        if p + q > 3 {
                            continue;
                        }
                        mu[p][q] += dx.powi(p as i32) * dy.powi(q as i32);
                    }
                }
            }
        }
    }

    // Normalized central moments η_pq = μ_pq / μ00^((p+q)/2 + 1).
    let eta = |p: usize, q: usize| -> f64 {
        let gamma = (p + q) as f64 / 2.0 + 1.0;
        mu[p][q] / m00.powf(gamma)
    };

    let n20 = eta(2, 0);
    let n02 = eta(0, 2);
    let n11 = eta(1, 1);
    let n30 = eta(3, 0);
    let n12 = eta(1, 2);
    let n21 = eta(2, 1);
    let n03 = eta(0, 3);

    let phi1 = n20 + n02;
    let phi2 = (n20 - n02).powi(2) + 4.0 * n11 * n11;
    let phi3 = (n30 - 3.0 * n12).powi(2) + (3.0 * n21 - n03).powi(2);
    let phi4 = (n30 + n12).powi(2) + (n21 + n03).powi(2);
    let phi5 = (n30 - 3.0 * n12) * (n30 + n12) * ((n30 + n12).powi(2) - 3.0 * (n21 + n03).powi(2))
        + (3.0 * n21 - n03) * (n21 + n03)
            * (3.0 * (n30 + n12).powi(2) - (n21 + n03).powi(2));
    let phi6 = (n20 - n02) * ((n30 + n12).powi(2) - (n21 + n03).powi(2))
        + 4.0 * n11 * (n30 + n12) * (n21 + n03);
    let phi7 = (3.0 * n21 - n03) * (n30 + n12)
        * ((n30 + n12).powi(2) - 3.0 * (n21 + n03).powi(2))
        - (n30 - 3.0 * n12) * (n21 + n03)
            * (3.0 * (n30 + n12).powi(2) - (n21 + n03).powi(2));

    let raw = [phi1, phi2, phi3, phi4, phi5, phi6, phi7];
    let mut out = [0f32; 7];
    for (i, v) in raw.iter().enumerate() {
        let abs = v.abs();
        out[i] = (v.signum() * (abs + 1e-10).log10()) as f32;
    }
    out
}

/// HOG-lite: Sobel gradient orientation histogram over 3 horizontal zones,
/// each with 4 orientation bins (0°, 45°, 90°, 135°). Returns 12 values,
/// each zone's histogram L1-normalized to sum to 1.
fn hog_lite(img: &GrayImage) -> [f32; 12] {
    let (w, h) = img.dimensions();
    let mut out = [0f32; 12];
    if w < 3 || h < 3 {
        return out;
    }
    let mut bins = [[0f32; 4]; 3]; // 3 zones × 4 bins

    for y in 1..h - 1 {
        // Zone index (0, 1, 2) by vertical position.
        let zone = ((y as u64 * 3) / h as u64).min(2) as usize;
        for x in 1..w - 1 {
            let gx = img.get_pixel(x + 1, y)[0] as f32 - img.get_pixel(x - 1, y)[0] as f32;
            let gy = img.get_pixel(x, y + 1)[0] as f32 - img.get_pixel(x, y - 1)[0] as f32;
            let mag = (gx * gx + gy * gy).sqrt();
            if mag < 10.0 {
                continue;
            }
            // Unsigned orientation in [0, π).
            let angle = gy.atan2(gx).abs();
            let bin = ((angle / std::f32::consts::PI) * 4.0).min(3.999) as usize;
            bins[zone][bin] += mag;
        }
    }

    // L1 normalize each zone.
    for (zi, zone) in bins.iter_mut().enumerate() {
        let sum: f32 = zone.iter().sum();
        if sum > 0.0 {
            for v in zone.iter_mut() {
                *v /= sum;
            }
        }
        for (bi, &v) in zone.iter().enumerate() {
            out[zi * 4 + bi] = v;
        }
    }
    out
}

struct SkeletonStats {
    endpoint_ratio: f32,
    junction_ratio: f32,
    length_ratio: f32,
    longest_run_ratio: f32,
}

/// Zhang-Suen thinning + topological stats on the resulting skeleton.
///
/// - `endpoint_ratio`: endpoints (skeleton pixels with ≤1 neighbor) / skeleton length
/// - `junction_ratio`: branch points (skeleton pixels with ≥3 neighbors) / skeleton length
/// - `length_ratio`: skeleton pixels / image area (compactness)
/// - `longest_run_ratio`: longest horizontal run on skeleton / width
fn skeleton_stats(img: &GrayImage) -> SkeletonStats {
    let (w, h) = img.dimensions();
    if w < 3 || h < 3 {
        return SkeletonStats {
            endpoint_ratio: 0.0,
            junction_ratio: 0.0,
            length_ratio: 0.0,
            longest_run_ratio: 0.0,
        };
    }
    // Binary buffer: 1 = ink, 0 = background.
    let mut buf: Vec<u8> = vec![0; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            if is_ink(img.get_pixel(x, y)[0]) {
                buf[(y * w + x) as usize] = 1;
            }
        }
    }

    loop {
        let mut removed_any = false;
        for step in 0..2 {
            let mut to_remove: Vec<usize> = Vec::new();
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    let i = (y * w + x) as usize;
                    if buf[i] != 1 {
                        continue;
                    }
                    let p = neighbors8(&buf, w, x, y);
                    let b = p.iter().filter(|&&v| v == 1).count() as u8;
                    if !(2..=6).contains(&b) {
                        continue;
                    }
                    let a = transitions(&p);
                    if a != 1 {
                        continue;
                    }
                    // Zhang-Suen step conditions alternate.
                    let cond = if step == 0 {
                        p[0] * p[2] * p[4] == 0 && p[2] * p[4] * p[6] == 0
                    } else {
                        p[0] * p[2] * p[6] == 0 && p[0] * p[4] * p[6] == 0
                    };
                    if cond {
                        to_remove.push(i);
                    }
                }
            }
            if to_remove.is_empty() {
                continue;
            }
            removed_any = true;
            for i in to_remove {
                buf[i] = 0;
            }
        }
        if !removed_any {
            break;
        }
    }

    // Count endpoints and junctions on the thinned skeleton.
    let mut skeleton_len = 0u32;
    let mut endpoints = 0u32;
    let mut junctions = 0u32;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if buf[i] != 1 {
                continue;
            }
            skeleton_len += 1;
            let count = if x > 0 && x < w - 1 && y > 0 && y < h - 1 {
                neighbors8(&buf, w, x, y).iter().filter(|&&v| v == 1).count() as u32
            } else {
                1 // border; treat as endpoint
            };
            if count <= 1 {
                endpoints += 1;
            } else if count >= 3 {
                junctions += 1;
            }
        }
    }

    // Longest horizontal skeleton run.
    let mut longest = 0u32;
    for y in 0..h {
        let mut run = 0u32;
        for x in 0..w {
            if buf[(y * w + x) as usize] == 1 {
                run += 1;
                longest = longest.max(run);
            } else {
                run = 0;
            }
        }
    }

    let area = (w * h) as f32;
    let len_f = skeleton_len.max(1) as f32;
    SkeletonStats {
        endpoint_ratio: endpoints as f32 / len_f,
        junction_ratio: junctions as f32 / len_f,
        length_ratio: skeleton_len as f32 / area,
        longest_run_ratio: longest as f32 / w as f32,
    }
}

fn neighbors8(buf: &[u8], w: u32, x: u32, y: u32) -> [u8; 8] {
    // Clockwise starting north: P2, P3, P4, P5, P6, P7, P8, P9.
    let idx = |xx: u32, yy: u32| (yy * w + xx) as usize;
    [
        buf[idx(x, y - 1)],
        buf[idx(x + 1, y - 1)],
        buf[idx(x + 1, y)],
        buf[idx(x + 1, y + 1)],
        buf[idx(x, y + 1)],
        buf[idx(x - 1, y + 1)],
        buf[idx(x - 1, y)],
        buf[idx(x - 1, y - 1)],
    ]
}

fn transitions(p: &[u8; 8]) -> u32 {
    let mut a = 0u32;
    for i in 0..8 {
        let cur = p[i];
        let next = p[(i + 1) % 8];
        if cur == 0 && next == 1 {
            a += 1;
        }
    }
    a
}
