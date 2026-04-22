//! Simple in-memory LRU cache keyed by image-byte hash.
//!
//! Wraps `maybe_ocr` / `run_ocr` callers to avoid redoing OCR on the same
//! bytes during a process. The cache is intentionally bounded and thread-
//! safe; the same image hit multiple times (e.g. in a batch job that
//! dedupes pages) only pays the OCR cost once.
//!
//! Not persistent across process restarts — use a disk cache in the caller
//! if you need that.

use crate::ocr::{OcrAttempt, OcrOutput};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

/// LRU cache with a fixed maximum entry count.
pub struct OcrCache {
    capacity: usize,
    inner: Mutex<Inner>,
}

struct Inner {
    map: HashMap<[u8; 32], OcrAttemptSnapshot>,
    order: VecDeque<[u8; 32]>,
}

/// OcrAttempt-shaped snapshot that also keeps the full `OcrOutput` when
/// recognition succeeded, so a cache hit can return the same value a fresh
/// call would.
#[derive(Clone)]
pub enum OcrAttemptSnapshot {
    Disabled,
    NoTextFound {
        mean_confidence: f32,
        regions: usize,
    },
    Error(String),
    Recognized(OcrOutput),
}

impl OcrCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            inner: Mutex::new(Inner {
                map: HashMap::new(),
                order: VecDeque::new(),
            }),
        }
    }

    /// Compute the hash key for `bytes`. Exposed so callers can share keys
    /// across caches (e.g. a disk cache).
    pub fn key(bytes: &[u8]) -> [u8; 32] {
        #[cfg(feature = "ocr")]
        {
            sha256_like(bytes)
        }
        #[cfg(not(feature = "ocr"))]
        {
            let _ = bytes;
            [0u8; 32]
        }
    }

    pub fn get(&self, key: &[u8; 32]) -> Option<OcrAttemptSnapshot> {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if guard.map.contains_key(key) {
            // LRU bump.
            guard.order.retain(|k| k != key);
            guard.order.push_back(*key);
            return guard.map.get(key).cloned();
        }
        None
    }

    pub fn put(&self, key: [u8; 32], value: OcrAttemptSnapshot) {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if guard.map.insert(key, value).is_none() {
            guard.order.push_back(key);
            while guard.order.len() > self.capacity {
                if let Some(evict) = guard.order.pop_front() {
                    guard.map.remove(&evict);
                }
            }
        } else {
            guard.order.retain(|k| k != &key);
            guard.order.push_back(key);
        }
    }

    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .map(|g| g.map.len())
            .unwrap_or(0)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Convert `OcrAttempt` into a cache-safe snapshot. Errors capture their
/// display-stringified form rather than the original typed error.
pub fn attempt_to_snapshot(attempt: &OcrAttempt) -> OcrAttemptSnapshot {
    match attempt {
        OcrAttempt::Disabled => OcrAttemptSnapshot::Disabled,
        OcrAttempt::NoTextFound { mean_confidence, regions } => {
            OcrAttemptSnapshot::NoTextFound {
                mean_confidence: *mean_confidence,
                regions: *regions,
            }
        }
        OcrAttempt::Error(msg) => OcrAttemptSnapshot::Error(msg.clone()),
        OcrAttempt::Recognized { text, mean_confidence } => {
            OcrAttemptSnapshot::Recognized(OcrOutput {
                text: text.clone(),
                lines: Vec::new(),
                mean_confidence: *mean_confidence,
                detected_script: crate::ocr::script::dominant_script(text),
            })
        }
    }
}

pub fn snapshot_to_attempt(snap: &OcrAttemptSnapshot) -> OcrAttempt {
    match snap {
        OcrAttemptSnapshot::Disabled => OcrAttempt::Disabled,
        OcrAttemptSnapshot::NoTextFound { mean_confidence, regions } => OcrAttempt::NoTextFound {
            mean_confidence: *mean_confidence,
            regions: *regions,
        },
        OcrAttemptSnapshot::Error(msg) => OcrAttempt::Error(msg.clone()),
        OcrAttemptSnapshot::Recognized(out) => OcrAttempt::Recognized {
            text: out.text.clone(),
            mean_confidence: out.mean_confidence,
        },
    }
}

/// Shared process-wide cache. Capacity set from `OMNIPARSE_OCR_CACHE_SIZE`
/// env var (default 64). Returns `None` when `OMNIPARSE_OCR_CACHE=0` is
/// explicitly set — otherwise the cache is on by default for any OCR run.
pub fn shared_cache() -> Option<&'static OcrCache> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Option<OcrCache>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let disabled = std::env::var("OMNIPARSE_OCR_CACHE")
                .map(|v| v == "0" || v.eq_ignore_ascii_case("false"))
                .unwrap_or(false);
            if disabled {
                None
            } else {
                let cap = std::env::var("OMNIPARSE_OCR_CACHE_SIZE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(64);
                Some(OcrCache::new(cap))
            }
        })
        .as_ref()
}

/// FNV-style 32-byte digest. Not cryptographic — we just need fast, fixed-
/// width hashing with low collision risk on typical image byte streams. For
/// a stronger digest users can build their own cache keys.
fn sha256_like(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut state: u64 = 0xcbf2_9ce4_8422_2325;
    let prime: u64 = 0x100_0000_01b3;
    for (i, chunk) in bytes.chunks(8).enumerate() {
        for &b in chunk {
            state = (state ^ b as u64).wrapping_mul(prime);
        }
        let slot = i % 4;
        let pos = slot * 8;
        for (j, byte) in state.to_le_bytes().iter().enumerate() {
            out[pos + j] ^= *byte;
        }
    }
    // Mix length into the digest to reduce cross-length collisions.
    let len = (bytes.len() as u64).to_le_bytes();
    for i in 0..8 {
        out[24 + i] ^= len[i];
    }
    out
}
