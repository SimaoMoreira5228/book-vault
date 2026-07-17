use super::span::Span;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TableCell {
    pub spans: Vec<Span>,
    pub col_span: u8,
    pub row_span: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Block {
    Paragraph(Vec<Span>),
    Heading {
        level: u8,
        spans: Vec<Span>,
    },
    Image {
        asset_ref: Uuid,
        alt: Option<String>,
    },
    BlockQuote(Vec<Block>),
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    OrderedList(Vec<Vec<Block>>),
    UnorderedList(Vec<Vec<Block>>),
    Table {
        headers: Vec<TableCell>,
        rows: Vec<Vec<TableCell>>,
    },
    HorizontalRule,
    Footnote {
        marker: String,
        blocks: Vec<Block>,
    },
    RawHtml {
        content: String,
    },
}
