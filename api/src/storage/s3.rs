use async_trait::async_trait;
use super::StorageProvider;

pub struct S3Provider;

impl S3Provider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl StorageProvider for S3Provider {
    async fn put(&self, _key: &str, _data: &[u8]) -> Result<(), crate::AppError> {
        Err(crate::AppError::Internal("S3 not implemented yet".to_string()))
    }

    async fn get(&self, _key: &str) -> Result<Vec<u8>, crate::AppError> {
        Err(crate::AppError::Internal("S3 not implemented yet".to_string()))
    }

    async fn delete(&self, _key: &str) -> Result<(), crate::AppError> {
        Err(crate::AppError::Internal("S3 not implemented yet".to_string()))
    }

    async fn exists(&self, _key: &str) -> Result<bool, crate::AppError> {
        Err(crate::AppError::Internal("S3 not implemented yet".to_string()))
    }
}
