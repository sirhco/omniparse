//! Archive format parsers

mod zip;
mod tar;

pub use self::zip::ZipParser;
pub use self::tar::TarParser;
