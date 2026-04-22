//! Script / language heuristic detection.
//!
//! Pure-Rust, no training data: counts characters by Unicode-range bucket
//! and returns the dominant script. Useful when the recognizer emits mixed-
//! script output and the caller wants to route the text to a language-
//! appropriate post-processor.
//!
//! Supported buckets:
//! - `Latin` (A-Z, a-z, and Latin-1 supplement letters)
//! - `Cyrillic`
//! - `Greek`
//! - `Arabic`
//! - `Hebrew`
//! - `Han` (CJK Unified Ideographs)
//! - `Hiragana`
//! - `Katakana`
//! - `Hangul`
//! - `Devanagari`
//! - `Digit` (0-9)
//! - `Other` (punctuation, symbols, etc.)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Script {
    Latin,
    Cyrillic,
    Greek,
    Arabic,
    Hebrew,
    Han,
    Hiragana,
    Katakana,
    Hangul,
    Devanagari,
    Digit,
    Other,
}

/// Classify a single character by script.
pub fn script_of(c: char) -> Script {
    let code = c as u32;
    if c.is_ascii_digit() {
        return Script::Digit;
    }
    match code {
        0x0041..=0x005A | 0x0061..=0x007A | 0x00C0..=0x024F => Script::Latin,
        0x0370..=0x03FF => Script::Greek,
        0x0400..=0x04FF => Script::Cyrillic,
        0x0590..=0x05FF => Script::Hebrew,
        0x0600..=0x06FF | 0x0750..=0x077F => Script::Arabic,
        0x0900..=0x097F => Script::Devanagari,
        0x3040..=0x309F => Script::Hiragana,
        0x30A0..=0x30FF => Script::Katakana,
        0x4E00..=0x9FFF => Script::Han,
        0xAC00..=0xD7AF => Script::Hangul,
        _ => Script::Other,
    }
}

/// Tally `Script` counts over a string. Whitespace is ignored.
pub fn script_histogram(text: &str) -> std::collections::HashMap<Script, u32> {
    let mut hist = std::collections::HashMap::new();
    for c in text.chars() {
        if c.is_whitespace() {
            continue;
        }
        *hist.entry(script_of(c)).or_insert(0) += 1;
    }
    hist
}

/// Return the dominant script (most frequent, ignoring `Digit` and `Other`).
/// Returns `None` when no linguistic script characters are present.
pub fn dominant_script(text: &str) -> Option<Script> {
    let hist = script_histogram(text);
    hist.into_iter()
        .filter(|(s, _)| !matches!(s, Script::Digit | Script::Other))
        .max_by_key(|(_, c)| *c)
        .map(|(s, _)| s)
}
