use std::io::Read;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::db::entities::prelude::{BookIr as BookIrEntity, Books};
use crate::db::entities::{book_ir, books, job_queue};
use crate::ir::block::Block;
use crate::ir::{BookIr, Section};
use crate::storage::AssetService;
use crate::{AppError, SharedState};

const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_PAGE_COUNT: usize = 2000;
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

pub async fn ingest(state: &SharedState, job: &job_queue::Model) -> Result<(), AppError> {
	let book_id = job.payload["book_id"]
		.as_str()
		.ok_or_else(|| AppError::Internal("Missing book_id".into()))?
		.parse::<Uuid>()
		.map_err(|_| AppError::Internal("Invalid book_id".into()))?;

	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

	let raw = state.storage.get(&book_id.to_string()).await?;
	let is_rar =
		raw.len() > 6 && (&raw[..6] == b"Rar!\x1a\x07\x00" || raw[..7] == *b"Rar!\x1a\x07\x01");

	let entries = if is_rar { extract_rar_entries(&raw).await? } else { extract_seven_z_entries(&raw).await? };

	let ir = build_ir(&book, state, entries).await?;
	let payload = super::serialize_ir(&ir)?;
	let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();

	let existing = BookIrEntity::find()
		.filter(book_ir::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?;
	if let Some(e) = existing {
		let mut a: book_ir::ActiveModel = e.into();
		a.payload = Set(payload);
		a.updated_at = Set(now);
		BookIrEntity::update(a).exec(&state.db).await?;
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

	let mut a: books::ActiveModel = book.into();
	a.read_status = Set("reading".to_string());
	a.updated_at = Set(now);
	Books::update(a).exec(&state.db).await?;
	Ok(())
}

fn is_image(name: &str) -> bool {
	let l = name.to_lowercase();
	l.ends_with(".png") || l.ends_with(".jpg") || l.ends_with(".jpeg") || l.ends_with(".webp")
}

async fn extract_rar_entries(data: &[u8]) -> Result<Vec<(String, Vec<u8>)>, AppError> {
	let owned = data.to_vec();
	tokio::task::spawn_blocking(move || -> Result<Vec<(String, Vec<u8>)>, AppError> {
		let tmp = NamedTempFile::new().map_err(|e| AppError::Internal(format!("tempfile: {e}")))?;
		std::fs::write(tmp.path(), &owned).map_err(|e| AppError::Internal(format!("write: {e}")))?;

		let mut archive = unrar::Archive::new(tmp.path())
			.open_for_processing()
			.map_err(|e| AppError::Internal(format!("RAR open: {e}")))?;

		let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
		let mut total: u64 = 0;

		loop {
			let header = match archive.read_header() {
				Ok(h) => h,
				Err(e) => return Err(AppError::Internal(format!("RAR header: {e}"))),
			};
			let Some(entry) = header else { break };
			let name = entry.entry().filename.to_string_lossy().to_string();
			if !is_image(&name) {
				archive = entry.skip().map_err(|e| AppError::Internal(format!("skip: {e}")))?;
				continue;
			}
			if entries.len() >= MAX_PAGE_COUNT {
				return Err(AppError::BadRequest(format!("CBR exceeds {MAX_PAGE_COUNT} pages")));
			}
			let (d, next) = entry.read().map_err(|e| AppError::Internal(format!("RAR read '{name}': {e}")))?;
			let sz = d.len() as u64;
			if sz > MAX_FILE_SIZE {
				return Err(AppError::BadRequest(format!("'{name}' exceeds {MAX_FILE_SIZE} bytes")));
			}
			total += sz;
			if total > MAX_TOTAL_BYTES {
				return Err(AppError::BadRequest("CBR exceeds max total size".into()));
			}
			entries.push((name, d));
			archive = next;
		}
		if entries.is_empty() {
			return Err(AppError::BadRequest("No images in CBR".into()));
		}
		entries.sort_by(|a, b| natural_cmp(&a.0, &b.0));
		Ok(entries)
	})
	.await
	.map_err(|e| AppError::Internal(format!("RAR spawn: {e}")))?
}

async fn extract_seven_z_entries(data: &[u8]) -> Result<Vec<(String, Vec<u8>)>, AppError> {
	let owned = data.to_vec();
	tokio::task::spawn_blocking(move || -> Result<Vec<(String, Vec<u8>)>, AppError> {
		use sevenz_rust::{Password, SevenZArchiveEntry, SevenZReader};
		use std::io::Cursor;

		let cursor = Cursor::new(&owned);
		let mut reader = SevenZReader::new(cursor, owned.len() as u64, Password::empty())
			.map_err(|e| AppError::Internal(format!("7z open: {e}")))?;

		let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
		let mut total: u64 = 0;

		let r: Result<(), sevenz_rust::Error> = reader.for_each_entries(&mut |entry: &SevenZArchiveEntry, rdr: &mut dyn Read| {
			let name = entry.name().to_string();
			if !is_image(&name) { return Ok(true); }
			if entries.len() >= MAX_PAGE_COUNT {
				return Err(sevenz_rust::Error::Unsupported("too many pages".into()));
			}
			let mut buf = Vec::new();
			rdr.read_to_end(&mut buf).map_err(|e| sevenz_rust::Error::Io(e, "read".into()))?;
			let sz = buf.len() as u64;
			if sz > MAX_FILE_SIZE {
				return Err(sevenz_rust::Error::Unsupported(format!("'{name}' too large").into()));
			}
			total += sz;
			if total > MAX_TOTAL_BYTES {
				return Err(sevenz_rust::Error::Unsupported("total too large".into()));
			}
			entries.push((name, buf));
			Ok(true)
		});
		r.map_err(|e| AppError::Internal(format!("7z iterate: {e}")))?;

		if entries.is_empty() { return Err(AppError::BadRequest("No images in CB7".into())); }
		entries.sort_by(|a, b| natural_cmp(&a.0, &b.0));
		Ok(entries)
	})
	.await
	.map_err(|e| AppError::Internal(format!("7z spawn: {e}")))?
}

async fn build_ir(book: &books::Model, state: &SharedState, entries: Vec<(String, Vec<u8>)>) -> Result<BookIr, AppError> {
	let mut blocks = Vec::new();
	for (idx, (name, bytes)) in entries.iter().enumerate() {
		let mime = guess_mime(name);
		let aid = AssetService::store_image(&state.db, state.storage.as_ref(), book.id, bytes, &mime, "comic_page").await?;
		blocks.push(Block::Image { asset_ref: aid, alt: Some(format!("Page {}", idx + 1)), src: None });
	}
	Ok(BookIr { version: 1, spine: vec![Section { id: Uuid::now_v7(), title: Some(book.title.clone()), sequence_index: 0, blocks }] })
}

fn guess_mime(name: &str) -> String {
	let l = name.to_lowercase();
	if l.ends_with(".png") { "image/png".into() }
	else if l.ends_with(".jpg") || l.ends_with(".jpeg") { "image/jpeg".into() }
	else if l.ends_with(".webp") { "image/webp".into() }
	else { "application/octet-stream".into() }
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
	let a_lower = a.to_lowercase();
	let b_lower = b.to_lowercase();
	let a_stem = std::path::Path::new(&a_lower).file_stem().and_then(|s| s.to_str()).unwrap_or(&a_lower);
	let b_stem = std::path::Path::new(&b_lower).file_stem().and_then(|s| s.to_str()).unwrap_or(&b_lower);
	let a_num = a_stem.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u64>().ok();
	let b_num = b_stem.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u64>().ok();
	match (a_num, b_num) {
		(Some(an), Some(bn)) if an != bn => an.cmp(&bn),
		_ => a_stem.cmp(b_stem),
	}
}
