mod common;

mod annotations {
	include!("../test_modules/annotations.rs");
}
mod auth {
	include!("../test_modules/auth.rs");
}
mod authors {
	include!("../test_modules/authors.rs");
}
mod bookmarks {
	include!("../test_modules/bookmarks.rs");
}
mod books {
	include!("../test_modules/books.rs");
}
mod content_search {
	include!("../test_modules/content_search.rs");
}
mod edge_cases {
	include!("../test_modules/edge_cases.rs");
}
mod epub_fuzzer {
	include!("../test_modules/epub_fuzzer.rs");
}
mod export {
	include!("../test_modules/export.rs");
}
mod ingestion {
	include!("../test_modules/ingestion.rs");
}
mod search_metadata {
	include!("../test_modules/search_metadata.rs");
}
mod series {
	include!("../test_modules/series.rs");
}
mod shelves {
	include!("../test_modules/shelves.rs");
}
mod studio {
	include!("../test_modules/studio.rs");
}
mod tenant_isolation {
	include!("../test_modules/tenant_isolation.rs");
}
mod workflow {
	include!("../test_modules/workflow.rs");
}
mod opds {
	include!("../test_modules/opds.rs");
}
mod koreader_sync {
	include!("../test_modules/koreader_sync.rs");
}
mod email {
	include!("../test_modules/email.rs");
}
