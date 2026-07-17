pub mod epub;
pub mod pdf;
pub mod markdown;

pub use epub::EpubExporter;
pub use pdf::PdfExporter;
pub use markdown::MarkdownExporter;

use async_trait::async_trait;
use crate::ir::BookIr;

#[async_trait]
pub trait Exporter: Send + Sync {
    async fn export(&self, ir: &BookIr) -> Result<Vec<u8>, crate::AppError>;
}
