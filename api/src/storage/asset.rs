use crate::db::entities::assets;
use crate::db::entities::prelude::Assets;
use crate::AppError;
use sea_orm::{EntityTrait, Set};
use uuid::Uuid;

pub struct AssetService;

impl AssetService {
    pub async fn store_image(
        db: &sea_orm::DatabaseConnection,
        storage: &dyn crate::storage::StorageProvider,
        book_id: Uuid,
        data: &[u8],
        mime_type: &str,
        kind: &str,
    ) -> Result<Uuid, AppError> {
        let asset_id = Uuid::now_v7();
        let key = format!("asset/{}", asset_id);

        let hash = blake3::hash(data).to_hex().to_string();

        storage.put(&key, data).await?;

        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        Assets::insert(assets::ActiveModel {
            id: Set(asset_id),
            book_id: Set(book_id),
            kind: Set(kind.to_string()),
            mime_type: Set(mime_type.to_string()),
            size_bytes: Set(data.len() as i64),
            storage_path: Set(key),
            sha256: Set(hash),
            created_at: Set(now),
        })
        .exec(db)
        .await?;

        Ok(asset_id)
    }

    pub async fn get_image_data(
        storage: &dyn crate::storage::StorageProvider,
        asset_id: Uuid,
    ) -> Result<Vec<u8>, AppError> {
        let key = format!("asset/{}", asset_id);
        storage.get(&key).await
    }

    pub async fn get_asset(
        db: &sea_orm::DatabaseConnection,
        asset_id: Uuid,
    ) -> Result<assets::Model, AppError> {
        Assets::find_by_id(asset_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Asset {asset_id} not found")))
    }

    pub async fn list_assets(
        db: &sea_orm::DatabaseConnection,
        book_id: Uuid,
    ) -> Result<Vec<assets::Model>, AppError> {
        use sea_orm::ColumnTrait;
        use sea_orm::QueryFilter;
        Assets::find()
            .filter(assets::Column::BookId.eq(book_id))
            .all(db)
            .await
            .map_err(AppError::from)
    }
}
