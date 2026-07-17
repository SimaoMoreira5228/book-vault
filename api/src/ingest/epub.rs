use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::AppError;
use crate::db::entities::prelude::{BookIr as BookIrEntity, Books};
use crate::db::entities::{book_ir, books, job_queue};
use crate::ir::block::Block;
use crate::ir::span::Span;
use crate::ir::{BookIr, Section};

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
	let ir = parse_epub(&raw_bytes)?;
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

	let cursor = std::io::Cursor::new(raw_bytes.as_slice());
	if let Ok(mut archive) = zip::ZipArchive::new(cursor) {
		if let Ok(Some((cover_data, mime))) = crate::cover::extract_from_epub(&mut archive) {
			let _ = crate::cover::store_cover(&state.db, &*state.storage, book_id, &cover_data, &mime).await;
		}
	}

	let mut active: books::ActiveModel = book.into();
	active.read_status = Set("reading".to_string());
	active.updated_at = Set(now);
	Books::update(active).exec(&state.db).await?;

	Ok(())
}

pub fn parse_epub(data: &[u8]) -> Result<BookIr, AppError> {
	let cursor = std::io::Cursor::new(data);
	let mut archive =
		zip::ZipArchive::new(cursor).map_err(|e| AppError::Internal(format!("Failed to open EPUB archive: {e}")))?;

	let opf_path = find_opf_path(&mut archive)?;
	let (_metadata_xml, _manifest, spine_ids) = parse_opf(&mut archive, &opf_path)?;

	let mut spine = Vec::new();
	let mut section_index = 0u32;
	for item_id in &spine_ids {
		let href = resolve_href(&opf_path, item_id);
		let content = read_file_from_archive(&mut archive, &href)?;
		let blocks = xhtml_to_blocks(&content)?;
		let title = extract_title_from_xhtml(&content);
		spine.push(Section {
			id: Uuid::now_v7(),
			title,
			sequence_index: section_index,
			blocks,
		});
		section_index += 1;
	}

	Ok(BookIr { version: 1, spine })
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

fn parse_opf(
	archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
	opf_path: &str,
) -> Result<(String, Vec<(String, String)>, Vec<String>), AppError> {
	use quick_xml::events::Event;
	let opf_xml = read_file_from_archive(archive, opf_path)?;
	let mut reader = quick_xml::Reader::from_str(&opf_xml);
	reader.config_mut().trim_text(true);

	let mut manifest: Vec<(String, String)> = Vec::new();
	let mut spine_ids: Vec<String> = Vec::new();
	let mut in_manifest = false;
	let mut in_spine = false;
	let mut buf = Vec::new();

	loop {
		match reader.read_event_into(&mut buf) {
			Ok(Event::Start(ref e)) => {
				let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
				match name.as_str() {
					"manifest" | "opf:manifest" => in_manifest = true,
					"spine" | "opf:spine" => in_spine = true,
					_ => {}
				}
			}
			Ok(Event::End(ref e)) => {
				let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
				match name.as_str() {
					"manifest" | "opf:manifest" => in_manifest = false,
					"spine" | "opf:spine" => in_spine = false,
					_ => {}
				}
			}
			Ok(Event::Empty(ref e)) => {
				let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
				if in_manifest && name == "item" {
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
						if mt.contains("xhtml") || mt.contains("html") {
							manifest.push((id.clone(), href.clone()));
						}
					}
				}
				if in_spine && name == "itemref" {
					if let Some(idref) = e
						.attributes()
						.flatten()
						.find(|a| a.key.as_ref() == b"idref")
						.and_then(|a| String::from_utf8(a.value.to_vec()).ok())
					{
						spine_ids.push(idref);
					}
				}
			}
			Ok(Event::Eof) => break,
			Err(e) => return Err(AppError::Internal(format!("OPF XML error: {e}"))),
			_ => {}
		}
		buf.clear();
	}

	let spine: Vec<String> = spine_ids
		.iter()
		.filter_map(|id| manifest.iter().find(|(m_id, _)| m_id == id).map(|(_, href)| href.clone()))
		.collect();

	Ok((String::new(), manifest, spine))
}

fn resolve_href(opf_path: &str, href: &str) -> String {
	let base = std::path::Path::new(opf_path);
	let parent = base.parent().unwrap_or(std::path::Path::new(""));
	parent.join(href).to_string_lossy().to_string()
}

fn read_file_from_archive(archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, path: &str) -> Result<String, AppError> {
	let path = path.replace('\\', "/");
	use std::io::Read;
	let mut file = archive
		.by_name(&path)
		.map_err(|e| AppError::Internal(format!("Failed to read '{path}' from archive: {e}")))?;
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
				let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
				if name == "title" {
					in_title = true;
				}
			}
			Ok(quick_xml::events::Event::Text(ref t)) if in_title => {
				let text = String::from_utf8_lossy(t.as_ref()).to_string();
				return Some(text);
			}
			Ok(quick_xml::events::Event::End(ref e)) => {
				let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
				if name == "title" {
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

fn xhtml_to_blocks(content: &str) -> Result<Vec<Block>, AppError> {
	use scraper::{Html, Selector};
	let document = Html::parse_fragment(content);
	let selector = Selector::parse("p, h1, h2, h3, h4, h5, h6, img, blockquote, pre, ol, ul, table, hr, div")
		.map_err(|e| AppError::Internal(format!("Selector error: {e}")))?;

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
				let t = text.text.trim();
				if !t.is_empty() {
					spans.push(Span::new(t.to_string()));
				}
			}
			scraper::node::Node::Element(el) => {
				let tag = el.name.local.as_ref();
				let child_ref = ElementRef::wrap(node);
				let text = child_ref
					.map(|r| r.text().collect::<String>().trim().to_string())
					.unwrap_or_default();
				if text.is_empty() {
					continue;
				}

				let href = el
					.attrs
					.iter()
					.find(|(a, _)| a.local.as_ref() == "href")
					.map(|(_, v)| v.to_string());

				let mut span = Span::new(text);
				span.href = href.clone();
				match tag {
					"em" | "i" => span.marks |= 2,
					"strong" | "b" => span.marks |= 1,
					"u" => span.marks |= 4,
					"s" | "strike" | "del" => span.marks |= 8,
					"sup" => span.marks |= 16,
					"sub" => span.marks |= 32,
					"code" | "tt" => span.marks |= 64,
					"a" => {
						span.href = href;
					}
					_ => {}
				}
				spans.push(span);
			}
			_ => {}
		}
	}
	spans
}
