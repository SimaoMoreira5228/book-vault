mod common;
use common::TestApp;

#[tokio::test]
async fn series_list_empty() {
	let app = TestApp::new().await;
	app.register_and_login("series").await;
	let (s, r) = app.raw_get("/api/v1/series").await;
	assert_eq!(s, 200);
	assert!(r.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn series_create_and_list() {
	let app = TestApp::new().await;
	app.register_and_login("series").await;
	let (s, _) = app.raw_post("/api/v1/series", &serde_json::json!({"name": "Foundation"})).await;
	assert_eq!(s, 200);
	let (s, r) = app.raw_get("/api/v1/series").await;
	assert_eq!(s, 200);
	assert_eq!(r.as_array().unwrap().len(), 1);
	assert_eq!(r[0]["name"], "Foundation");
}

#[tokio::test]
async fn series_get_by_id() {
	let app = TestApp::new().await;
	app.register_and_login("series").await;
	let (s, c) = app.raw_post("/api/v1/series", &serde_json::json!({"name": "Dune"})).await;
	assert_eq!(s, 200);
	let id = c["id"].as_str().unwrap();
	let (s, r) = app.raw_get(&format!("/api/v1/series/{}", id)).await;
	assert_eq!(s, 200);
	assert_eq!(r["name"], "Dune");
}

#[tokio::test]
async fn series_delete_unlinks_books() {
	let app = TestApp::new().await;
	app.register_and_login("series").await;
	let (s, c) = app.raw_post("/api/v1/series", &serde_json::json!({"name": "Series to Delete"})).await;
	assert_eq!(s, 200);
	let sid = c["id"].as_str().unwrap().to_string();
	let book = app.create_book("Series Book", None).await;
	let bid = book["id"].as_str().unwrap();
	app.raw_put(&format!("/api/v1/books/{}/link-series", bid), &serde_json::json!({"series_id": sid})).await;
	let (s, _) = app.raw_delete(&format!("/api/v1/series/{}", sid)).await;
	assert_eq!(s, 200);
	let list = app.list_books().await;
	assert_eq!(list["books"][0]["series_id"], serde_json::Value::Null, "series_id should be null after series deletion");
}

#[tokio::test]
async fn series_cross_user_isolation() {
	let app = TestApp::new().await;
	app.register("alice_s@test.com", "pass1!", "Alice").await.unwrap();
	app.login("alice_s@test.com", "pass1!").await.unwrap();
	app.raw_post("/api/v1/series", &serde_json::json!({"name": "Alice Series"})).await;
	app.logout().await.unwrap();
	app.register("bob_s@test.com", "pass2!", "Bob").await.unwrap();
	app.login("bob_s@test.com", "pass2!").await.unwrap();
	let (s, r) = app.raw_get("/api/v1/series").await;
	assert_eq!(s, 200);
	assert_eq!(r.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn series_nonexistent_returns_404() {
	let app = TestApp::new().await;
	app.register_and_login("series").await;
	let (s, _) = app.raw_get("/api/v1/series/00000000-0000-0000-0000-000000000000").await;
	assert_eq!(s, 404);
}
