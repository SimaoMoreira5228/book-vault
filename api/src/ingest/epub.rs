use async_trait::async_trait;
use crate::ir::BookIr;
use super::IngestionPipeline;

pub struct EpubIngester;

#[async_trait]
impl IngestionPipeline for EpubIngester {
    async fn ingest(&self, _data: &[u8]) -> Result<BookIr, crate::AppError> {
        Err(crate::AppError::Internal("EPUB ingestion not implemented yet".to_string()))
    }
}
