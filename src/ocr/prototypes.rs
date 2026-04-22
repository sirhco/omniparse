//! Hand-authored bitmap glyph prototypes used to seed the default recognizer.
//!
//! Format: ASCII-art. `#` marks ink, anything else (space, `.`) is background.
//! Rows should be uniform length; the parser trims trailing whitespace and
//! ignores the first line (allowing a leading newline in raw string literals).
//!
//! The bundled set covers uppercase A–Z and digits 0–9 in a 7×9 grid. It is
//! deliberately minimal — expect accuracy to fall off on non-matching fonts.
//! Callers who need better results should supply their own reference set
//! through [`crate::ocr::recognize::FeatureRecognizer::new`].

use crate::ocr::error::{OcrError, OcrResult};
use crate::ocr::features::extract;
use crate::ocr::recognize::Prototype;
use image::{GrayImage, Luma};
use std::path::Path;

/// Build a [`Prototype`] from an ASCII-art bitmap.
///
/// Any character drawn with `#` becomes ink (0), any other non-newline
/// character becomes background (255). The resulting `GrayImage` is then fed
/// to [`extract`] to produce the feature vector.
pub fn prototype_from_art(label: char, art: &str) -> Prototype {
    let rows: Vec<&str> = art
        .lines()
        .map(str::trim_end)
        .filter(|row| !row.is_empty())
        .collect();
    let height = rows.len() as u32;
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0) as u32;

    let mut full = GrayImage::from_pixel(width.max(1), height.max(1), Luma([255]));
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == '#' {
                full.put_pixel(x as u32, y as u32, Luma([0]));
            }
        }
    }

    // Tight-crop to the ink bounding box so features align with the cropped
    // regions the recognizer sees after connected-component layout.
    let img = tight_crop(&full);
    Prototype {
        label,
        features: extract(&img),
    }
}

fn tight_crop(src: &GrayImage) -> GrayImage {
    let (w, h) = src.dimensions();
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    for y in 0..h {
        for x in 0..w {
            if src.get_pixel(x, y)[0] < 128 {
                if x < min_x {
                    min_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if x > max_x {
                    max_x = x;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }
    }
    if min_x > max_x || min_y > max_y {
        return src.clone();
    }
    let cw = max_x - min_x + 1;
    let ch = max_y - min_y + 1;
    image::imageops::crop_imm(src, min_x, min_y, cw, ch).to_image()
}

/// Glyph bitmaps shipped with the crate. One entry per labeled class.
pub const BUNDLED_GLYPHS: &[(char, &str)] = &[
    (
        '0',
        "
.#####.
##...##
##...##
##...##
##...##
##...##
##...##
##...##
.#####.",
    ),
    (
        '1',
        "
...##..
..###..
.####..
...##..
...##..
...##..
...##..
...##..
######.",
    ),
    (
        '2',
        "
.#####.
##...##
.....##
....##.
...##..
..##...
.##....
##.....
#######",
    ),
    (
        '3',
        "
.#####.
##...##
.....##
...###.
.....##
.....##
##...##
##...##
.#####.",
    ),
    (
        '4',
        "
....##.
...###.
..####.
.##.##.
##..##.
#######
....##.
....##.
....##.",
    ),
    (
        '5',
        "
#######
##.....
##.....
######.
.....##
.....##
##...##
##...##
.#####.",
    ),
    (
        '6',
        "
.#####.
##...##
##.....
##.....
######.
##...##
##...##
##...##
.#####.",
    ),
    (
        '7',
        "
#######
##...##
.....##
....##.
...##..
..##...
..##...
..##...
..##...",
    ),
    (
        '8',
        "
.#####.
##...##
##...##
##...##
.#####.
##...##
##...##
##...##
.#####.",
    ),
    (
        '9',
        "
.#####.
##...##
##...##
##...##
.######
.....##
.....##
##...##
.#####.",
    ),
    (
        'A',
        "
...#...
..###..
..###..
.##.##.
.##.##.
#######
##...##
##...##
##...##",
    ),
    (
        'B',
        "
######.
##...##
##...##
##...##
######.
##...##
##...##
##...##
######.",
    ),
    (
        'C',
        "
.#####.
##...##
##.....
##.....
##.....
##.....
##.....
##...##
.#####.",
    ),
    (
        'D',
        "
#####..
##..##.
##...##
##...##
##...##
##...##
##...##
##..##.
#####..",
    ),
    (
        'E',
        "
#######
##.....
##.....
##.....
######.
##.....
##.....
##.....
#######",
    ),
    (
        'F',
        "
#######
##.....
##.....
##.....
######.
##.....
##.....
##.....
##.....",
    ),
    (
        'G',
        "
.#####.
##...##
##.....
##.....
##..###
##...##
##...##
##...##
.#####.",
    ),
    (
        'H',
        "
##...##
##...##
##...##
##...##
#######
##...##
##...##
##...##
##...##",
    ),
    (
        'I',
        "
#####..
.##....
.##....
.##....
.##....
.##....
.##....
.##....
#####..",
    ),
    (
        'J',
        "
...####
.....##
.....##
.....##
.....##
.....##
##...##
##...##
.#####.",
    ),
    (
        'K',
        "
##..##.
##.##..
####...
###....
####...
####...
##.##..
##..##.
##...##",
    ),
    (
        'L',
        "
##.....
##.....
##.....
##.....
##.....
##.....
##.....
##.....
#######",
    ),
    (
        'M',
        "
##...##
###.###
#######
##.#.##
##...##
##...##
##...##
##...##
##...##",
    ),
    (
        'N',
        "
##...##
###..##
####.##
#######
##.####
##..###
##...##
##...##
##...##",
    ),
    (
        'O',
        "
.#####.
##...##
##...##
##...##
##...##
##...##
##...##
##...##
.#####.",
    ),
    (
        'P',
        "
######.
##...##
##...##
##...##
######.
##.....
##.....
##.....
##.....",
    ),
    (
        'Q',
        "
.#####.
##...##
##...##
##...##
##...##
##.#.##
##..###
##...##
.######",
    ),
    (
        'R',
        "
######.
##...##
##...##
##...##
######.
####...
##.##..
##..##.
##...##",
    ),
    (
        'S',
        "
.#####.
##...##
##.....
##.....
.#####.
.....##
.....##
##...##
.#####.",
    ),
    (
        'T',
        "
#######
#.##.#.
..##...
..##...
..##...
..##...
..##...
..##...
..##...",
    ),
    (
        'U',
        "
##...##
##...##
##...##
##...##
##...##
##...##
##...##
##...##
.#####.",
    ),
    (
        'V',
        "
##...##
##...##
##...##
##...##
##...##
##...##
.##.##.
..###..
...#...",
    ),
    (
        'W',
        "
##...##
##...##
##...##
##...##
##.#.##
##.#.##
#######
###.###
##...##",
    ),
    (
        'X',
        "
##...##
##...##
.##.##.
..###..
...#...
..###..
.##.##.
##...##
##...##",
    ),
    (
        'Y',
        "
##...##
##...##
.##.##.
..###..
...#...
...#...
...#...
...#...
...#...",
    ),
    (
        'Z',
        "
#######
#....##
....##.
...##..
..##...
.##....
##.....
##.....
#######",
    ),
];

/// Build prototypes for every glyph in [`BUNDLED_GLYPHS`].
pub fn bundled_prototypes() -> Vec<Prototype> {
    BUNDLED_GLYPHS
        .iter()
        .map(|(ch, art)| prototype_from_art(*ch, art))
        .collect()
}

/// Serialize a prototype set to JSON and write it to `path`.
pub fn save_prototypes_json(prototypes: &[Prototype], path: impl AsRef<Path>) -> OcrResult<()> {
    let json = serde_json::to_vec_pretty(prototypes)
        .map_err(|e| OcrError::Config(format!("serialize prototypes: {e}")))?;
    std::fs::write(path.as_ref(), json).map_err(OcrError::Io)?;
    Ok(())
}

/// Load a prototype set from a JSON file previously written via
/// [`save_prototypes_json`].
///
/// Validates that every prototype's feature vector matches the current
/// [`crate::ocr::features::FEATURE_COUNT`]. Mismatches produce a descriptive
/// error asking the caller to retrain — silently accepting a wrong-length
/// vector would panic later during recognition.
pub fn load_prototypes_json(path: impl AsRef<Path>) -> OcrResult<Vec<Prototype>> {
    use crate::ocr::features::FEATURE_COUNT;
    let bytes = std::fs::read(path.as_ref()).map_err(OcrError::Io)?;
    let protos: Vec<Prototype> = serde_json::from_slice(&bytes)
        .map_err(|e| OcrError::Config(format!("parse prototypes: {e}")))?;
    for p in &protos {
        if p.features.len() != FEATURE_COUNT {
            return Err(OcrError::Config(format!(
                "prototype '{}' has {} features but this build expects {}. \
                 The feature vector was extended in a recent release — retrain with \
                 `cargo run --features ocr-train --example train_prototypes`.",
                p.label,
                p.features.len(),
                FEATURE_COUNT
            )));
        }
    }
    Ok(protos)
}

/// Reduce a prototype set by clustering near-duplicates within each label.
///
/// Multi-font × multi-scale training produces dozens of feature vectors per
/// label that often cluster tightly in feature space. k-medoids per label
/// picks `max_per_label` representative prototypes (real feature vectors,
/// not centroids) that best cover the label's feature-space distribution.
/// Drops the overall prototype count without materially hurting accuracy.
///
/// `max_per_label = 1` collapses each label to a single medoid.
pub fn dedupe_prototypes(
    prototypes: Vec<Prototype>,
    max_per_label: usize,
) -> Vec<Prototype> {
    let max_per_label = max_per_label.max(1);
    let mut by_label: std::collections::HashMap<char, Vec<Prototype>> =
        std::collections::HashMap::new();
    for p in prototypes {
        by_label.entry(p.label).or_default().push(p);
    }

    let mut out: Vec<Prototype> = Vec::new();
    for (_label, group) in by_label {
        if group.len() <= max_per_label {
            out.extend(group);
            continue;
        }
        out.extend(k_medoids(group, max_per_label));
    }
    out
}

/// Simple PAM-like k-medoids: choose `k` items from `items` that minimize
/// total within-cluster distance sum. Greedy swap-to-improve loop up to 50
/// iterations. Good enough for prototype deduplication.
fn k_medoids(items: Vec<Prototype>, k: usize) -> Vec<Prototype> {
    let n = items.len();
    if k >= n {
        return items;
    }
    // Precompute pairwise distances (O(n²) memory — fine for the per-label
    // groups we operate on, typically n < 200).
    let mut dist = vec![0f32; n * n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = euclidean(&items[i].features, &items[j].features);
            dist[i * n + j] = d;
            dist[j * n + i] = d;
        }
    }

    // Initialize medoids deterministically: spread picks across the input.
    let mut medoids: Vec<usize> = (0..k).map(|i| (i * n) / k).collect();

    for _iter in 0..50 {
        let mut total_cost: f32 = (0..n)
            .map(|i| (0..k).map(|m| dist[i * n + medoids[m]]).fold(f32::INFINITY, f32::min))
            .sum();
        let mut improved = false;
        for m_idx in 0..k {
            for candidate in 0..n {
                if medoids.contains(&candidate) {
                    continue;
                }
                let original = medoids[m_idx];
                medoids[m_idx] = candidate;
                let new_cost: f32 = (0..n)
                    .map(|i| {
                        (0..k)
                            .map(|m| dist[i * n + medoids[m]])
                            .fold(f32::INFINITY, f32::min)
                    })
                    .sum();
                if new_cost < total_cost {
                    total_cost = new_cost;
                    improved = true;
                } else {
                    medoids[m_idx] = original;
                }
            }
        }
        if !improved {
            break;
        }
    }

    medoids.iter().map(|&i| items[i].clone()).collect()
}

fn euclidean(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}
