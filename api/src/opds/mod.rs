use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use quick_xml::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;
use uuid::Uuid;

use crate::db::entities::prelude::*;
use crate::db::entities::{books, shelf_entries};
use crate::{AppError, SharedState};

#[derive(Deserialize)]
pub struct SearchQuery {
	q: Option<String>,
}

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/", get(root_feed))
		.route("/books", get(books_feed))
		.route("/shelves/{id}", get(shelf_feed))
		.route("/search", get(search_feed))
}

fn atom_response(xml: String) -> Response {
	let headers = [(header::CONTENT_TYPE, HeaderValue::from_static("application/atom+xml;charset=utf-8"))];
	(StatusCode::OK, headers, xml).into_response()
}

async fn root_feed(State(state): State<SharedState>) -> Result<Response, AppError> {
	let xml = generate_root_catalog(&state).await?;
	Ok(atom_response(xml))
}

async fn books_feed(
	State(state): State<SharedState>,
	Query(params): Query<SearchQuery>,
) -> Result<Response, AppError> {
	let xml = generate_acquisition_feed(&state, None, params.q.as_deref()).await?;
	Ok(atom_response(xml))
}

async fn shelf_feed(
	State(state): State<SharedState>,
	Path(shelf_id): Path<Uuid>,
) -> Result<Response, AppError> {
	let shelf = Shelves::find_by_id(shelf_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Shelf not found".into()))?;

	let book_ids: Vec<Uuid> = ShelfEntries::find()
		.filter(shelf_entries::Column::ShelfId.eq(shelf_id))
		.all(&state.db)
		.await?
		.into_iter()
		.map(|se| se.book_id)
		.collect();

	let xml = generate_acquisition_feed_with_ids(
		&state,
		Some(&shelf.name),
		Some("Shelf"),
		&book_ids,
	)
	.await?;
	Ok(atom_response(xml))
}

async fn search_feed(
	State(state): State<SharedState>,
	Query(params): Query<SearchQuery>,
) -> Result<Response, AppError> {
	let q = params.q.unwrap_or_default();
	let xml = generate_acquisition_feed(&state, None, Some(&q)).await?;
	Ok(atom_response(xml))
}

async fn generate_root_catalog(state: &SharedState) -> Result<String, AppError> {
	let shelf_count = Shelves::find().count(&state.db).await.unwrap_or(0);
	let book_count = Books::find().count(&state.db).await.unwrap_or(0);

	let mut w = Writer::new_with_indent(Vec::new(), b' ', 2);
	write_decl(&mut w);

	let mut feed = BytesStart::new("feed");
	feed.push_attribute(("xmlns", "http://www.w3.org/2005/Atom"));
	feed.push_attribute(("xmlns:opds", "http://opds-spec.org/2010/catalog"));
	write_event(&mut w, Event::Start(feed));

	write_text_elem(&mut w, "id", "urn:bookvault:opds");
	write_text_elem(&mut w, "title", "BookVault");
	write_text_elem(&mut w, "updated", &chrono::Utc::now().to_rfc3339());

	write_text_elem(&mut w, "author", concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION")));

	write_link(&mut w, "/opds/books", "subsection", "All Books", Some("application/atom+xml;profile=opds-catalog;kind=acquisition"));
	write_link(&mut w, "/opds/search", "subsection", "Search", Some("application/atom+xml;profile=opds-catalog;kind=acquisition"));

	if shelf_count > 0 {
		write_link(&mut w, "/opds/shelves", "subsection", "Shelves", Some("application/atom+xml;profile=opds-catalog;kind=navigation"));
	}

	let mut entry_xml = String::new();
	entry_xml.push_str(&format!("<title>All Books ({})</title>", book_count));
	entry_xml.push_str("<id>urn:bookvault:opds:books</id>");
	entry_xml.push_str(&format!("<updated>{}</updated>", chrono::Utc::now().to_rfc3339()));
	entry_xml.push_str(r#"<link href="/opds/books" rel="http://opds-spec.org/subsections" type="application/atom+xml;profile=opds-catalog;kind=acquisition"/>"#);
	write_content(&mut w, "entry", &entry_xml);

	write_event(&mut w, Event::End(BytesEnd::new("feed")));
	String::from_utf8(w.into_inner()).map_err(|e| AppError::Internal(format!("OPDS XML: {}", e)))
}

async fn generate_acquisition_feed(state: &SharedState, shelf_name: Option<&str>, search: Option<&str>) -> Result<String, AppError> {
	let mut query = Books::find().order_by_desc(books::Column::UpdatedAt).limit(200);

	if let Some(q) = search {
		if !q.is_empty() {
			let pattern = format!("%{}%", q);
			query = query.filter(
				sea_orm::Condition::any()
					.add(books::Column::Title.like(&pattern))
					.add(books::Column::Author.like(&pattern)),
			);
		}
	}

	let all_books = query.all(&state.db).await?;
	let ids: Vec<Uuid> = all_books.iter().map(|b| b.id).collect();
	generate_acquisition_feed_with_ids(state, shelf_name, search, &ids).await
}

async fn generate_acquisition_feed_with_ids(state: &SharedState, title_override: Option<&str>, subtitle: Option<&str>, book_ids: &[Uuid]) -> Result<String, AppError> {
	let mut w = Writer::new_with_indent(Vec::new(), b' ', 2);
	write_decl(&mut w);

	let mut feed = BytesStart::new("feed");
	feed.push_attribute(("xmlns", "http://www.w3.org/2005/Atom"));
	feed.push_attribute(("xmlns:opds", "http://opds-spec.org/2010/catalog"));
	feed.push_attribute(("xmlns:dcterms", "http://purl.org/dc/terms/"));
	write_event(&mut w, Event::Start(feed));

	let title = title_override.unwrap_or("Books");
	let title_full = if let Some(sub) = subtitle {
		format!("{} - {} - BookVault", title, sub)
	} else {
		format!("{} - BookVault", title)
	};
	write_text_elem(&mut w, "title", &title_full);
	write_text_elem(&mut w, "id", "urn:bookvault:opds:books");
	write_text_elem(&mut w, "updated", &chrono::Utc::now().to_rfc3339());
	write_link(&mut w, "/opds", "start", "Home", Some("application/atom+xml;profile=opds-catalog;kind=navigation"));

	for book_id in book_ids {
		let book = Books::find_by_id(*book_id).one(&state.db).await?;
		if let Some(b) = book {
			let format_links = match b.format.as_str() {
				"epub" => vec![("application/epub+zip", "epub")],
				"pdf" => vec![("application/pdf", "pdf")],
				"mobi_raw" => vec![("application/x-mobipocket-ebook", "mobi")],
				"native" | "bvir" => vec![
					("application/epub+zip", "epub"),
					("application/pdf", "pdf"),
					("text/markdown", "md"),
				],
				"cbz" => vec![("application/x-cbz", "cbz")],
				_ => vec![("application/octet-stream", "bvir")],
			};

			let mut entry = String::new();
			entry.push_str(&format!("<title>{}</title>", escape_xml(&b.title)));
			entry.push_str(&format!("<id>urn:bookvault:book:{}:opds</id>", b.id));
			entry.push_str(&format!("<updated>{}</updated>", b.updated_at.to_rfc3339()));

			if let Some(ref author) = b.author {
				entry.push_str(&format!("<author><name>{}</name></author>", escape_xml(author)));
			}

			if let Some(ref content_str) = b.isbn {
				if !content_str.is_empty() {
					entry.push_str(&format!("<dcterms:identifier>{}</dcterms:identifier>", escape_xml(content_str)));
				}
			}

			if let Some(ref lang) = b.language {
				if !lang.is_empty() {
					entry.push_str(&format!("<dcterms:language>{}</dcterms:language>", escape_xml(lang)));
				}
			}

			entry.push_str(&format!(
				r#"<link href="/api/v1/books/{id}/cover" rel="http://opds-spec.org/image" type="image/jpeg"/>"#,
				id = b.id
			));

			for (mime, ext) in &format_links {
				entry.push_str(&format!(
					r#"<link href="/api/v1/books/{id}/export?format={ext}" rel="http://opds-spec.org/acquisition" type="{mime}"/>"#,
					id = b.id,
					ext = ext,
					mime = mime,
				));
			}

			write_content(&mut w, "entry", &entry);
		}
	}

	write_event(&mut w, Event::End(BytesEnd::new("feed")));
	String::from_utf8(w.into_inner()).map_err(|e| AppError::Internal(format!("OPDS XML: {}", e)))
}

fn write_decl(w: &mut Writer<Vec<u8>>) {
	let _ = w.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)));
}

fn write_event(w: &mut Writer<Vec<u8>>, event: Event<'_>) {
	let _ = w.write_event(event);
}

fn write_text_elem(w: &mut Writer<Vec<u8>>, name: &str, text: &str) {
	let _ = w.write_event(Event::Start(BytesStart::new(name)));
	let _ = w.write_event(Event::Text(BytesText::new(text)));
	let _ = w.write_event(Event::End(BytesEnd::new(name)));
}

fn write_link(w: &mut Writer<Vec<u8>>, href: &str, rel: &str, title: &str, type_: Option<&str>) {
	let mut link = BytesStart::new("link");
	link.push_attribute(("href", href));
	link.push_attribute(("rel", rel));
	if let Some(t) = type_ {
		link.push_attribute(("type", t));
	}
	if !title.is_empty() {
		link.push_attribute(("title", title));
	}
	let _ = w.write_event(Event::Empty(link));
}

fn write_content(w: &mut Writer<Vec<u8>>, elem: &str, content: &str) {
	let _ = w.write_event(Event::Start(BytesStart::new(elem)));
	let _ = w.write_event(Event::CData(BytesCData::new(content)));
	let _ = w.write_event(Event::End(BytesEnd::new(elem)));
}

fn escape_xml(s: &str) -> String {
	s.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&apos;")
}
