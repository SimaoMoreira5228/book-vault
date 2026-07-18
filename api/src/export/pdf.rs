use async_trait::async_trait;
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};

use super::Exporter;
use crate::ir::BookIr;
use crate::ir::block::Block;

pub struct PdfExporter;

const FONT_SIZE: f32 = 11.0;
const HEADING_SIZE: f32 = 16.0;
const LINE_HEIGHT: f32 = 15.0;
const MARGIN: f32 = 50.0;
const PAGE_W: f32 = 595.0;
const PAGE_H: f32 = 842.0;
const TEXT_W: f32 = PAGE_W - 2.0 * MARGIN;
const CHAR_W: f32 = FONT_SIZE * 0.55;

fn wrap_text(text: &str) -> Vec<String> {
	let max = (TEXT_W / CHAR_W).floor() as usize;
	let mut lines = Vec::new();
	let mut cur = String::new();
	for word in text.split_whitespace() {
		if cur.len() + word.len() + 1 > max && !cur.is_empty() {
			lines.push(cur.split_off(0));
		}
		if !cur.is_empty() {
			cur.push(' ');
		}
		cur.push_str(word);
	}
	if !cur.is_empty() {
		lines.push(cur);
	}
	if lines.is_empty() {
		lines.push(String::new());
	}
	lines
}

fn block_text(block: &Block) -> Vec<String> {
	match block {
		Block::Paragraph(spans) => {
			let text: String = spans.iter().map(|s| s.text.clone()).collect();
			wrap_text(&text)
		}
		Block::Heading { spans, .. } => {
			let text: String = spans.iter().map(|s| s.text.clone()).collect();
			vec![text]
		}
		Block::CodeBlock { content, .. } => wrap_text(content),
		Block::BlockQuote(blocks) => {
			let mut lines = Vec::new();
			for b in blocks {
				lines.extend(block_text(b));
			}
			lines
		}
		Block::OrderedList(items) | Block::UnorderedList(items) => {
			let mut lines = Vec::new();
			for item in items {
				for b in item {
					lines.extend(block_text(b));
				}
			}
			lines
		}
		Block::Table { headers, rows } => {
			let mut lines = Vec::new();
			for h in headers {
				let t: String = h.spans.iter().map(|s| s.text.clone()).collect();
				lines.extend(wrap_text(&t));
			}
			for row in rows {
				for c in row {
					let t: String = c.spans.iter().map(|s| s.text.clone()).collect();
					lines.extend(wrap_text(&t));
				}
			}
			lines
		}
		Block::Footnote { blocks, .. } => {
			let mut lines = Vec::new();
			for b in blocks {
				lines.extend(block_text(b));
			}
			lines
		}
		Block::HorizontalRule => vec!["---".to_string()],
		Block::Image { .. } | Block::RawHtml { .. } => Vec::new(),
	}
}

struct TextLine {
	text: String,
	size: f32,
}

fn collect_lines(ir: &BookIr) -> Vec<TextLine> {
	let mut lines = Vec::new();
	for section in &ir.spine {
		if let Some(title) = &section.title {
			lines.push(TextLine {
				text: title.clone(),
				size: HEADING_SIZE,
			});
		}
		for block in &section.blocks {
			let is_heading = matches!(block, Block::Heading { .. });
			let block_lines = block_text(block);
			for (j, l) in block_lines.iter().enumerate() {
				if is_heading && j == 0 {
					lines.push(TextLine {
						text: l.clone(),
						size: HEADING_SIZE,
					});
				} else {
					lines.push(TextLine {
						text: l.clone(),
						size: FONT_SIZE,
					});
				}
			}
			lines.push(TextLine {
				text: String::new(),
				size: FONT_SIZE,
			});
		}
	}
	lines
}

#[async_trait]
impl Exporter for PdfExporter {
	async fn export(&self, ir: &BookIr) -> Result<Vec<u8>, crate::AppError> {
		let lines = collect_lines(ir);
		let lines_per_page = ((PAGE_H - 2.0 * MARGIN) / LINE_HEIGHT).floor() as usize;
		let total_pages = if lines.is_empty() {
			1
		} else {
			lines.len().div_ceil(lines_per_page)
		};

		let mut pdf = Pdf::new();

		let catalog_id = Ref::new(1);
		let pages_id = Ref::new(2);
		let font_id = Ref::new(3);

		pdf.catalog(catalog_id).pages(pages_id);

		let page_refs: Vec<Ref> = (0..total_pages).map(|i| Ref::new(4 + 2 * i as i32)).collect();
		let content_refs: Vec<Ref> = (0..total_pages).map(|i| Ref::new(5 + 2 * i as i32)).collect();

		pdf.pages(pages_id).kids(page_refs.iter().copied()).count(total_pages as i32);

		pdf.type1_font(font_id).base_font(Name(b"Helvetica"));
		let font_name = Name(b"F1");

		for page_idx in 0..total_pages {
			let start = page_idx * lines_per_page;
			let end = (start + lines_per_page).min(lines.len());

			let mut content = Content::new();
			let mut y = PAGE_H - MARGIN;

			for line in &lines[start..end] {
				if line.text.is_empty() {
					y -= LINE_HEIGHT;
					continue;
				}
				content.begin_text();
				content.set_font(font_name, line.size);
				content.next_line(MARGIN, y);
				content.show(Str(line.text.as_bytes()));
				content.end_text();
				y -= LINE_HEIGHT;
			}

			pdf.stream(content_refs[page_idx], &content.finish());

			let mut page = pdf.page(page_refs[page_idx]);
			page.media_box(Rect::new(0.0, 0.0, PAGE_W, PAGE_H));
			page.parent(pages_id);
			page.contents(content_refs[page_idx]);
			page.resources().fonts().pair(font_name, font_id);
			page.finish();
		}

		Ok(pdf.finish())
	}
}
