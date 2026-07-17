use async_trait::async_trait;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;

use super::StorageProvider;
use crate::config::S3Config;

pub struct S3Provider {
    client: Client,
    bucket: String,
}

impl S3Provider {
    pub async fn new(config: &S3Config) -> Result<Self, crate::AppError> {
        let credentials = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "bookvault",
        );

        let mut builder = aws_sdk_s3::Config::builder()
            .region(Region::new(config.region.clone()))
            .credentials_provider(credentials);

        if !config.endpoint.is_empty() {
            builder = builder.endpoint_url(&config.endpoint);
        }

        if config.path_style {
            builder = builder.force_path_style(true);
        }

        let sdk_config = builder.build();
        let client = Client::from_conf(sdk_config);

        Ok(Self {
            client,
            bucket: config.bucket.clone(),
        })
    }
}

#[async_trait]
impl StorageProvider for S3Provider {
    async fn put(&self, key: &str, data: &[u8]) -> Result<(), crate::AppError> {
        let body = ByteStream::from(data.to_vec());
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await
            .map_err(|e| crate::AppError::Internal(format!("S3 put failed: {e}")))?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, crate::AppError> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| crate::AppError::NotFound(format!("S3 key not found: {e}")))?;

        let data = output
            .body
            .collect()
            .await
            .map_err(|e| crate::AppError::Internal(format!("S3 read failed: {e}")))?
            .into_bytes()
            .to_vec();

        Ok(data)
    }

    async fn delete(&self, key: &str) -> Result<(), crate::AppError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| crate::AppError::Internal(format!("S3 delete failed: {e}")))?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, crate::AppError> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                    Ok(false)
                } else {
                    Err(crate::AppError::Internal(format!(
                        "S3 exists check failed: {e}"
                    )))
                }
            }
        }
    }
}
