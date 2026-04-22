//! Stage 3 of the OCR pipeline: transcribe per-glyph regions into characters.
//!
//! The shipped backend is a pure classical 1-nearest-neighbor classifier over
//! hand-crafted glyph features (`features` module). It has **no** ML model and
//! **no** runtime inference engine — the only persistent state is a bundle of
//! labeled reference feature vectors.
//!
//! Accuracy is modest on clean printed Latin text and degrades quickly on
//! noise, skew, or unusual fonts. Callers who need higher quality can
//! implement the `Recognizer` trait with their own backend.

use crate::ocr::error::OcrResult;
use crate::ocr::features::{extract, FeatureVec, FEATURE_COUNT};
use crate::ocr::layout::TextRegion;
use image::GrayImage;

#[derive(Clone, Debug)]
pub struct RecognizedLine {
    pub text: String,
    pub confidence: f32,
    pub region: TextRegion,
    /// Top candidate labels sorted ascending by distance. The first entry
    /// always matches `text[0]`. Useful for downstream re-ranking
    /// (dictionary, n-gram, beam search).
    pub alternatives: Vec<(char, f32)>,
}

pub trait Recognizer: Send + Sync {
    fn recognize(&self, img: &GrayImage, region: &TextRegion) -> OcrResult<RecognizedLine>;
}

/// Recognizer that always returns empty text. Placeholder when reference data
/// hasn't been supplied.
pub struct NullRecognizer;

impl Recognizer for NullRecognizer {
    fn recognize(&self, _img: &GrayImage, region: &TextRegion) -> OcrResult<RecognizedLine> {
        Ok(RecognizedLine {
            text: String::new(),
            confidence: 0.0,
            region: region.clone(),
            alternatives: Vec::new(),
        })
    }
}

/// A labeled reference glyph: features + the character it represents.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Prototype {
    pub label: char,
    pub features: FeatureVec,
}

/// Classical k-nearest-neighbor classifier over feature vectors.
///
/// Distance = Euclidean over the feature vector. Confidence comes from an
/// exponential decay on the winning neighbor's distance (`exp(-d / scale)`).
/// When `k > 1`, nearest neighbors vote with weights proportional to
/// `1 / (distance + ε)`; the most-weighted label wins.
pub struct FeatureRecognizer {
    prototypes: Vec<Prototype>,
    /// Distance at which confidence drops to `exp(-1) ≈ 0.37`.
    pub distance_scale: f32,
    /// Number of neighbors consulted per recognition. `k = 1` reproduces the
    /// original 1-NN behavior.
    pub k: usize,
    /// When `true`, extract features from both the region crop AND its
    /// inverted copy; keep the label whose nearest-neighbor distance is
    /// smaller. Costs 2× feature extraction per region.
    pub try_both_polarities: bool,
    /// When set, resize each region crop to this canonical height
    /// (preserving aspect) before feature extraction. Compensates for the
    /// fact that feature vectors aren't fully scale-invariant at small
    /// image sizes. Typical values: 24, 32, 48.
    pub normalize_height: Option<u32>,
    /// Optional k-d tree built from `prototypes`. When present, NN lookups
    /// use the tree; otherwise a linear scan is performed. Populated via
    /// [`FeatureRecognizer::build_kdtree`].
    kdtree: Option<crate::ocr::kdtree::KdTree>,
}

impl FeatureRecognizer {
    pub fn new(prototypes: Vec<Prototype>) -> Self {
        Self {
            prototypes,
            distance_scale: 1.0,
            k: 1,
            try_both_polarities: false,
            normalize_height: None,
            kdtree: None,
        }
    }

    pub fn with_normalize_height(mut self, h: Option<u32>) -> Self {
        self.normalize_height = h;
        self
    }

    /// Build and retain a k-d tree over the current prototype set. Recommended
    /// when the prototype count exceeds a few hundred; below that threshold
    /// the linear scan is faster due to smaller per-step overhead.
    pub fn build_kdtree(mut self) -> Self {
        self.kdtree = Some(crate::ocr::kdtree::KdTree::new(&self.prototypes));
        self
    }

    pub fn with_default_prototypes() -> Self {
        Self::new(default_prototypes())
    }

    pub fn with_k(mut self, k: usize) -> Self {
        self.k = k.max(1);
        self
    }

    pub fn with_both_polarities(mut self, enabled: bool) -> Self {
        self.try_both_polarities = enabled;
        self
    }

    pub fn prototypes(&self) -> &[Prototype] {
        &self.prototypes
    }
}

impl Recognizer for FeatureRecognizer {
    fn recognize(&self, img: &GrayImage, region: &TextRegion) -> OcrResult<RecognizedLine> {
        if self.prototypes.is_empty() {
            return Ok(RecognizedLine {
                text: String::new(),
                confidence: 0.0,
                region: region.clone(),
                alternatives: Vec::new(),
            });
        }

        let raw_glyph = crop(img, region);
        let glyph = match self.normalize_height {
            Some(h) if raw_glyph.height() > 0 => resize_to_height(&raw_glyph, h),
            _ => raw_glyph,
        };
        let feats = extract(&glyph);

        // Compute distances, keep top-k in a small partial-sort buffer.
        let k = self.k.min(self.prototypes.len()).max(1);
        let mut top = if let Some(tree) = &self.kdtree {
            tree.knn(&feats, k)
        } else {
            top_k_neighbors(&feats, &self.prototypes, k)
        };

        // If requested, also try the inverted crop and keep whichever
        // polarity produced the smaller nearest-neighbor distance.
        if self.try_both_polarities {
            let inverted = invert_gray(&glyph);
            let inv_feats = extract(&inverted);
            let inv_top = if let Some(tree) = &self.kdtree {
                tree.knn(&inv_feats, k)
            } else {
                top_k_neighbors(&inv_feats, &self.prototypes, k)
            };
            let best_d = top.first().map(|t| t.0).unwrap_or(f32::INFINITY);
            let inv_best = inv_top.first().map(|t| t.0).unwrap_or(f32::INFINITY);
            if inv_best < best_d {
                top = inv_top;
            }
        }

        // Vote. Weight = 1 / (d + ε). Best label = highest total weight.
        let mut votes: std::collections::HashMap<char, f32> = std::collections::HashMap::new();
        for (d, label) in &top {
            let w = 1.0 / (d + 1e-3);
            *votes.entry(*label).or_insert(0.0) += w;
        }
        let winner = votes
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(c, _)| *c)
            .unwrap_or(' ');

        let best_distance = top.first().map(|t| t.0).unwrap_or(f32::INFINITY);
        let confidence = (-best_distance / self.distance_scale).exp();

        // Deduplicate alternatives by label (same label can appear multiple
        // times across different prototype instances — e.g. per-font and per-
        // size copies). Keep the closest distance per label.
        let mut seen: std::collections::HashMap<char, f32> =
            std::collections::HashMap::new();
        for (d, label) in &top {
            seen.entry(*label)
                .and_modify(|cur| {
                    if *d < *cur {
                        *cur = *d
                    }
                })
                .or_insert(*d);
        }
        let mut alternatives: Vec<(char, f32)> =
            seen.into_iter().map(|(c, d)| (c, d)).collect();
        alternatives.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(RecognizedLine {
            text: winner.to_string(),
            confidence,
            region: region.clone(),
            alternatives,
        })
    }
}

fn top_k_neighbors(feats: &FeatureVec, prototypes: &[Prototype], k: usize) -> Vec<(f32, char)> {
    let mut top: Vec<(f32, char)> = Vec::with_capacity(k + 1);
    for proto in prototypes {
        let d = squared_distance(feats, &proto.features).sqrt();
        if top.len() < k {
            top.push((d, proto.label));
            top.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        } else if d < top.last().unwrap().0 {
            top.pop();
            top.push((d, proto.label));
            top.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        }
    }
    top
}

fn resize_to_height(img: &GrayImage, target_h: u32) -> GrayImage {
    let (w, h) = img.dimensions();
    if h == target_h || h == 0 {
        return img.clone();
    }
    let ratio = target_h as f32 / h as f32;
    let new_w = ((w as f32 * ratio).round() as u32).max(1);
    image::imageops::resize(
        img,
        new_w,
        target_h,
        image::imageops::FilterType::Triangle,
    )
}

fn invert_gray(img: &GrayImage) -> GrayImage {
    let (w, h) = img.dimensions();
    let mut out = GrayImage::new(w, h);
    for (x, y, px) in img.enumerate_pixels() {
        out.put_pixel(x, y, image::Luma([255 - px[0]]));
    }
    out
}

fn crop(img: &GrayImage, region: &TextRegion) -> GrayImage {
    let (w, h) = img.dimensions();
    let x = region.x.min(w.saturating_sub(1));
    let y = region.y.min(h.saturating_sub(1));
    let width = region.width.min(w - x);
    let height = region.height.min(h - y);
    image::imageops::crop_imm(img, x, y, width, height).to_image()
}

fn squared_distance(a: &FeatureVec, b: &FeatureVec) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..FEATURE_COUNT {
        let d = a[i] - b[i];
        sum += d * d;
    }
    sum
}

/// Built-in reference prototypes.
///
/// Seeded from hand-authored bitmaps (uppercase A–Z + digits 0–9). See
/// [`crate::ocr::prototypes::BUNDLED_GLYPHS`] for the raw art.
fn default_prototypes() -> Vec<Prototype> {
    crate::ocr::prototypes::bundled_prototypes()
}
