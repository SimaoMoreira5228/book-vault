

use crate::common::TestApp;

#[tokio::test]
async fn annotations_list_returns_empty_for_new_book() {
	let app = TestApp::new().await;
	app.register_and_login("ann").await;

	let book = app.create_book("Annotations Test", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let (status, result) = app.raw_get(&format!("/api/v1/books/{}/annotations", book_id)).await;
	assert_eq!(status, 200);
	assert_eq!(result.as_array().unwrap().len(), 0, "new book should have no annotations");
}

#[tokio::test]
async fn annotations_create_and_list() {
	let app = TestApp::new().await;
	app.register_and_login("ann").await;

	let book = app.create_book("Annotation CRUD", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();
	let section_id = "00000000-0000-0000-0000-000000000001";

	let ann = app.create_annotation(&book_id, section_id).await;
	let ann_id = ann["id"].as_str().unwrap().to_string();
	assert_eq!(ann["book_id"], book_id);
	assert_eq!(ann["color"], "yellow");

	let (status, list) = app.raw_get(&format!("/api/v1/books/{}/annotations", book_id)).await;
	assert_eq!(status, 200);
	let list = list.as_array().unwrap();
	assert_eq!(list.len(), 1);
	assert_eq!(list[0]["id"], ann_id);
}

#[tokio::test]
async fn annotations_create_with_color_and_block_index() {
	let app = TestApp::new().await;
	app.register_and_login("ann_col").await;

	let book = app.create_book("Colored Annotation", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();
	let section_id = "00000000-0000-0000-0000-000000000001";

	let (status, ann) = app
		.raw_post(
			&format!("/api/v1/books/{}/annotations", book_id),
			&serde_json::json!({
				"section_id": section_id,
				"block_index": 2,
				"start_offset": 5,
				"end_offset": 15,
				"color": "blue"
			}),
		)
		.await;
	assert_eq!(status, 201);
	assert_eq!(ann["block_index"], 2);
	assert_eq!(ann["color"], "blue");
	assert_eq!(ann["start_offset"], 5);
	assert_eq!(ann["end_offset"], 15);
}

#[tokio::test]
async fn annotations_update_note_and_color() {
	let app = TestApp::new().await;
	app.register_and_login("ann_upd").await;

	let book = app.create_book("Update Annotation", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();
	let section_id = "00000000-0000-0000-0000-000000000001";

	let ann = app.create_annotation(&book_id, section_id).await;
	let ann_id = ann["id"].as_str().unwrap().to_string();

	let (status, updated) = app
		.raw_put(
			&format!("/api/v1/annotations/{}", ann_id),
			&serde_json::json!({ "note": "This is a test note", "color": "green" }),
		)
		.await;
	assert_eq!(status, 200);
	assert_eq!(updated["note"], "This is a test note");
	assert_eq!(updated["color"], "green");
}

#[tokio::test]
async fn annotations_delete() {
	let app = TestApp::new().await;
	app.register_and_login("ann_del").await;

	let book = app.create_book("Delete Annotation", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();
	let section_id = "00000000-0000-0000-0000-000000000001";

	let ann = app.create_annotation(&book_id, section_id).await;
	let ann_id = ann["id"].as_str().unwrap().to_string();

	let (status, _) = app.raw_delete(&format!("/api/v1/annotations/{}", ann_id)).await;
	assert_eq!(status, 200);

	let (status, list) = app.raw_get(&format!("/api/v1/books/{}/annotations", book_id)).await;
	assert_eq!(status, 200);
	assert_eq!(list.as_array().unwrap().len(), 0, "annotation should be removed from list");
}

#[tokio::test]
async fn annotations_list_all_endpoint() {
	let app = TestApp::new().await;
	app.register_and_login("ann_all").await;

	let book1 = app.create_book("Annotations List All 1", None).await;
	let book2 = app.create_book("Annotations List All 2", None).await;
	let section_id = "00000000-0000-0000-0000-000000000001";

	let id1 = book1["id"].as_str().unwrap().to_string();
	let id2 = book2["id"].as_str().unwrap().to_string();
	app.create_annotation(&id1, section_id).await;
	app.create_annotation(&id2, section_id).await;

	let (status, result) = app.raw_get("/api/v1/annotations/all").await;
	assert_eq!(status, 200);
	let list = result.as_array().unwrap();
	assert_eq!(list.len(), 2, "should return annotations from all books");
}

#[tokio::test]
async fn annotations_cross_user_isolation() {
	let app = TestApp::new().await;

	app.register("alice_ann@test.com", "pass1!", "Alice").await.unwrap();
	app.login("alice_ann@test.com", "pass1!").await.unwrap();
	let book = app.create_book("Alice's Annotations", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();
	let section_id = "00000000-0000-0000-0000-000000000001";
	app.create_annotation(&book_id, section_id).await;

	app.register("bob_ann@test.com", "pass2!", "Bob").await.unwrap();
	app.login("bob_ann@test.com", "pass2!").await.unwrap();
	let (status, list) = app.raw_get(&format!("/api/v1/books/{}/annotations", book_id)).await;
	assert_eq!(status, 403, "Bob should not see Alice's book annotations");
}

#[tokio::test]
async fn annotations_nonexistent_returns_404() {
	let app = TestApp::new().await;
	app.register_and_login("ann_404").await;

	let fake_id = "00000000-0000-0000-0000-000000000000";
	let (status, _) = app
		.raw_put(
			&format!("/api/v1/annotations/{}", fake_id),
			&serde_json::json!({ "note": "test" }),
		)
		.await;
	assert_eq!(status, 404, "updating nonexistent annotation should 404");

	let (status, _) = app.raw_delete(&format!("/api/v1/annotations/{}", fake_id)).await;
	assert_eq!(status, 404, "deleting nonexistent annotation should 404");
}
