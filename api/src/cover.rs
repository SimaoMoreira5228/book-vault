use uuid::Uuid;

use crate::AppError;

const COVER_FILENAMES: &[&str] = &[
	"cover.jpg",
	"cover.jpeg",
	"cover.png",
	"Cover.jpg",
	"Cover.jpeg",
	"Cover.png",
	"OPS/cover.jpg",
	"OPS/cover.jpeg",
	"OPS/cover.png",
	"images/cover.jpg",
	"images/cover.png",
];

pub fn extract_from_epub(
	archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
) -> Result<Option<(Vec<u8>, String)>, AppError> {
	for name in COVER_FILENAMES {
		if let Ok(mut file) = archive.by_name(name) {
			let mut data = Vec::new();
			std::io::Read::read_to_end(&mut file, &mut data)?;
			let mime = if name.ends_with(".png") { "image/png" } else { "image/jpeg" };
			return Ok(Some((data, mime.to_string())));
		}
	}

	let names: Vec<String> = (0..archive.len())
		.filter_map(|i| {
			let f = archive.by_index(i).ok()?;
			let name = f.name().to_lowercase();
			if name.ends_with(".jpg") || name.ends_with(".jpeg") || name.ends_with(".png") {
				Some(name)
			} else {
				None
			}
		})
		.collect();

	if let Some(first) = names.into_iter().next() {
		if let Ok(mut file) = archive.by_name(&first) {
			let mut data = Vec::new();
			std::io::Read::read_to_end(&mut file, &mut data)?;
			let mime = if first.ends_with(".png") { "image/png" } else { "image/jpeg" };
			return Ok(Some((data, mime.to_string())));
		}
	}

	Ok(None)
}

pub fn extract_from_cbz(
	archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
) -> Result<Option<(Vec<u8>, String)>, AppError> {
	for i in 0..archive.len() {
		let mut file = archive.by_index(i)?;
		let name = file.name().to_lowercase();
		if name.ends_with(".jpg") || name.ends_with(".jpeg") || name.ends_with(".png") {
			let mut data = Vec::new();
			std::io::Read::read_to_end(&mut file, &mut data)?;
			let mime = if name.ends_with(".png") { "image/png" } else { "image/jpeg" };
			return Ok(Some((data, mime.to_string())));
		}
	}
	Ok(None)
}

pub async fn store_cover(
	db: &sea_orm::DatabaseConnection,
	storage: &dyn crate::storage::StorageProvider,
	book_id: Uuid,
	data: &[u8],
	mime_type: &str,
) -> Result<(), AppError> {
	crate::storage::AssetService::store_image(db, storage, book_id, data, mime_type, "cover").await?;
	Ok(())
}
