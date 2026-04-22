//! Document format parsers

mod pdf;
mod docx;
mod odt;
mod xlsx;
mod pptx;
mod ods;
mod odp;
mod xls;
mod doc;
mod ppt;
#[cfg(feature = "epub")]
mod epub;

pub use pdf::PdfParser;
pub use docx::DocxParser;
pub use odt::OdtParser;
pub use xlsx::XlsxParser;
pub use pptx::PptxParser;
pub use ods::OdsParser;
pub use odp::OdpParser;
pub use xls::XlsParser;
pub use doc::DocParser;
pub use ppt::PptParser;
#[cfg(feature = "epub")]
pub use epub::EpubParser;
