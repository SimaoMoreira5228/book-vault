mod common;

use common::TestApp;

#[tokio::test]
async fn books_isolation() {
    let app = TestApp::new().await;

    app.register("alice@tenant.test", "password1!", "Alice").await.unwrap();
    app.login("alice@tenant.test", "password1!").await.unwrap();
    let book = app.create_book("Alice's Secret", None).await;
    let book_id = book["id"].as_str().unwrap().to_string();

    app.register("bob@tenant.test", "password2!", "Bob").await.unwrap();
    app.login("bob@tenant.test", "password2!").await.unwrap();

    for path in &[
        format!("/api/v1/books/{}", book_id),
        format!("/api/v1/books/{}/read", book_id),
        format!("/api/v1/books/{}/raw", book_id),
        format!("/api/v1/books/{}/progress", book_id),
    ] {
        let (status, _) = app.raw_get(path).await;
        assert_eq!(status, 403, "Bob should get 403 on GET {}", path);
    }

    let (status, _) = app.raw_put(&format!("/api/v1/books/{}", book_id), &serde_json::json!({"title": "hacked"})).await;
    assert_eq!(status, 403, "Bob should get 403 on PUT book");

    let (status, _) = app.raw_delete(&format!("/api/v1/books/{}", book_id)).await;
    assert_eq!(status, 403, "Bob should get 403 on DELETE book");
}

#[tokio::test]
async fn shelves_isolation() {
    let app = TestApp::new().await;

    app.register("alice_s@tenant.test", "password1!", "Alice").await.unwrap();
    app.login("alice_s@tenant.test", "password1!").await.unwrap();
    let shelf = app.create_shelf("Alice's Shelf", "static", None).await;
    let shelf_id = shelf["id"].as_str().unwrap().to_string();

    app.register("bob_s@tenant.test", "password2!", "Bob").await.unwrap();
    app.login("bob_s@tenant.test", "password2!").await.unwrap();

    let (status, _) = app.raw_get(&format!("/api/v1/shelves/{}", shelf_id)).await;
    assert_eq!(status, 403, "Bob should get 403 on GET shelf");

    let (status, _) = app.raw_delete(&format!("/api/v1/shelves/{}", shelf_id)).await;
    assert_eq!(status, 403, "Bob should get 403 on DELETE shelf");
}

#[tokio::test]
async fn annotations_isolation() {
    let app = TestApp::new().await;

    app.register("alice_a@tenant.test", "password1!", "Alice").await.unwrap();
    app.login("alice_a@tenant.test", "password1!").await.unwrap();
    let book = app.create_book("Annotated Book", None).await;
    let book_id = book["id"].as_str().unwrap().to_string();

    let ann = app.create_annotation(&book_id, "00000000-0000-0000-0000-000000000001").await;
    let ann_id = ann.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();

    app.register("bob_a@tenant.test", "password2!", "Bob").await.unwrap();
    app.login("bob_a@tenant.test", "password2!").await.unwrap();

    let (status, _) = app.raw_get(&format!("/api/v1/books/{}/annotations", book_id)).await;
    assert_eq!(status, 403, "Bob should get 403 listing Alice's annotations");

    if !ann_id.is_empty() {
        let (status, _) = app.raw_delete(&format!("/api/v1/annotations/{}", ann_id)).await;
        assert_eq!(status, 403, "Bob should get 403 deleting Alice's annotation");
    }
}

#[tokio::test]
async fn metadata_isolation() {
    let app = TestApp::new().await;

    app.register("alice_m@tenant.test", "password1!", "Alice").await.unwrap();
    app.login("alice_m@tenant.test", "password1!").await.unwrap();
    let book = app.create_book("Metadata Book", None).await;
    let book_id = book["id"].as_str().unwrap().to_string();

    app.register("bob_m@tenant.test", "password2!", "Bob").await.unwrap();
    app.login("bob_m@tenant.test", "password2!").await.unwrap();

    let (status, _) = app.raw_get(&format!("/api/v1/books/{}/metadata", book_id)).await;
    assert_eq!(status, 403, "Bob should get 403 on metadata");

    let (status, _) = app.raw_get(&format!("/api/v1/books/{}/metadata/candidates", book_id)).await;
    assert_eq!(status, 403, "Bob should get 403 on metadata candidates");
}

#[tokio::test]
async fn export_isolation() {
    let app = TestApp::new().await;

    app.register("alice_e@tenant.test", "password1!", "Alice").await.unwrap();
    app.login("alice_e@tenant.test", "password1!").await.unwrap();
    let book = app.create_book("Export Book", None).await;
    let book_id = book["id"].as_str().unwrap().to_string();

    app.register("bob_e@tenant.test", "password2!", "Bob").await.unwrap();
    app.login("bob_e@tenant.test", "password2!").await.unwrap();

    let (status, _) = app.raw_get(&format!("/api/v1/books/{}/export", book_id)).await;
    assert_eq!(status, 403, "Bob should get 403 on export");
}
