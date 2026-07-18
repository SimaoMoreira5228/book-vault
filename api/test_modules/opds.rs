use crate::common::TestApp;

#[tokio::test]
async fn opds_root_feed() {
	let app = TestApp::new().await;
	app.register_and_login("opds_root").await;

	let resp = app.client.get(app.url("/opds")).send().await.expect("opds root");
	assert_eq!(resp.status().as_u16(), 200);
	let ct = resp.headers().get("content-type").unwrap().to_str().unwrap().to_string();
	assert!(ct.contains("application/atom+xml"), "wrong content-type: {}", ct);

	let body = resp.text().await.expect("body");
	assert!(body.starts_with("<?xml"), "not xml: {}", &body[..100]);
	assert!(body.contains("<feed"), "no feed element");
	assert!(body.contains("BookVault"), "no title");
	assert!(body.contains("/opds/books"), "no books link");
	assert!(body.contains("/opds/search"), "no search link");
}

#[tokio::test]
async fn opds_books_feed() {
	let app = TestApp::new().await;
	app.register_and_login("opds_books").await;

	app.create_book("OPDS Test Book", Some("Test Author")).await;

	let resp = app.client.get(app.url("/opds/books")).send().await.expect("opds books");
	assert_eq!(resp.status().as_u16(), 200);
	let ct = resp.headers().get("content-type").unwrap().to_str().unwrap().to_string();
	assert!(ct.contains("application/atom+xml"), "wrong content-type: {}", ct);

	let body = resp.text().await.expect("body");
	assert!(body.contains("OPDS Test Book"), "missing book title");
	assert!(body.contains("Test Author"), "missing author");
	assert!(body.contains("opds-spec.org/acquisition"), "missing acquisition link");
	assert!(body.contains("opds-spec.org/image"), "missing cover link");
}

#[tokio::test]
async fn opds_shelf_feed() {
	let app = TestApp::new().await;
	app.register_and_login("opds_shelf").await;

	let book = app.create_book("Shelf Book", Some("Shelf Author")).await;
	let shelf = app.create_shelf("OPDS Shelf", "static", None).await;

	let shelf_id = shelf["id"].as_str().unwrap();
	let book_id = book["id"].as_str().unwrap();
	app.add_book_to_shelf(shelf_id, book_id).await;

	let resp = app
		.client
		.get(app.url(&format!("/opds/shelves/{}", shelf_id)))
		.send()
		.await
		.expect("opds shelf");
	assert_eq!(resp.status().as_u16(), 200, "shelf feed failed");

	let body = resp.text().await.expect("body");
	assert!(body.contains("Shelf Book"), "missing shelf book");
	assert!(body.contains("OPDS Shelf"), "missing shelf name");
}

#[tokio::test]
async fn opds_search_feed() {
	let app = TestApp::new().await;
	app.register_and_login("opds_search").await;

	app.create_book("Unique Searchable Title", Some("Author A")).await;
	app.create_book("Another Book", Some("Author B")).await;

	let resp = app
		.client
		.get(app.url("/opds/search?q=Unique"))
		.send()
		.await
		.expect("opds search");
	assert_eq!(resp.status().as_u16(), 200);

	let body = resp.text().await.expect("body");
	assert!(body.contains("Unique Searchable Title"), "missing matched book");
	assert!(!body.contains("Another Book"), "should not contain non-matching book");
}

#[tokio::test]
async fn opds_search_by_author() {
	let app = TestApp::new().await;
	app.register_and_login("opds_search_author").await;

	app.create_book("Book One", Some("Target Author")).await;
	app.create_book("Book Two", Some("Other Author")).await;

	let resp = app
		.client
		.get(app.url("/opds/search?q=Target"))
		.send()
		.await
		.expect("opds search author");
	assert_eq!(resp.status().as_u16(), 200);

	let body = resp.text().await.expect("body");
	assert!(body.contains("Book One"), "missing book by target author");
}

#[tokio::test]
async fn opds_empty_search() {
	let app = TestApp::new().await;
	app.register_and_login("opds_empty").await;

	app.create_book("Any Book", None).await;

	let resp = app
		.client
		.get(app.url("/opds/search?q=ThisDoesNotExistAtAllXYZ"))
		.send()
		.await
		.expect("opds empty search");
	assert_eq!(resp.status().as_u16(), 200);

	let body = resp.text().await.expect("body");
	assert!(!body.contains("Any Book"), "should have no entries");
}

#[tokio::test]
async fn opds_empty_shelf() {
	let app = TestApp::new().await;
	app.register_and_login("opds_empty_shelf").await;

	let shelf = app.create_shelf("Empty Shelf", "static", None).await;
	let shelf_id = shelf["id"].as_str().unwrap();

	let resp = app
		.client
		.get(app.url(&format!("/opds/shelves/{}", shelf_id)))
		.send()
		.await
		.expect("opds empty shelf");
	assert_eq!(resp.status().as_u16(), 200);

	let body = resp.text().await.expect("body");
	assert!(!body.contains("<entry>"), "empty shelf should have no entries");
}
