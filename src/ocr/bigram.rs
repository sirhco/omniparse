//! Character bigram language model for post-recognition re-ranking.
//!
//! Given a sequence of per-glyph top-k candidates, pick each character to
//! jointly maximize `α · recognition_score + β · bigram_log_prob(prev, cur)`.
//! The bigram table is a bundled English character-frequency model derived
//! from common-word statistics — small, no external data required.
//!
//! Usage:
//!
//! ```no_run
//! # #[cfg(feature = "ocr")]
//! # fn go() -> Result<(), omniparse::Error> {
//! use omniparse::ocr::bigram::BigramRanker;
//! use omniparse::ocr::OcrEngine;
//! # let engine = OcrEngine::new();
//! # let img: image::DynamicImage = image::DynamicImage::new_luma8(1, 1);
//! let out = engine.recognize(img)?;
//! let ranker = BigramRanker::english();
//! let reranked = ranker.rerank_line(&out.lines);
//! println!("{}", reranked);
//! # Ok(()) }
//! ```

use crate::ocr::recognize::RecognizedLine;
use std::collections::HashMap;

/// Character bigram log-probability table. Missing pairs fall back to a
/// fixed-floor probability so the ranker never returns -∞.
pub struct BigramRanker {
    /// log P(cur | prev) for every observed pair.
    pub log_probs: HashMap<(char, char), f32>,
    /// Floor log-probability for pairs not in the table.
    pub floor_log_prob: f32,
    /// Weight for recognition confidence in the combined score.
    pub alpha: f32,
    /// Weight for bigram log-probability in the combined score.
    pub beta: f32,
}

impl BigramRanker {
    /// Build a ranker from a bundled English character bigram table.
    pub fn english() -> Self {
        let table = build_english_table();
        Self {
            log_probs: table,
            floor_log_prob: -8.0, // ≈ 1/3000 probability
            alpha: 1.0,
            beta: 0.6,
        }
    }

    /// Re-rank a grouped sequence of glyph recognitions, producing the
    /// joint-optimal character string. Glyphs without alternatives keep
    /// their top label. Whitespace/line breaks between regions are preserved
    /// by passing one line at a time.
    pub fn rerank_line(&self, glyphs: &[RecognizedLine]) -> String {
        if glyphs.is_empty() {
            return String::new();
        }
        // Viterbi over per-glyph candidate lists. State = candidate char at
        // position i. Transition cost = recognition term + bigram term.
        let mut dp: Vec<Vec<f32>> = Vec::with_capacity(glyphs.len());
        let mut back: Vec<Vec<usize>> = Vec::with_capacity(glyphs.len());

        // Initial column — recognition score only.
        let first_cands = candidates(&glyphs[0]);
        let init: Vec<f32> = first_cands
            .iter()
            .map(|(_, d)| self.alpha * recognition_score(*d))
            .collect();
        dp.push(init);
        back.push(vec![0; first_cands.len()]);

        let mut prev_cands = first_cands;
        for g in &glyphs[1..] {
            let cands = candidates(g);
            let mut col_score: Vec<f32> = Vec::with_capacity(cands.len());
            let mut col_back: Vec<usize> = Vec::with_capacity(cands.len());
            for (cand_label, cand_d) in &cands {
                let mut best = f32::NEG_INFINITY;
                let mut best_prev = 0usize;
                for (pi, (prev_label, _)) in prev_cands.iter().enumerate() {
                    let bigram = self.bigram_log_prob(*prev_label, *cand_label);
                    let score = dp.last().unwrap()[pi]
                        + self.alpha * recognition_score(*cand_d)
                        + self.beta * bigram;
                    if score > best {
                        best = score;
                        best_prev = pi;
                    }
                }
                col_score.push(best);
                col_back.push(best_prev);
            }
            dp.push(col_score);
            back.push(col_back);
            prev_cands = cands;
        }

        // Backtrace.
        let last_col = dp.last().unwrap();
        let mut idx = 0usize;
        let mut best_score = f32::NEG_INFINITY;
        for (i, &s) in last_col.iter().enumerate() {
            if s > best_score {
                best_score = s;
                idx = i;
            }
        }

        let mut chars: Vec<char> = Vec::with_capacity(glyphs.len());
        let mut cur_idx = idx;
        for col in (0..glyphs.len()).rev() {
            let cands = candidates(&glyphs[col]);
            let label = cands.get(cur_idx).map(|(c, _)| *c).unwrap_or(' ');
            chars.push(label);
            if col > 0 {
                cur_idx = back[col][cur_idx];
            }
        }
        chars.reverse();
        chars.into_iter().collect()
    }

    fn bigram_log_prob(&self, prev: char, cur: char) -> f32 {
        *self
            .log_probs
            .get(&(prev.to_ascii_lowercase(), cur.to_ascii_lowercase()))
            .unwrap_or(&self.floor_log_prob)
    }
}

fn candidates(g: &RecognizedLine) -> Vec<(char, f32)> {
    if g.alternatives.is_empty() {
        let label = g.text.chars().next().unwrap_or(' ');
        vec![(label, 0.0)]
    } else {
        g.alternatives.clone()
    }
}

fn recognition_score(distance: f32) -> f32 {
    // Convert distance to a log-probability-ish quantity. Smaller distance
    // = higher score. Scale chosen so typical classification distances
    // produce scores in ~[-2, 0].
    -(distance as f32)
}

/// English character bigram counts (approximate, per million bigrams from
/// common web corpora). Values are log-probabilities of cur given prev,
/// stored for 28 tokens: 'a'..='z', space, and '.' (period treated as
/// generic end-of-word).
///
/// Constructed from hand-authored log-smoothed counts, enough to strongly
/// prefer common English patterns (th, he, in, er, an, re, …) without
/// requiring an external data file.
fn build_english_table() -> HashMap<(char, char), f32> {
    // Raw bigram frequency estimates (×1000 per million). A subset of the
    // highest-frequency English bigrams with hand-smoothed medium
    // frequencies filling in. Not a substitute for a real corpus, but
    // enough to push the classifier away from obvious nonsense pairs.
    let raw: &[(char, char, f32)] = &[
        // high-frequency pairs
        ('t', 'h', 3.56), ('h', 'e', 3.07), ('i', 'n', 2.43), ('e', 'r', 2.05),
        ('a', 'n', 1.99), ('r', 'e', 1.85), ('o', 'n', 1.76), ('a', 't', 1.49),
        ('e', 'n', 1.45), ('n', 'd', 1.35), ('t', 'i', 1.34), ('e', 's', 1.34),
        ('o', 'r', 1.28), ('t', 'e', 1.20), ('o', 'f', 1.17), ('e', 'd', 1.17),
        ('i', 's', 1.13), ('i', 't', 1.12), ('a', 'l', 1.09), ('a', 'r', 1.07),
        ('s', 't', 1.05), ('t', 'o', 1.05), ('n', 't', 1.04), ('n', 'g', 0.95),
        ('s', 'e', 0.93), ('h', 'a', 0.93), ('a', 's', 0.87), ('o', 'u', 0.87),
        ('i', 'o', 0.83), ('l', 'e', 0.83), ('v', 'e', 0.83), ('c', 'o', 0.79),
        ('m', 'e', 0.79), ('d', 'e', 0.76), ('h', 'i', 0.76), ('r', 'i', 0.73),
        ('r', 'o', 0.73), ('i', 'c', 0.70), ('n', 'e', 0.69), ('e', 'a', 0.69),
        ('r', 'a', 0.69), ('c', 'e', 0.65), ('l', 'i', 0.62), ('l', 'l', 0.58),
        ('b', 'e', 0.58), ('m', 'a', 0.57), ('s', 'i', 0.55), ('o', 'm', 0.55),
        ('u', 'r', 0.54), ('c', 'a', 0.54), ('i', 'l', 0.52), ('d', 'i', 0.50),
        ('t', ' ', 2.00), (' ', 't', 1.75), (' ', 'a', 1.70), ('e', ' ', 1.90),
        (' ', 's', 1.10), (' ', 'o', 0.90), ('d', ' ', 0.95), ('s', ' ', 1.50),
        (' ', 'i', 0.88), ('n', ' ', 1.40), ('y', ' ', 0.72), (' ', 'c', 0.75),
    ];

    // Normalize to log-probabilities per preceding char. First, accumulate
    // totals for each `prev` row.
    let mut row_totals: HashMap<char, f32> = HashMap::new();
    for (prev, _cur, freq) in raw {
        *row_totals.entry(*prev).or_insert(0.0) += freq;
    }
    // Add a uniform smoothing mass so un-seen cur chars after a seen prev
    // aren't left at the floor.
    let smoothing = 0.2f32;
    let alphabet_size = 27.0f32; // 26 letters + space

    let mut table: HashMap<(char, char), f32> = HashMap::new();
    for (prev, cur, freq) in raw {
        let row_total = row_totals.get(prev).copied().unwrap_or(1.0);
        let prob = (freq + smoothing) / (row_total + smoothing * alphabet_size);
        table.insert((*prev, *cur), prob.ln());
    }
    table
}
