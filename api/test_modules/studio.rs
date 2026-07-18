

use crate::common::TestApp;
use serde_json::json;

#[tokio::test]
async fn studio_save_section() {
	let app = TestApp::new().await;
	app.register_and_login("studio").await;

	let book = app.create_book("Studio Draft", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let (status, _) = app
		.raw_put(
			&format!("/api/v1/books/{}/sections/00000000-0000-0000-0000-000000000001", book_id),
			&json!({ "blocks": [] }),
		)
		.await;
	assert_eq!(status, 404, "native book without IR should return 404: {status}");
}

#[tokio::test]
async fn studio_revision_list_empty() {
	let app = TestApp::new().await;
	app.register_and_login("studio").await;

	let book = app.create_book("No Revisions Yet", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let (status, revisions) = app.raw_get(&format!("/api/v1/books/{}/revisions", book_id)).await;
	assert_eq!(status, 200);
	assert!(revisions.as_array().unwrap().is_empty(), "new book should have no revisions");
}

#[tokio::test]
async fn studio_revision_get_not_found() {
	let app = TestApp::new().await;
	app.register_and_login("studio").await;

	let (status, _) = app.raw_get("/api/v1/revisions/00000000-0000-0000-0000-000000000000").await;
	assert_eq!(status, 404, "nonexistent revision should 404");
}

#[tokio::test]
async fn studio_revision_restore_not_found() {
	let app = TestApp::new().await;
	app.register_and_login("studio").await;

	let (status, _) = app
		.raw_post("/api/v1/revisions/00000000-0000-0000-0000-000000000000/restore", &json!({}))
		.await;
	assert_eq!(status, 404, "restore nonexistent revision should 404");
}

#[tokio::test]
async fn studio_edit_others_book_forbidden() {
	let app = TestApp::new().await;

	app.register("alice_studio@test.com", "pass1!", "Alice").await.unwrap();
	app.login("alice_studio@test.com", "pass1!").await.unwrap();
	let book = app.create_book("Alice's Draft", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	app.register("bob_studio@test.com", "pass2!", "Bob").await.unwrap();
	app.login("bob_studio@test.com", "pass2!").await.unwrap();
	let (status, _) = app
		.raw_put(
			&format!("/api/v1/books/{}/sections/00000000-0000-0000-0000-000000000001", book_id),
			&json!({ "blocks": [] }),
		)
		.await;
	assert_eq!(status, 403, "Bob should not edit Alice's book");
}

#[tokio::test]
async fn studio_revision_isolation() {
	let app = TestApp::new().await;

	app.register("alice_rev@test.com", "pass1!", "Alice").await.unwrap();
	app.login("alice_rev@test.com", "pass1!").await.unwrap();
	let book = app.create_book("Alice's History", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let (status, revisions) = app.raw_get(&format!("/api/v1/books/{}/revisions", book_id)).await;
	assert_eq!(status, 200);

	app.register("bob_rev@test.com", "pass2!", "Bob").await.unwrap();
	app.login("bob_rev@test.com", "pass2!").await.unwrap();
	let (status, _) = app.raw_get(&format!("/api/v1/books/{}/revisions", book_id)).await;
	assert_eq!(status, 403, "Bob should not see Alice's revisions");
}
