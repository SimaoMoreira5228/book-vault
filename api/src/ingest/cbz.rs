use crate::db::entities::book_ir;
use crate::db::entities::books;
use crate::db::entities::job_queue;
use crate::db::entities::prelude::{BookIr as BookIrEntity, Books};
use crate::ir::{block::Block, BookIr, Section};
use crate::storage::AssetService;
use crate::AppError;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use std::io::Read;
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
    let ir = parse_cbz(state, &book, &raw_bytes).await?;
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

async fn parse_cbz(
    state: &crate::SharedState,
    book: &books::Model,
    data: &[u8],
) -> Result<BookIr, AppError> {
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| AppError::Internal(format!("Failed to open CBZ archive: {e}")))?;

    let mut entries: Vec<(String, usize)> = Vec::new();
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| AppError::Internal(format!("Failed to read entry {i}: {e}")))?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_string();
        let lower = name.to_lowercase();
        if lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".webp")
        {
            entries.push((name, i));
        }
    }

    entries.sort_by(|a, b| natural_cmp(&a.0, &b.0));

    let mut blocks = Vec::new();
    for (idx, (name, entry_idx)) in entries.iter().enumerate() {
        let mut file = archive
            .by_index(*entry_idx)
            .map_err(|e| AppError::Internal(format!("Failed to read '{name}': {e}")))?;
        let mut image_bytes = Vec::new();
        file.read_to_end(&mut image_bytes)
            .map_err(|e| AppError::Internal(format!("Failed to read '{name}': {e}")))?;

        let mime_type = guess_image_mime(name);
        let asset_id = AssetService::store_image(
            &state.db,
            state.storage.as_ref(),
            book.id,
            &image_bytes,
            mime_type,
            "comic_page",
        )
        .await?;

        blocks.push(Block::Image {
            asset_ref: asset_id,
            alt: Some(format!("Page {}", idx + 1)),
        });
    }

    let spine = vec![Section {
        id: Uuid::now_v7(),
        title: book.title.clone().into(),
        sequence_index: 0,
        blocks,
    }];

    Ok(BookIr { version: 1, spine })
}

fn guess_image_mime(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let a_stem = std::path::Path::new(&a_lower)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&a_lower);
    let b_stem = std::path::Path::new(&b_lower)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&b_lower);
    let a_meta = parse_numeric_parts(a_stem);
    let b_meta = parse_numeric_parts(b_stem);
    if let (Some(a_num), Some(b_num)) = (&a_meta.num, &b_meta.num) {
        if a_num != b_num {
            return a_num.cmp(b_num);
        }
    }
    a_stem.cmp(b_stem)
}

struct NumericMeta {
    num: Option<u64>,
}

fn parse_numeric_parts(s: &str) -> NumericMeta {
    let mut num = 0u64;
    let mut in_digits = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            num = num * 10 + (c as u64);
            in_digits = true;
        } else if in_digits {
            return NumericMeta { num: Some(num) };
        }
    }
    if in_digits {
        NumericMeta { num: Some(num) }
    } else {
        NumericMeta { num: None }
    }
}
