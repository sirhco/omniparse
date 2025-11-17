//! Document format parsers

mod pdf;
mod docx;
mod odt;

pub use pdf::PdfParser;
pub use docx::DocxParser;
pub use odt::OdtParser;
