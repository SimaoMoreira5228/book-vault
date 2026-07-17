use async_trait::async_trait;
use std::io::Cursor;
use epub_builder::{EpubBuilder, EpubContent, ReferenceType, ZipLibrary};
use crate::ir::BookIr;
use crate::ir::block::Block;
use crate::ir::span::Span;
use super::Exporter;

pub struct EpubExporter;

const CSS: &str = r#"
body { font-family: serif; line-height: 1.6; margin: 2em; }
h1, h2, h3, h4, h5, h6 { font-family: sans-serif; }
p { margin: 0.5em 0; text-align: justify; }
blockquote { margin: 1em 2em; font-style: italic; color: #555; }
pre { background: #f5f5f5; padding: 1em; overflow-x: auto; }
code { font-family: monospace; }
table { border-collapse: collapse; width: 100%; margin: 1em 0; }
th, td { border: 1px solid #ccc; padding: 0.5em; text-align: left; }
th { background: #f0f0f0; }
img { max-width: 100%; height: auto; }
hr { border: none; border-top: 1px solid #ccc; margin: 1em 0; }
"#;

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_spans(spans: &[Span]) -> String {
    let mut out = String::new();
    for s in spans {
        let mut text = escape_html(&s.text);
        if s.marks & 8 != 0 { text = format!("<s>{text}</s>"); }
        if s.marks & 4 != 0 { text = format!("<u>{text}</u>"); }
        if s.marks & 2 != 0 { text = format!("<em>{text}</em>"); }
        if s.marks & 1 != 0 { text = format!("<strong>{text}</strong>"); }
        if let Some(href) = &s.href {
            text = format!("<a href=\"{href}\">{text}</a>");
        }
        out.push_str(&text);
    }
    out
}

fn render_block(block: &Block) -> String {
    match block {
        Block::Paragraph(spans) => format!("<p>{}</p>", render_spans(spans)),
        Block::Heading { level, spans } => {
            let l = (*level).clamp(1, 6);
            format!("<h{l}>{}</h{l}>", render_spans(spans))
        }
        Block::Image { asset_ref, alt } => {
            let alt_text = escape_html(alt.as_deref().unwrap_or(""));
            format!("<img src=\"{asset_ref}\" alt=\"{alt_text}\" />")
        }
        Block::BlockQuote(blocks) => {
            let inner: String = blocks.iter().map(|b| render_block(b)).collect();
            format!("<blockquote>{inner}</blockquote>")
        }
        Block::CodeBlock { language, content } => {
            let lang = language.as_deref().unwrap_or("");
            format!("<pre><code class=\"language-{lang}\">{}</code></pre>", escape_html(content))
        }
        Block::OrderedList(items) => {
            let inner: String = items.iter()
                .map(|item| format!("<li>{}</li>", item.iter().map(|b| render_block(b)).collect::<String>()))
                .collect();
            format!("<ol>{inner}</ol>")
        }
        Block::UnorderedList(items) => {
            let inner: String = items.iter()
                .map(|item| format!("<li>{}</li>", item.iter().map(|b| render_block(b)).collect::<String>()))
                .collect();
            format!("<ul>{inner}</ul>")
        }
        Block::Table { headers, rows } => {
            let header_row: String = headers.iter()
                .map(|h| format!("<th>{}</th>", render_spans(&h.spans)))
                .collect();
            let body_rows: String = rows.iter()
                .map(|row| {
                    let cells: String = row.iter()
                        .map(|c| format!("<td>{}</td>", render_spans(&c.spans)))
                        .collect();
                    format!("<tr>{cells}</tr>")
                })
                .collect();
            format!("<table><thead><tr>{header_row}</tr></thead><tbody>{body_rows}</tbody></table>")
        }
        Block::HorizontalRule => "<hr />".to_string(),
        Block::Footnote { marker, blocks } => {
            let inner: String = blocks.iter().map(|b| render_block(b)).collect();
            format!("<sup>{marker}</sup>{inner}")
        }
        Block::RawHtml { content } => content.clone(),
    }
}

fn section_to_xhtml(title: &str, blocks: &[Block]) -> String {
    let safe_title = escape_html(title);
    let body: String = blocks.iter().map(|b| render_block(b)).collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{safe_title}</title></head>
<body>
<h1>{safe_title}</h1>
{body}
</body>
</html>"#
    )
}

#[async_trait]
impl Exporter for EpubExporter {
    async fn export(&self, ir: &BookIr) -> Result<Vec<u8>, crate::AppError> {
        let zip = ZipLibrary::new()
            .map_err(|e| crate::AppError::Internal(format!("Failed to init ZIP: {e}")))?;
        let mut builder = EpubBuilder::new(zip)
            .map_err(|e| crate::AppError::Internal(format!("Failed to create EPUB: {e}")))?;

        let title = ir.spine.first()
            .and_then(|s| s.title.as_deref())
            .unwrap_or("Untitled");

        builder.metadata("title", title)
            .map_err(|e| crate::AppError::Internal(format!("Failed to set title: {e}")))?;

        builder.stylesheet(Cursor::new(CSS.as_bytes()))
            .map_err(|e| crate::AppError::Internal(format!("Failed to set stylesheet: {e}")))?;

        if !ir.spine.is_empty() {
            builder.inline_toc();
        }

        for (i, section) in ir.spine.iter().enumerate() {
            let section_title = section.title.as_deref().unwrap_or("Untitled");
            let xhtml = section_to_xhtml(section_title, &section.blocks);
            builder.add_content(
                EpubContent::new(format!("section_{i}.xhtml"), Cursor::new(xhtml.into_bytes()))
                    .title(section_title)
                    .reftype(ReferenceType::Text),
            ).map_err(|e| crate::AppError::Internal(format!("Failed to add section: {e}")))?;
        }

        let mut output = Vec::new();
        builder.generate(&mut output)
            .map_err(|e| crate::AppError::Internal(format!("Failed to generate EPUB: {e}")))?;

        Ok(output)
    }
}
