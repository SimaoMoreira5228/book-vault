pub mod cbz;
pub mod epub;
pub mod mobi;
pub mod pdf;

use async_trait::async_trait;

use crate::ir::BookIr;

#[async_trait]
pub trait IngestionPipeline: Send + Sync {
	async fn ingest(&self, data: &[u8]) -> Result<BookIr, crate::AppError>;
}

pub fn compress(data: &[u8]) -> Vec<u8> {
	zstd::encode_all(data, 3).unwrap_or_else(|_| data.to_vec())
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, crate::AppError> {
	zstd::decode_all(data).map_err(|e| crate::AppError::Internal(format!("Failed to decompress: {}", e)))
}

pub fn serialize_ir(ir: &BookIr) -> Result<Vec<u8>, crate::AppError> {
	let payload = rmp_serde::to_vec(ir).map_err(|e| crate::AppError::Internal(format!("Failed to serialize IR: {}", e)))?;
	Ok(compress(&payload))
}

pub fn deserialize_ir(data: &[u8]) -> Result<BookIr, crate::AppError> {
	let decompressed = decompress(data)?;
	let ir: BookIr = rmp_serde::from_slice(&decompressed)
		.map_err(|e| crate::AppError::Internal(format!("Failed to deserialize IR: {}", e)))?;
	Ok(ir)
}
