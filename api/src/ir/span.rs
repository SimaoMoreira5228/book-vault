use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Span {
    pub text: String,
    pub marks: u16,
    pub href: Option<String>,
}

impl Span {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            marks: 0,
            href: None,
        }
    }

    pub fn bold(mut self) -> Self {
        self.marks |= 1;
        self
    }

    pub fn italic(mut self) -> Self {
        self.marks |= 2;
        self
    }

    pub fn underline(mut self) -> Self {
        self.marks |= 4;
        self
    }

    pub fn strikethrough(mut self) -> Self {
        self.marks |= 8;
        self
    }
}
