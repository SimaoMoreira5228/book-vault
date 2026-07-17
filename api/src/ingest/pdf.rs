use crate::db::entities::book_ir;
use crate::db::entities::books;
use crate::db::entities::job_queue;
use crate::db::entities::prelude::{BookIr as BookIrEntity, Books};
use crate::ir::{block::Block, span::Span, BookIr, Section};
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
    let ir = parse_pdf(&raw_bytes, &book.title)?;
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

fn parse_pdf(data: &[u8], title: &str) -> Result<BookIr, AppError> {
    let dir = tempfile::tempdir()
        .map_err(|e| AppError::Internal(format!("Temp dir failed: {e}")))?;
    let pdf_path = dir.path().join("input.pdf");
    std::fs::write(&pdf_path, data)
        .map_err(|e| AppError::Internal(format!("Temp write failed: {e}")))?;
    let text = pdf_extract::extract_text(&pdf_path)
        .map_err(|e| AppError::Internal(format!("PDF extraction failed: {e}")))?;

    let mut all_blocks = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        all_blocks.push(Block::Paragraph(vec![Span::new(trimmed.to_string())]));
    }

    let spine = vec![Section {
        id: Uuid::now_v7(),
        title: Some(title.to_string()),
        sequence_index: 0,
        blocks: all_blocks,
    }];

    Ok(BookIr { version: 1, spine })
}
