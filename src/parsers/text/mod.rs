//! Text format parsers

mod plain;
mod json;
mod csv;
mod xml;
mod html;
mod css;
mod rtf;
#[cfg(feature = "markdown")]
mod markdown;

pub use plain::PlainTextParser;
pub use json::JsonParser;
pub use csv::CsvParser;
pub use xml::XmlParser;
pub use html::HtmlParser;
pub use css::CssParser;
pub use rtf::RtfParser;
#[cfg(feature = "markdown")]
pub use markdown::MarkdownParser;
