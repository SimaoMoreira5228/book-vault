use async_trait::async_trait;
use crate::ir::BookIr;
use super::Exporter;

pub struct PdfExporter;

#[async_trait]
impl Exporter for PdfExporter {
    async fn export(&self, _ir: &BookIr) -> Result<Vec<u8>, crate::AppError> {
        Err(crate::AppError::Internal("PDF export not implemented yet".to_string()))
    }
}
