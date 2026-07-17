use crate::db::entities::book_ir;
use crate::db::entities::books;
use crate::db::entities::job_queue;
use crate::db::entities::prelude::{BookIr as BookIrEntity, Books};
use crate::ir::{block::Block, span::Span, BookIr, Section};
use crate::storage::AssetService;
use crate::AppError;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
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
    let ir = parse_pdf(state, &book, &raw_bytes).await?;
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

async fn parse_pdf(
    state: &crate::SharedState,
    book: &books::Model,
    data: &[u8],
) -> Result<BookIr, AppError> {
    let dir = tempfile::tempdir()
        .map_err(|e| AppError::Internal(format!("Temp dir failed: {e}")))?;
    let pdf_path = dir.path().join("input.pdf");
    std::fs::write(&pdf_path, data)
        .map_err(|e| AppError::Internal(format!("Temp write failed: {e}")))?;

    let text = pdf_extract::extract_text(&pdf_path)
        .map_err(|e| AppError::Internal(format!("PDF extraction failed: {e}")))?;

    let doc = lopdf::Document::load_mem(data)
        .map_err(|e| AppError::Internal(format!("Failed to load PDF with lopdf: {e}")))?;

    let pages: Vec<(u32, _)> = doc.get_pages().into_iter().collect();
    let page_count = pages.len();
    if page_count == 0 {
        return Err(AppError::Internal("PDF has no pages".into()));
    }

    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let lines_per_page = (lines.len() + page_count - 1) / page_count;

    let mut spine = Vec::new();

    for (idx, (page_num, _object_id)) in pages.iter().enumerate() {
        let start = idx * lines_per_page;
        let end = std::cmp::min(start + lines_per_page, lines.len());

        let mut blocks: Vec<Block> = Vec::new();

        for i in start..end {
            blocks.push(Block::Paragraph(vec![Span::new(lines[i].to_string())]));
        }

        if let Ok(images) = extract_page_images(&doc, *page_num) {
            if !images.is_empty() && !blocks.is_empty() {
                let spacing = (blocks.len() / (images.len() + 1)).max(1);
                for (i, (image_data, mime_type)) in images.iter().enumerate() {
                    match AssetService::store_image(
                        &state.db,
                        state.storage.as_ref(),
                        book.id,
                        image_data,
                        mime_type,
                        "page_image",
                    )
                    .await
                    {
                        Ok(asset_id) => {
                            let pos = std::cmp::min((i + 1) * spacing, blocks.len());
                            blocks.insert(
                                pos,
                                Block::Image {
                                    asset_ref: asset_id,
                                    alt: Some(format!("Page {} image", idx + 1)),
                                },
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to store page {} image: {}",
                                idx + 1,
                                e
                            );
                        }
                    }
                }
            } else if !images.is_empty() {
                for (image_data, mime_type) in &images {
                    match AssetService::store_image(
                        &state.db,
                        state.storage.as_ref(),
                        book.id,
                        image_data,
                        mime_type,
                        "page_image",
                    )
                    .await
                    {
                        Ok(asset_id) => {
                            blocks.push(Block::Image {
                                asset_ref: asset_id,
                                alt: Some(format!("Page {} image", idx + 1)),
                            });
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to store page {} image: {}",
                                idx + 1,
                                e
                            );
                        }
                    }
                }
            }
        }

        if !blocks.is_empty() {
            let title = if idx == 0 {
                book.title.clone().into()
            } else {
                None
            };
            spine.push(Section {
                id: Uuid::now_v7(),
                title,
                sequence_index: idx as u32,
                blocks,
            });
        }
    }

    Ok(BookIr { version: 1, spine })
}

fn extract_page_images(
    doc: &lopdf::Document,
    page_num: u32,
) -> Result<Vec<(Vec<u8>, String)>, AppError> {
    let mut images = Vec::new();

    let page_id = match doc.get_pages().get(&page_num) {
        Some(id) => *id,
        None => return Ok(images),
    };

    let page = match doc.get_object(page_id) {
        Ok(p) => p,
        Err(_) => return Ok(images),
    };

    let page_dict = match page.as_dict() {
        Ok(d) => d,
        Err(_) => return Ok(images),
    };

    let resources_dict = resolve_dictionary(doc, page_dict, b"Resources");
    let resources_dict = match resources_dict {
        Some(d) => d,
        None => return Ok(images),
    };

    let xobject_dict = resolve_dictionary(doc, resources_dict, b"XObject");
    let xobject_dict = match xobject_dict {
        Some(d) => d,
        None => return Ok(images),
    };

    for (_name, obj) in xobject_dict.iter() {
        let object_id = match obj.as_reference() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let object = match doc.get_object(object_id) {
            Ok(o) => o,
            Err(_) => continue,
        };

        let stream = match object.as_stream() {
            Ok(s) => s,
            Err(_) => continue,
        };

        let is_image = stream.dict.get(b"Subtype")
            .ok()
            .and_then(|subtype| subtype.as_name().ok())
            == Some(b"Image");

        if !is_image {
            continue;
        }

        let mime = image_mime_from_dict(&stream.dict);
        images.push((stream.content.clone(), mime));
    }

    Ok(images)
}

fn resolve_dictionary<'a>(
    doc: &'a lopdf::Document,
    dict: &'a lopdf::Dictionary,
    key: &[u8],
) -> Option<&'a lopdf::Dictionary> {
    let obj = dict.get(key).ok()?;
    if let Ok(d) = obj.as_dict() {
        return Some(d);
    }
    if let Ok(id) = obj.as_reference() {
        if let Ok(resolved) = doc.get_object(id) {
            if let Ok(d) = resolved.as_dict() {
                return Some(d);
            }
        }
    }
    None
}

fn image_mime_from_dict(dict: &lopdf::Dictionary) -> String {
    if let Ok(filter) = dict.get(b"Filter") {
        if let Ok(name) = filter.as_name() {
            match name {
                b"DCTDecode" => return "image/jpeg".to_string(),
                b"JPXDecode" => return "image/jp2".to_string(),
                _ => {}
            }
        }
        if let Ok(arr) = filter.as_array() {
            for f in arr {
                if let Ok(name) = f.as_name() {
                    match name {
                        b"DCTDecode" => return "image/jpeg".to_string(),
                        b"JPXDecode" => return "image/jp2".to_string(),
                        _ => {}
                    }
                }
            }
        }
    }
    "application/octet-stream".to_string()
}
