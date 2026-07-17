use async_trait::async_trait;
use crate::ir::BookIr;
use super::IngestionPipeline;

pub struct CbzIngester;

#[async_trait]
impl IngestionPipeline for CbzIngester {
    async fn ingest(&self, _data: &[u8]) -> Result<BookIr, crate::AppError> {
        Err(crate::AppError::Internal("CBZ ingestion not implemented yet".to_string()))
    }
}
