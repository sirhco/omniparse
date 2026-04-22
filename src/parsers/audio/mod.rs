//! Audio format parsers.

#[cfg(feature = "mp3")]
mod mp3;

#[cfg(feature = "mp3")]
pub use mp3::Mp3Parser;
