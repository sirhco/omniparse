//! Stage 4 of the OCR pipeline: correct recognizer output against a dictionary.

use crate::ocr::error::{OcrError, OcrResult};

pub trait PostProcessor: Send + Sync {
    fn correct(&self, text: &str) -> String;
}

/// No-op corrector — returns input unchanged. Useful default and test harness.
pub struct NoopCorrector;

impl PostProcessor for NoopCorrector {
    fn correct(&self, text: &str) -> String {
        text.to_string()
    }
}

/// Dictionary-based corrector backed by SymSpell (edit-distance lookup over a
/// hash table of known words — classical algorithm, no ML).
pub struct SymspellCorrector {
    inner: symspell::SymSpell<symspell::AsciiStringStrategy>,
    max_edit_distance: i64,
}

impl SymspellCorrector {
    /// Build a corrector from a newline-delimited `word<space>frequency` list.
    ///
    /// The bundled default list covers ~5k common English words; callers can
    /// supply a larger corpus via `with_wordlist`.
    pub fn with_wordlist(content: &str, max_edit_distance: i64) -> OcrResult<Self> {
        use symspell::{SymSpell, SymSpellBuilder};

        let mut inner: SymSpell<symspell::AsciiStringStrategy> = SymSpellBuilder::default()
            .max_dictionary_edit_distance(max_edit_distance)
            .prefix_length(7)
            .count_threshold(1)
            .build()
            .map_err(|e| OcrError::Config(format!("symspell build: {e:?}")))?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // symspell expects `word<sep>count` on each line. Our wordlist uses
            // whitespace; normalize to a single space for the loader.
            let mut parts = line.split_whitespace();
            if let (Some(word), Some(count)) = (parts.next(), parts.next()) {
                let normalized = format!("{word} {count}");
                inner.load_dictionary_line(&normalized, 0, 1, " ");
            }
        }
        Ok(Self {
            inner,
            max_edit_distance,
        })
    }

    /// Build with the small bundled English wordlist.
    pub fn with_default_wordlist() -> OcrResult<Self> {
        Self::with_wordlist(DEFAULT_WORDLIST, 2)
    }
}

impl PostProcessor for SymspellCorrector {
    fn correct(&self, text: &str) -> String {
        use symspell::Verbosity;
        text.split_inclusive(char::is_whitespace)
            .map(|token| {
                let (word, trailing): (String, String) = token
                    .chars()
                    .partition(|c| !c.is_whitespace());
                if word.is_empty() {
                    return token.to_string();
                }
                let suggestions = self
                    .inner
                    .lookup(&word.to_lowercase(), Verbosity::Top, self.max_edit_distance);
                if let Some(best) = suggestions.first() {
                    // Preserve the original case on the first character.
                    let mut corrected = best.term.clone();
                    if let Some(first) = word.chars().next() {
                        if first.is_uppercase() {
                            let mut chars = corrected.chars();
                            if let Some(c) = chars.next() {
                                corrected = c.to_uppercase().collect::<String>() + chars.as_str();
                            }
                        }
                    }
                    format!("{corrected}{trailing}")
                } else {
                    token.to_string()
                }
            })
            .collect()
    }
}

/// A minimal English wordlist used when no custom list is supplied.
///
/// Format: `word<space>frequency` per line. Kept intentionally small so the
/// baseline crate stays slim; production users should call `with_wordlist`
/// with a larger corpus (e.g. Peter Norvig's `big.txt` split).
pub const DEFAULT_WORDLIST: &str = include_str!("wordlist.txt");

/// Beam search over per-glyph top-k candidates, scored against a bigram
/// language model and a dictionary of known words. Produces the string that
/// jointly maximizes recognition confidence + bigram fluency + dictionary
/// membership across the whole line.
///
/// Algorithm:
/// 1. Maintain `beam_width` partial-string hypotheses per position.
/// 2. At each glyph, extend every hypothesis with every candidate label and
///    score = prev_score + α·recognition + β·bigram(prev_char, new_char) +
///    γ·dictionary_bonus.
/// 3. Keep top `beam_width` by combined score, drop the rest.
/// 4. At the end, return the best-scoring full hypothesis.
///
/// This is a simple word-level variant of Viterbi decoding; the dictionary
/// term is the only coupling that requires beam search rather than per-glyph
/// DP (because membership depends on the complete word-so-far).
pub fn beam_search_line(
    glyphs: &[crate::ocr::recognize::RecognizedLine],
    beam_width: usize,
    wordlist: &str,
) -> String {
    if glyphs.is_empty() {
        return String::new();
    }
    let beam_width = beam_width.max(1);
    let ranker = crate::ocr::bigram::BigramRanker::english();
    let dict = build_word_set(wordlist);

    #[derive(Clone)]
    struct Hypothesis {
        text: String,
        score: f32,
    }

    let mut beam: Vec<Hypothesis> = vec![Hypothesis {
        text: String::new(),
        score: 0.0,
    }];
    let alpha: f32 = 1.0;
    let beta: f32 = 0.6;
    let gamma: f32 = 0.8;

    for g in glyphs {
        let cands: Vec<(char, f32)> = if g.alternatives.is_empty() {
            vec![(g.text.chars().next().unwrap_or(' '), 0.0)]
        } else {
            g.alternatives.clone()
        };

        let mut extended: Vec<Hypothesis> = Vec::with_capacity(beam.len() * cands.len());
        for h in &beam {
            for (label, dist) in &cands {
                let prev_char = h.text.chars().last().unwrap_or(' ');
                let bigram = ranker
                    .log_probs
                    .get(&(prev_char.to_ascii_lowercase(), label.to_ascii_lowercase()))
                    .copied()
                    .unwrap_or(ranker.floor_log_prob);
                // Dictionary bonus: if the trailing word in the new text is
                // a prefix of (or equal to) a dictionary entry, reward it.
                let mut new_text = h.text.clone();
                new_text.push(*label);
                let bonus = dict_bonus(&new_text, &dict);
                let recog = -dist;
                let score = h.score + alpha * recog + beta * bigram + gamma * bonus;
                extended.push(Hypothesis {
                    text: new_text,
                    score,
                });
            }
        }

        extended.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        extended.truncate(beam_width);
        beam = extended;
    }

    beam.into_iter()
        .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
        .map(|h| h.text)
        .unwrap_or_default()
}

fn build_word_set(wordlist: &str) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    for line in wordlist.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(word) = trimmed.split_whitespace().next() {
            set.insert(word.to_ascii_lowercase());
        }
    }
    set
}

fn dict_bonus(text: &str, dict: &std::collections::HashSet<String>) -> f32 {
    // Look at the trailing word (chars after last whitespace).
    let last_word: String = text
        .chars()
        .rev()
        .take_while(|c| !c.is_whitespace())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if last_word.is_empty() {
        return 0.0;
    }
    if dict.contains(&last_word) {
        1.0
    } else if dict.iter().any(|w| w.starts_with(&last_word) && w.len() > last_word.len()) {
        0.2
    } else {
        0.0
    }
}
