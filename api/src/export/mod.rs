pub mod epub;
pub mod markdown;
pub mod pdf;

use async_trait::async_trait;
pub use epub::EpubExporter;
pub use markdown::MarkdownExporter;
pub use pdf::PdfExporter;

use crate::ir::BookIr;

#[async_trait]
pub trait Exporter: Send + Sync {
	async fn export(&self, ir: &BookIr) -> Result<Vec<u8>, crate::AppError>;
}
