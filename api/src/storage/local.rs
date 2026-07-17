use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs;

use super::StorageProvider;

pub struct LocalFsProvider {
	base_path: PathBuf,
}

impl LocalFsProvider {
	pub fn new(base_path: PathBuf) -> Self {
		Self { base_path }
	}
}

#[async_trait]
impl StorageProvider for LocalFsProvider {
	async fn put(&self, key: &str, data: &[u8]) -> Result<(), crate::AppError> {
		let path = self.base_path.join(key);
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent)
				.await
				.map_err(|e| crate::AppError::Internal(format!("Failed to create directory: {}", e)))?;
		}
		fs::write(&path, data)
			.await
			.map_err(|e| crate::AppError::Internal(format!("Failed to write file: {}", e)))?;
		Ok(())
	}

	async fn get(&self, key: &str) -> Result<Vec<u8>, crate::AppError> {
		let path = self.base_path.join(key);
		fs::read(&path)
			.await
			.map_err(|e| crate::AppError::NotFound(format!("Storage file not found: {}", e)))
	}

	async fn delete(&self, key: &str) -> Result<(), crate::AppError> {
		let path = self.base_path.join(key);
		fs::remove_file(&path)
			.await
			.map_err(|e| crate::AppError::Internal(format!("Failed to delete file: {}", e)))?;
		Ok(())
	}

	async fn exists(&self, key: &str) -> Result<bool, crate::AppError> {
		let path = self.base_path.join(key);
		Ok(path.exists())
	}
}
