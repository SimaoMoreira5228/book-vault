use async_trait::async_trait;
use crate::ir::BookIr;
use super::IngestionPipeline;

pub struct PdfIngester;

#[async_trait]
impl IngestionPipeline for PdfIngester {
    async fn ingest(&self, _data: &[u8]) -> Result<BookIr, crate::AppError> {
        Err(crate::AppError::Internal("PDF ingestion not implemented yet".to_string()))
    }
}
