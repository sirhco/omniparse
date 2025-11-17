//! Text format parsers

mod plain;
mod json;
mod csv;
mod xml;

pub use plain::PlainTextParser;
pub use json::JsonParser;
pub use csv::CsvParser;
pub use xml::XmlParser;
