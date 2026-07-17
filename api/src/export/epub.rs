use async_trait::async_trait;
use crate::ir::BookIr;
use super::Exporter;

pub struct EpubExporter;

#[async_trait]
impl Exporter for EpubExporter {
    async fn export(&self, _ir: &BookIr) -> Result<Vec<u8>, crate::AppError> {
        Err(crate::AppError::Internal("EPUB export not implemented yet".to_string()))
    }
}
