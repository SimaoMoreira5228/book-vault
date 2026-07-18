use std::io::Read;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use tracing::warn;
use uuid::Uuid;

use crate::AppError;
use crate::db::entities::prelude::{BookIr as BookIrEntity, Books};
use crate::db::entities::{book_ir, books, job_queue};
use crate::ir::block::Block;
use crate::ir::span::Span;
use crate::ir::{BookIr, Section};
use crate::storage::AssetService;

struct OpfMeta {
	title: Option<String>,
	author: Option<String>,
	language: Option<String>,
	publisher: Option<String>,
	cover_id: Option<String>,
	cover_href: Option<String>,
}

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
	let cursor = std::io::Cursor::new(raw_bytes.as_slice());
	let mut archive =
		zip::ZipArchive::new(cursor).map_err(|e| AppError::Internal(format!("Failed to open EPUB archive: {e}")))?;

	let (mut ir, meta) = parse_archive(&mut archive)?;
	process_image_blocks(&mut ir, &mut archive, &state.db, &*state.storage, book_id).await?;

	if let Some(ref cover_path) = meta.cover_href {
		if let Ok(mut file) = archive.by_name(cover_path) {
			use std::io::Read;
			let mut data = Vec::new();
			if file.read_to_end(&mut data).is_ok() && !data.is_empty() {
				let mime = guess_image_mime(cover_path);
				let _ = crate::cover::store_cover(&state.db, &*state.storage, book_id, &data, mime).await;
			}
		}
	} else if let Ok(Some((cover_data, mime))) = crate::cover::extract_from_epub(&mut archive) {
		let _ = crate::cover::store_cover(&state.db, &*state.storage, book_id, &cover_data, &mime).await;
	}

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
	if let Some(ref t) = meta.title {
		active.title = Set(t.clone());
	}
	if let Some(ref a) = meta.author {
		active.author = Set(Some(a.clone()));
	}
	if let Some(ref l) = meta.language {
		active.language = Set(Some(l.clone()));
	}
	if let Some(ref p) = meta.publisher {
		active.publisher = Set(Some(p.clone()));
	}
	active.read_status = Set("reading".to_string());
	active.updated_at = Set(now);
	Books::update(active).exec(&state.db).await?;

	Ok(())
}

pub fn parse_epub(data: &[u8]) -> Result<BookIr, AppError> {
	let cursor = std::io::Cursor::new(data);
	let mut archive = zip::ZipArchive::new(cursor).map_err(|e| AppError::Internal(format!("Failed to open EPUB: {e}")))?;
	let (ir, _) = parse_archive(&mut archive)?;
	Ok(ir)
}

fn parse_archive(archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>) -> Result<(BookIr, OpfMeta), AppError> {
	let opf_path = find_opf_path(archive)?;
	let (meta, _manifest, spine_ids) = parse_opf(archive, &opf_path)?;

	let mut spine = Vec::new();
	for (sequence_index, item_id) in spine_ids.iter().enumerate() {
		let href = resolve_href(&opf_path, item_id);
		let content = read_file_from_archive(archive, &href)?;
		let blocks = xhtml_to_blocks(&content, &href)?;
		let section_title = extract_title_from_xhtml(&content);
		spine.push(Section {
			id: Uuid::now_v7(),
			title: section_title,
			sequence_index: sequence_index as u32,
			blocks,
		});
	}

	Ok((BookIr { version: 1, spine }, meta))
}

async fn process_image_blocks(
	ir: &mut BookIr,
	archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
	db: &sea_orm::DatabaseConnection,
	storage: &dyn crate::storage::StorageProvider,
	book_id: Uuid,
) -> Result<(), AppError> {
	for section in &mut ir.spine {
		for block in &mut section.blocks {
			if let Block::Image {
				ref mut asset_ref,
				ref src,
				..
			} = block
			{
				if src.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
					continue;
				}
				if let Some(ref src_path) = src {
					let resolved = normalize_path(&src_path.replace('\\', "/"));
					match archive.by_name(&resolved) {
						Ok(mut file) => {
							use std::io::Read;
							let mut data = Vec::new();
							if file.read_to_end(&mut data).is_err() || data.is_empty() {
								warn!("image read failed: {resolved}");
								continue;
							}
							let mime = guess_image_mime(&resolved);
							match AssetService::store_image(db, storage, book_id, &data, mime, "image").await {
								Ok(aid) => {
									*asset_ref = aid;
								}
								Err(e) => warn!("image store failed {resolved}: {e}"),
							}
						}
						Err(_) => {
							warn!("image not found in archive: {resolved}");
						}
					}
				}
			}
		}
	}
	Ok(())
}

fn normalize_path(path: &str) -> String {
	let mut components: Vec<&str> = Vec::new();
	for segment in path.split('/') {
		match segment {
			"." | "" => continue,
			".." => {
				components.pop();
			}
			_ => components.push(segment),
		}
	}
	components.join("/")
}

fn guess_image_mime(path: &str) -> &'static str {
	let lower = path.to_lowercase();
	if lower.ends_with(".png") {
		"image/png"
	} else if lower.ends_with(".gif") {
		"image/gif"
	} else if lower.ends_with(".webp") {
		"image/webp"
	} else if lower.ends_with(".svg") || lower.ends_with(".svgz") {
		"image/svg+xml"
	} else if lower.ends_with(".avif") {
		"image/avif"
	} else if lower.ends_with(".jpeg") || lower.ends_with(".jpg") || lower.ends_with(".jpe") {
		"image/jpeg"
	} else if lower.ends_with(".tiff") || lower.ends_with(".tif") {
		"image/tiff"
	} else if lower.ends_with(".bmp") {
		"image/bmp"
	} else {
		"image/jpeg"
	}
}

fn find_opf_path(archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>) -> Result<String, AppError> {
	let container_xml = read_file_from_archive(archive, "META-INF/container.xml")?;
	let mut reader = quick_xml::Reader::from_str(&container_xml);
	reader.config_mut().trim_text(true);
	let mut buf = Vec::new();

	loop {
		match reader.read_event_into(&mut buf) {
			Ok(quick_xml::events::Event::Start(ref e)) | Ok(quick_xml::events::Event::Empty(ref e)) => {
				if e.name().as_ref() == b"rootfile" {
					for attr in e.attributes().flatten() {
						if attr.key.as_ref() == b"full-path" {
							return Ok(String::from_utf8_lossy(&attr.value).to_string());
						}
					}
				}
			}
			Ok(quick_xml::events::Event::Eof) => break,
			Err(e) => return Err(AppError::Internal(format!("XML parse error: {e}"))),
			_ => {}
		}
		buf.clear();
	}
	Err(AppError::Internal("No OPF found in EPUB".into()))
}

type OpfResult = (OpfMeta, Vec<(String, String)>, Vec<String>);

fn parse_opf(
	archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
	opf_path: &str,
) -> Result<OpfResult, AppError> {
	use quick_xml::events::Event;
	let opf_xml = read_file_from_archive(archive, opf_path)?;
	let mut reader = quick_xml::Reader::from_str(&opf_xml);
	reader.config_mut().trim_text(true);

	let opf_dir = std::path::Path::new(opf_path).parent().unwrap_or(std::path::Path::new(""));

	let mut manifest: Vec<(String, String)> = Vec::new();
	let mut spine_ids: Vec<String> = Vec::new();
	let mut meta = OpfMeta {
		title: None,
		author: None,
		language: None,
		publisher: None,
		cover_id: None,
		cover_href: None,
	};
	let mut in_metadata = false;
	let mut in_title = false;
	let mut in_creator = false;
	let mut in_language = false;
	let mut in_publisher = false;
	let mut buf = Vec::new();

	loop {
		match reader.read_event_into(&mut buf) {
			Ok(Event::Start(ref e)) => {
				let name = e.name().as_ref().to_vec();
				if name == b"metadata" || name == b"opf:metadata" {
					in_metadata = true;
				}
				if !in_metadata {
					continue;
				}
				match name.as_slice() {
					b"title" | b"dc:title" => in_title = true,
					b"creator" | b"dc:creator" => in_creator = true,
					b"language" | b"dc:language" => in_language = true,
					b"publisher" | b"dc:publisher" => in_publisher = true,
					_ => {}
				}
			}
			Ok(Event::Text(ref t)) => {
				let val = String::from_utf8_lossy(t.as_ref()).trim().to_string();
				if val.is_empty() {
					continue;
				}
				if in_title && meta.title.is_none() {
					meta.title = Some(val.clone());
				}
				if in_creator && meta.author.is_none() {
					let author = val.split(',').next().unwrap_or(&val).trim().to_string();
					meta.author = Some(author);
				}
				if in_language && meta.language.is_none() {
					let lang = val
						.split(|c: char| !c.is_ascii_alphabetic())
						.next()
						.unwrap_or(&val)
						.to_string();
					meta.language = Some(lang.to_lowercase());
				}
				if in_publisher && meta.publisher.is_none() {
					meta.publisher = Some(val);
				}
			}
			Ok(Event::End(ref e)) => {
				let name = e.name().as_ref().to_vec();
				if name == b"metadata" || name == b"opf:metadata" {
					in_metadata = false;
				}
				match name.as_slice() {
					b"title" | b"dc:title" => in_title = false,
					b"creator" | b"dc:creator" => in_creator = false,
					b"language" | b"dc:language" => in_language = false,
					b"publisher" | b"dc:publisher" => in_publisher = false,
					_ => {}
				}
			}
			Ok(Event::Empty(ref e)) => {
				let name = e.name().as_ref().to_vec();
				match name.as_slice() {
					b"meta" => {
						let mut content = None;
						let mut name_attr = None;
						for attr in e.attributes().flatten() {
							match attr.key.as_ref() {
								b"name" => name_attr = Some(String::from_utf8_lossy(&attr.value).to_string()),
								b"content" => content = Some(String::from_utf8_lossy(&attr.value).to_string()),
								_ => {}
							}
						}
						if name_attr.as_deref() == Some("cover") {
							meta.cover_id = content;
						}
					}
					b"item" => {
						let id = e
							.attributes()
							.flatten()
							.find(|a| a.key.as_ref() == b"id")
							.and_then(|a| String::from_utf8(a.value.to_vec()).ok());
						let href = e
							.attributes()
							.flatten()
							.find(|a| a.key.as_ref() == b"href")
							.and_then(|a| String::from_utf8(a.value.to_vec()).ok());
						let mt = e
							.attributes()
							.flatten()
							.find(|a| a.key.as_ref() == b"media-type")
							.and_then(|a| String::from_utf8(a.value.to_vec()).ok());
						if let (Some(id), Some(href), Some(mt)) = (&id, &href, &mt) {
							if mt.starts_with("image/") || mt.contains("xhtml") || mt.contains("html") {
								manifest.push((id.clone(), href.clone()));
							}
						}
					}
					b"itemref" => {
						if let Some(idref) = e
							.attributes()
							.flatten()
							.find(|a| a.key.as_ref() == b"idref")
							.and_then(|a| String::from_utf8(a.value.to_vec()).ok())
						{
							spine_ids.push(idref);
						}
					}
					_ => {}
				}
			}
			Ok(Event::Eof) => break,
			Err(e) => return Err(AppError::Internal(format!("OPF XML error: {e}"))),
			_ => {}
		}
		buf.clear();
	}

	if let Some(ref cid) = meta.cover_id {
		if let Some((_, href)) = manifest.iter().find(|(id, _)| id == cid) {
			meta.cover_href = Some(opf_dir.join(href).to_string_lossy().to_string().replace('\\', "/"));
		}
	}

	let spine: Vec<String> = spine_ids
		.iter()
		.filter_map(|id| manifest.iter().find(|(m_id, _)| m_id == id).map(|(_, href)| href.clone()))
		.collect();

	Ok((meta, manifest, spine))
}

fn resolve_href(opf_path: &str, href: &str) -> String {
	let base = std::path::Path::new(opf_path);
	let parent = base.parent().unwrap_or(std::path::Path::new(""));
	parent.join(href).to_string_lossy().to_string()
}

fn read_file_from_archive(archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, path: &str) -> Result<String, AppError> {
	let path = path.replace('\\', "/");
	let mut file = archive
		.by_name(&path)
		.map_err(|e| AppError::Internal(format!("Failed to read '{path}': {e}")))?;
	let mut content = String::new();
	file.read_to_string(&mut content)
		.map_err(|e| AppError::Internal(format!("Failed to read '{path}': {e}")))?;
	Ok(content)
}

fn extract_title_from_xhtml(content: &str) -> Option<String> {
	let mut reader = quick_xml::Reader::from_str(content);
	reader.config_mut().trim_text(true);
	let mut buf = Vec::new();
	let mut in_title = false;
	loop {
		match reader.read_event_into(&mut buf) {
			Ok(quick_xml::events::Event::Start(ref e)) => {
				if e.name().as_ref() == b"title" {
					in_title = true;
				}
			}
			Ok(quick_xml::events::Event::Text(ref t)) if in_title => {
				return Some(String::from_utf8_lossy(t.as_ref()).to_string());
			}
			Ok(quick_xml::events::Event::End(ref e)) => {
				if e.name().as_ref() == b"title" {
					in_title = false;
				}
			}
			Ok(quick_xml::events::Event::Eof) => break,
			Err(_) => break,
			_ => {}
		}
		buf.clear();
	}
	None
}

fn xhtml_to_blocks(content: &str, doc_path: &str) -> Result<Vec<Block>, AppError> {
	use scraper::{Html, Selector};
	let document = Html::parse_fragment(content);
	let selector = Selector::parse("p, h1, h2, h3, h4, h5, h6, img, blockquote, pre, ol, ul, table, hr, div")
		.map_err(|e| AppError::Internal(format!("Selector error: {e}")))?;

	let doc_dir = std::path::Path::new(doc_path).parent().unwrap_or(std::path::Path::new(""));

	let mut blocks = Vec::new();
	for element in document.select(&selector) {
		let tag = element.value().name();
		let block = match tag {
			"p" => Block::Paragraph(extract_spans(&element)),
			"h1" => Block::Heading {
				level: 1,
				spans: extract_spans(&element),
			},
			"h2" => Block::Heading {
				level: 2,
				spans: extract_spans(&element),
			},
			"h3" => Block::Heading {
				level: 3,
				spans: extract_spans(&element),
			},
			"h4" => Block::Heading {
				level: 4,
				spans: extract_spans(&element),
			},
			"h5" => Block::Heading {
				level: 5,
				spans: extract_spans(&element),
			},
			"h6" => Block::Heading {
				level: 6,
				spans: extract_spans(&element),
			},
			"hr" => Block::HorizontalRule,
			"img" => {
				let raw_src = element.value().attr("src").unwrap_or("");
				let alt = element.value().attr("alt").map(|a| a.to_string());
				let resolved = normalize_path(&doc_dir.join(raw_src).to_string_lossy().to_string().replace('\\', "/"));
				Block::Image {
					asset_ref: Uuid::nil(),
					alt,
					src: Some(resolved),
				}
			}
			_ => continue,
		};
		blocks.push(block);
	}
	Ok(blocks)
}

fn extract_spans(element: &scraper::ElementRef) -> Vec<Span> {
	use scraper::ElementRef;
	let mut spans = Vec::new();
	for node in element.children() {
		match node.value() {
			scraper::node::Node::Text(text) => {
				let t = text.text.to_string();
				if !t.is_empty() {
					spans.push(Span::new(t));
				}
			}
			scraper::node::Node::Element(el) => {
				let tag = el.name.local.as_ref();
				if let Some(child_ref) = ElementRef::wrap(node) {
					spans.extend(extract_spans_recursive(&child_ref, tag, 0, None));
				}
			}
			_ => {}
		}
	}
	spans
}

fn extract_spans_recursive(
	element: &scraper::ElementRef,
	tag: &str,
	inherited_marks: u16,
	inherited_href: Option<String>,
) -> Vec<Span> {
	use scraper::ElementRef;

	let mut marks = inherited_marks;
	let mut href = inherited_href;

	match tag {
		"em" | "i" => marks |= 2,
		"strong" | "b" => marks |= 1,
		"u" => marks |= 4,
		"s" | "strike" | "del" => marks |= 8,
		"sup" => marks |= 16,
		"sub" => marks |= 32,
		"code" | "tt" => marks |= 64,
		"a" => {
			if let Some(h) = element.value().attr("href") {
				href = Some(h.to_string());
			}
		}
		_ => {}
	}

	let mut spans = Vec::new();
	for node in element.children() {
		match node.value() {
			scraper::node::Node::Text(text) => {
				let t = text.text.to_string();
				if !t.is_empty() {
					let mut span = Span::new(t);
					span.marks = marks;
					span.href = href.clone();
					spans.push(span);
				}
			}
			scraper::node::Node::Element(el) => {
				let child_tag = el.name.local.as_ref();
				if let Some(child_ref) = ElementRef::wrap(node) {
					spans.extend(extract_spans_recursive(&child_ref, child_tag, marks, href.clone()));
				}
			}
			_ => {}
		}
	}
	spans
}
