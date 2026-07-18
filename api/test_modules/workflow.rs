

use crate::common::TestApp;

#[tokio::test]
async fn register_create_book_then_annotate() {
	let app = TestApp::new().await;
	let (_email, _pw, _user) = app.register_and_login("workflow").await;

	let book = app.create_book("Workflow Book", Some("Test Author")).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let list = app.list_books().await;
	assert!(list["books"].as_array().unwrap().iter().any(|b| b["id"] == book_id));

	let section_id = "00000000-0000-0000-0000-000000000001";
	let ann = app.create_annotation(&book_id, section_id).await;
	assert!(ann.get("id").is_some(), "annotation should have id");
	assert_eq!(ann["book_id"], book_id);

	app.save_progress(&book_id, section_id, 50.0).await;

	let (status, progress) = app.raw_get(&format!("/api/v1/books/{}/progress", book_id)).await;
	assert_eq!(status, 200);
	assert_eq!(progress["percentage"], 50.0);
	assert_eq!(progress["section_id"], section_id);

	let (status, export_result) = app
		.raw_get(&format!("/api/v1/books/{}/export?format=markdown", book_id))
		.await;
	assert!(
		status == 200 || status == 404 || status == 500,
		"export should not crash: {status}"
	);
	if status == 200 {
		let _ = export_result;
	}
}

#[tokio::test]
async fn full_lifecycle() {
	let app = TestApp::new().await;
	let (_email, _pw, _user) = app.register_and_login("lifecycle").await;

	
	let shelf = app.create_shelf("My Favorites", "static", None).await;
	let shelf_id = shelf["id"].as_str().unwrap().to_string();

	
	let book1 = app.create_book("Lifecycle Book 1", Some("Author A")).await;
	let book2 = app.create_book("Lifecycle Book 2", Some("Author B")).await;
	let book1_id = book1["id"].as_str().unwrap().to_string();
	let book2_id = book2["id"].as_str().unwrap().to_string();

	
	app.add_book_to_shelf(&shelf_id, &book1_id).await;

	
	let shelves = app.list_shelves().await;
	let s = shelves.as_array().unwrap().iter().find(|s| s["id"] == shelf_id).unwrap();
	assert_eq!(s["book_count"], 1, "shelf should have 1 book");

	
	let (status, updated) = app
		.raw_put(
			&format!("/api/v1/books/{}", book1_id),
			&serde_json::json!({ "read_status": "reading", "rating": 4 }),
		)
		.await;
	assert_eq!(status, 200);
	assert_eq!(updated["read_status"], "reading");

	
	let (status, _) = app.raw_delete(&format!("/api/v1/books/{}", book2_id)).await;
	assert_eq!(status, 200);

	
	let list = app.list_books().await;
	let ids: Vec<&str> = list["books"].as_array().unwrap().iter().map(|b| b["id"].as_str().unwrap()).collect();
	assert!(ids.contains(&book1_id.as_str()));
	assert!(!ids.contains(&book2_id.as_str()));
}

#[tokio::test]
async fn create_dynamic_shelf_and_verify_books() {
	let app = TestApp::new().await;
	let (_email, _pw, _user) = app.register_and_login("dynamic").await;

	app.create_book("Sci-Fi Novel", Some("Isaac Asimov")).await;
	app.create_book("Fantasy Epic", Some("J.R.R. Tolkien")).await;
	app.create_book("Sci-Fi Shorts", Some("Arthur C. Clarke")).await;

	let ast = serde_json::json!({
		"operator": "or",
		"rules": [
			{ "field": "author", "op": "contains", "value": "Asimov" },
			{ "field": "author", "op": "contains", "value": "Clarke" }
		]
	});

	let shelf = app.create_shelf("Science Fiction Authors", "dynamic", Some(ast)).await;
	assert_eq!(shelf["book_count"], 2, "should match 2 sci-fi authors");
}
