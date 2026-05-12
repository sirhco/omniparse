//! Image format parsers

mod exif;
mod jpeg;
mod png;
mod tiff;
#[cfg(feature = "svg")]
mod svg;
#[cfg(feature = "webp")]
mod webp;

pub use jpeg::JpegParser;
pub use png::PngParser;
pub use tiff::TiffParser;
#[cfg(feature = "svg")]
pub use svg::SvgParser;
#[cfg(feature = "webp")]
pub use webp::WebpParser;

use crate::core::{Content, Metadata};
#[cfg(feature = "ocr")]
use crate::core::MetadataValue;

/// Run OCR on raw image bytes when the `ocr` feature is enabled and the
/// `OMNIPARSE_OCR=classical|ml` runtime gate is set. Populates structured metadata:
///
/// - `ocr_status`: one of `disabled`, `no_text_found`, `error`, `recognized`.
/// - `ocr_applied`: `true` only when text was recognized.
/// - `ocr_confidence`: mean confidence (only for `recognized` and
///   `no_text_found`).
/// - `ocr_regions`: number of candidate regions examined (only for
///   `no_text_found`).
/// - `ocr_error`: human-readable error string (only for `error`).
///
/// Returns `Content::Text(text)` on successful recognition, else
/// `Content::None`. Callers always get the same return shape regardless of
/// whether the `ocr` feature is compiled in.
pub(crate) fn maybe_ocr_content(_data: &[u8], _metadata: &mut Metadata) -> Content {
    #[cfg(feature = "ocr")]
    {
        use crate::ocr::{run_ocr, OcrAttempt};
        match run_ocr(_data) {
            OcrAttempt::Disabled => {
                // Leave metadata untouched so no-OCR behavior matches the
                // feature-disabled build exactly.
            }
            OcrAttempt::NoTextFound { mean_confidence, regions } => {
                _metadata.insert(
                    "ocr_status".to_string(),
                    MetadataValue::Text("no_text_found".into()),
                );
                _metadata.insert(
                    "ocr_applied".to_string(),
                    MetadataValue::Boolean(false),
                );
                _metadata.insert(
                    "ocr_confidence".to_string(),
                    MetadataValue::Float(mean_confidence as f64),
                );
                _metadata.insert(
                    "ocr_regions".to_string(),
                    MetadataValue::Number(regions as i64),
                );
            }
            OcrAttempt::Error(msg) => {
                _metadata.insert(
                    "ocr_status".to_string(),
                    MetadataValue::Text("error".into()),
                );
                _metadata.insert(
                    "ocr_applied".to_string(),
                    MetadataValue::Boolean(false),
                );
                _metadata.insert("ocr_error".to_string(), MetadataValue::Text(msg));
            }
            OcrAttempt::Recognized { text, mean_confidence } => {
                _metadata.insert(
                    "ocr_status".to_string(),
                    MetadataValue::Text("recognized".into()),
                );
                _metadata.insert(
                    "ocr_applied".to_string(),
                    MetadataValue::Boolean(true),
                );
                _metadata.insert(
                    "ocr_confidence".to_string(),
                    MetadataValue::Float(mean_confidence as f64),
                );
                return Content::Text(text);
            }
        }
    }
    Content::None
}
