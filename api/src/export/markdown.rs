use async_trait::async_trait;

use super::Exporter;
use crate::ir::BookIr;
use crate::ir::block::Block;
use crate::ir::span::Span;

pub struct MarkdownExporter;

fn render_spans(spans: &[Span]) -> String {
	let mut out = String::new();
	for s in spans {
		let mut text = s.text.clone();
		if s.marks & 8 != 0 {
			text = format!("~~{text}~~");
		}
		if s.marks & 2 != 0 {
			text = format!("*{text}*");
		}
		if s.marks & 1 != 0 {
			text = format!("**{text}**");
		}
		if let Some(href) = &s.href {
			text = format!("[{text}]({href})");
		}
		out.push_str(&text);
	}
	out
}

fn render_block(block: &Block) -> String {
	match block {
		Block::Paragraph(spans) => render_spans(spans),
		Block::Heading { level, spans } => {
			let prefix = "#".repeat((*level).clamp(1, 6) as usize);
			format!("{prefix} {}", render_spans(spans))
		}
		Block::Image { asset_ref, alt } => {
			let alt_text = alt.as_deref().unwrap_or("");
			format!("![{alt_text}]({asset_ref})")
		}
		Block::BlockQuote(blocks) => {
			let inner: String = blocks
				.iter()
				.flat_map(|b| render_block(b).lines().map(|l| format!("> {l}")).collect::<Vec<_>>())
				.collect::<Vec<_>>()
				.join("\n");
			inner
		}
		Block::CodeBlock { language, content } => {
			let lang = language.as_deref().unwrap_or("");
			format!("```{lang}\n{content}\n```")
		}
		Block::OrderedList(items) => items
			.iter()
			.enumerate()
			.map(|(i, item)| {
				let inner: String = item.iter().map(|b| render_block(b)).collect::<Vec<_>>().join(" ");
				format!("{}. {inner}", i + 1)
			})
			.collect::<Vec<_>>()
			.join("\n"),
		Block::UnorderedList(items) => items
			.iter()
			.map(|item| {
				let inner: String = item.iter().map(|b| render_block(b)).collect::<Vec<_>>().join(" ");
				format!("- {inner}")
			})
			.collect::<Vec<_>>()
			.join("\n"),
		Block::Table { headers, rows } => {
			let header: String = headers
				.iter()
				.map(|h| format!("| {}", render_spans(&h.spans)))
				.collect::<Vec<_>>()
				.join("") + "|";
			let sep: String = headers.iter().map(|_| "| --- ").collect::<Vec<_>>().join("") + "|";
			let body: String = rows
				.iter()
				.map(|row| {
					row.iter()
						.map(|c| format!("| {}", render_spans(&c.spans)))
						.collect::<Vec<_>>()
						.join("") + "|"
				})
				.collect::<Vec<_>>()
				.join("\n");
			format!("{header}\n{sep}\n{body}")
		}
		Block::HorizontalRule => "---".to_string(),
		Block::Footnote { marker, blocks } => {
			let inner: String = blocks.iter().map(|b| render_block(b)).collect::<Vec<_>>().join(" ");
			format!("[^{marker}]: {inner}")
		}
		Block::RawHtml { content } => content.clone(),
	}
}

#[async_trait]
impl Exporter for MarkdownExporter {
	async fn export(&self, ir: &BookIr) -> Result<Vec<u8>, crate::AppError> {
		let mut output = String::new();
		for section in &ir.spine {
			if let Some(title) = &section.title {
				output.push_str(&format!("# {title}\n\n"));
			}
			for block in &section.blocks {
				let rendered = render_block(block);
				output.push_str(&rendered);
				output.push('\n');
				output.push('\n');
			}
		}
		Ok(output.into_bytes())
	}
}
