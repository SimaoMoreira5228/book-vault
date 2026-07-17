pub mod asset;
pub mod local;
pub mod s3;

pub use asset::AssetService;
use async_trait::async_trait;
pub use local::LocalFsProvider;
pub use s3::S3Provider;

#[async_trait]
pub trait StorageProvider: Send + Sync {
	async fn put(&self, key: &str, data: &[u8]) -> Result<(), crate::AppError>;
	async fn get(&self, key: &str) -> Result<Vec<u8>, crate::AppError>;
	async fn delete(&self, key: &str) -> Result<(), crate::AppError>;
	async fn exists(&self, key: &str) -> Result<bool, crate::AppError>;
}
