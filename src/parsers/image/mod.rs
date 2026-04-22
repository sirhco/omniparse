//! Image format parsers

mod exif;
mod jpeg;
mod png;
mod tiff;

pub use jpeg::JpegParser;
pub use png::PngParser;
pub use tiff::TiffParser;
