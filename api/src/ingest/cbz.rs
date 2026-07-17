use crate::db::entities::book_ir;
use crate::db::entities::books;
use crate::db::entities::job_queue;
use crate::db::entities::prelude::{BookIr as BookIrEntity, Books};
use crate::ir::{block::Block, BookIr, Section};
use crate::AppError;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

pub async fn ingest(state: &crate::SharedState, job: &job_queue::Model) -> Result<(), AppError> {
    let payload = &job.payload;
    let book_id: Uuid = payload["book_id"]
        .as_str()
        .ok_or_else(|| AppError::Internal("Missing book_id".into()))?
        .parse()
        .map_err(|_| AppError::Internal("Invalid book_id".into()))?;

    let book = Books::find_by_id(book_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Book not found".into()))?;

    let raw_bytes = state.storage.get(&book_id.to_string()).await?;
    let ir = parse_cbz(&raw_bytes, &book.title)?;
    let payload = crate::ingest::serialize_ir(&ir)?;

    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
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

    Ok(())
}

fn parse_cbz(data: &[u8], title: &str) -> Result<BookIr, AppError> {
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| AppError::Internal(format!("Failed to open CBZ archive: {e}")))?;

    let mut blocks = Vec::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i)
            .map_err(|e| AppError::Internal(format!("Failed to read page {i}: {e}")))?;
        if file.is_dir() { continue; }

        let name = file.name().to_lowercase();
        if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg") || name.ends_with(".webp") {
            blocks.push(Block::Image {
                asset_ref: Uuid::nil(),
                alt: Some(format!("Page {}", i + 1)),
            });
        }
    }

    let spine = vec![Section {
        id: Uuid::now_v7(),
        title: Some(title.to_string()),
        sequence_index: 0,
        blocks,
    }];

    Ok(BookIr { version: 1, spine })
}
