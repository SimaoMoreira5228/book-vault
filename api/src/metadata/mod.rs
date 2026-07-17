pub mod openlibrary;
pub mod google_books;
pub mod service;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MetadataQuery {
    pub title: Option<String>,
    pub author: Option<String>,
    pub isbn: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProspectiveMetadata {
    pub provider: String,
    pub provider_id: String,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub isbn10: Option<String>,
    pub isbn13: Option<String>,
    pub page_count: Option<i32>,
    pub genres: Vec<String>,
    pub cover_url: Option<String>,
    pub published_date: Option<String>,
    pub publisher: Option<String>,
    pub rating: Option<f32>,
    pub subtitle: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MetadataField {
    Title,
    Author,
    Description,
    Cover,
    Genres,
    PageCount,
    Isbn,
    Publisher,
    PublishedDate,
    Subtitle,
}

impl MetadataField {
    pub fn all() -> Vec<MetadataField> {
        vec![
            MetadataField::Title,
            MetadataField::Author,
            MetadataField::Description,
            MetadataField::Cover,
            MetadataField::Genres,
            MetadataField::PageCount,
            MetadataField::Isbn,
            MetadataField::Publisher,
            MetadataField::PublishedDate,
            MetadataField::Subtitle,
        ]
    }
}

impl std::fmt::Display for MetadataField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataField::Title => write!(f, "title"),
            MetadataField::Author => write!(f, "author"),
            MetadataField::Description => write!(f, "description"),
            MetadataField::Cover => write!(f, "cover"),
            MetadataField::Genres => write!(f, "genres"),
            MetadataField::PageCount => write!(f, "page_count"),
            MetadataField::Isbn => write!(f, "isbn"),
            MetadataField::Publisher => write!(f, "publisher"),
            MetadataField::PublishedDate => write!(f, "published_date"),
            MetadataField::Subtitle => write!(f, "subtitle"),
        }
    }
}

impl std::str::FromStr for MetadataField {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "title" => Ok(MetadataField::Title),
            "author" => Ok(MetadataField::Author),
            "description" => Ok(MetadataField::Description),
            "cover" => Ok(MetadataField::Cover),
            "genres" => Ok(MetadataField::Genres),
            "page_count" => Ok(MetadataField::PageCount),
            "isbn" => Ok(MetadataField::Isbn),
            "publisher" => Ok(MetadataField::Publisher),
            "published_date" => Ok(MetadataField::PublishedDate),
            "subtitle" => Ok(MetadataField::Subtitle),
            _ => Err(format!("Unknown metadata field: {s}")),
        }
    }
}

#[async_trait]
pub trait MetadataProvider: Send + Sync {
    fn id(&self) -> &'static str;
    async fn search(&self, query: &MetadataQuery) -> Result<Vec<ProspectiveMetadata>, crate::AppError>;
}
