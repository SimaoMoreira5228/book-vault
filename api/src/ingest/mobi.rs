use crate::db::entities::book_ir;
use crate::db::entities::books;
use crate::db::entities::job_queue;
use crate::db::entities::prelude::{BookIr as BookIrEntity, Books};
use crate::AppError;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use std::process::Command;
use tempfile::tempdir;
use uuid::Uuid;

pub async fn ingest(state: &crate::SharedState, job: &job_queue::Model) -> Result<(), AppError> {
    let payload = &job.payload;
    let book_id: Uuid = payload["book_id"]
        .as_str()
        .ok_or_else(|| AppError::Internal("Missing book_id in job payload".into()))?
        .parse()
        .map_err(|_| AppError::Internal("Invalid book_id".into()))?;

    let book = Books::find_by_id(book_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Book not found".into()))?;

    let raw_bytes = state.storage.get(&book_id.to_string()).await?;

    let dir = tempdir()
        .map_err(|e| AppError::Internal(format!("Temp dir failed: {e}")))?;
    let input_path = dir.path().join("input.mobi");
    let output_path = dir.path().join("output.epub");

    std::fs::write(&input_path, &raw_bytes)
        .map_err(|e| AppError::Internal(format!("Temp write failed: {e}")))?;

    let calibre_result = Command::new("ebook-convert")
        .arg(&input_path)
        .arg(&output_path)
        .output();

    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();

    match calibre_result {
        Ok(output) if output.status.success() => {
            let epub_bytes = std::fs::read(&output_path)
                .map_err(|e| AppError::Internal(format!("Failed to read converted EPUB: {e}")))?;

            match crate::ingest::epub::parse_epub(&epub_bytes) {
                Ok(ir) => {
                    let payload = crate::ingest::serialize_ir(&ir)?;

                    let existing_ir = BookIrEntity::find()
                        .filter(book_ir::Column::BookId.eq(book_id))
                        .one(&state.db)
                        .await?;

                    if let Some(existing) = existing_ir {
                        let mut active: book_ir::ActiveModel = existing.into();
                        active.payload = Set(payload);
                        active.updated_at = Set(now);
                        BookIrEntity::update(active).exec(&state.db).await?;
                    } else {
                        BookIrEntity::insert(book_ir::ActiveModel {
                            id: Set(Uuid::now_v7()),
                            book_id: Set(book_id),
                            payload: Set(payload),
                            version: Set(1),
                            created_at: Set(now),
                            updated_at: Set(now),
                        })
                        .exec(&state.db)
                        .await?;
                    }

                    let mut active: books::ActiveModel = book.into();
                    active.read_status = Set("reading".to_string());
                    active.updated_at = Set(now);
                    Books::update(active).exec(&state.db).await?;
                }
                Err(e) => {
                    let msg = format!("Calibre converted but EPUB parse failed: {e}");
                    tracing::warn!("{msg}");
                    let mut active: books::ActiveModel = book.into();
                    active.format = Set("mobi_raw".to_string());
                    active.keep_source = Set(Some(true));
                    active.updated_at = Set(now);
                    Books::update(active).exec(&state.db).await?;
                }
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!("ebook-convert failed: {stderr}");
            let mut active: books::ActiveModel = book.into();
            active.format = Set("mobi_raw".to_string());
            active.keep_source = Set(Some(true));
            active.updated_at = Set(now);
            Books::update(active).exec(&state.db).await?;
        }
        Err(e) => {
            tracing::warn!("ebook-convert not available: {e}");
            let mut active: books::ActiveModel = book.into();
            active.format = Set("mobi_raw".to_string());
            active.keep_source = Set(Some(true));
            active.updated_at = Set(now);
            Books::update(active).exec(&state.db).await?;
        }
    }

    Ok(())
}
