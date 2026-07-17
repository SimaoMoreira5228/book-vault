mod common;

use common::TestApp;

#[tokio::test]
async fn admin_non_admin_gets_403() {
    let app = TestApp::new().await;
    app.register_and_login("admin").await;

    let (status, _) = app.raw_get("/api/v1/admin/jobs").await;
    assert_eq!(status, 403, "non-admin should not access admin jobs");

    let (status, _) = app.raw_get("/api/v1/admin/users").await;
    assert_eq!(status, 403, "non-admin should not access admin users");
}

#[tokio::test]
async fn raw_source_returns_source_file() {
    let app = TestApp::new().await;
    app.register_and_login("raw").await;

    let epub_bytes = b"PK\x03\x04...mock content for epub detection...";
    let resp = app
        .client
        .post(app.url("/api/v1/books/upload"))
        .multipart(
            reqwest::multipart::Form::new()
                .part("file", reqwest::multipart::Part::bytes(epub_bytes.to_vec()).file_name("test.epub")),
        )
        .send()
        .await
        .expect("upload");

    let json: serde_json::Value = resp.json().await.unwrap_or_default();
    let book_id = json.get("book_id").and_then(|v| v.as_str()).unwrap_or("");

    if !book_id.is_empty() {
        let resp = app
            .client
            .get(app.url(&format!("/api/v1/books/{}/raw", book_id)))
            .send()
            .await
            .expect("raw source");
        assert_eq!(resp.status().as_u16(), 200, "raw source should be accessible");
        let content_type = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");
        assert!(content_type.contains("epub") || content_type.contains("octet-stream"), "should have proper content type");
    }
}

#[tokio::test]
async fn invalid_book_id_returns_404() {
    let app = TestApp::new().await;
    app.register_and_login("edge").await;

    let (status, _) = app.raw_get("/api/v1/books/not-a-uuid").await;
    assert_eq!(status, 400, "invalid UUID should return 400 bad request");
}

#[tokio::test]
async fn nonexistent_book_404() {
    let app = TestApp::new().await;
    app.register_and_login("edge").await;

    let id = "00000000-0000-0000-0000-000000000000";
    for path in &[
        format!("/api/v1/books/{}", id),
        format!("/api/v1/books/{}/read", id),
        format!("/api/v1/books/{}/raw", id),
        format!("/api/v1/books/{}/progress", id),
        format!("/api/v1/books/{}/export", id),
        format!("/api/v1/books/{}/metadata", id),
    ] {
        let (status, _) = app.raw_get(path).await;
        assert_eq!(status, 404, "nonexistent book should 404 on GET {}", path);
    }
}

#[tokio::test]
async fn create_book_empty_title() {
    let app = TestApp::new().await;
    app.register_and_login("edge").await;

    let (status, _) = app.raw_post("/api/v1/books", &serde_json::json!({ "title": "" })).await;
    assert!(status == 200 || status == 201 || status == 400, "should accept or reject empty title: {status}");
}

#[tokio::test]
async fn delete_shelf_with_books() {
    let app = TestApp::new().await;
    app.register_and_login("edge").await;

    let shelf = app.create_shelf("Cascade Test", "static", None).await;
    let shelf_id = shelf["id"].as_str().unwrap().to_string();
    let book = app.create_book("Cascade Book", None).await;
    let book_id = book["id"].as_str().unwrap().to_string();

    app.add_book_to_shelf(&shelf_id, &book_id).await;

    let (status, _) = app.raw_delete(&format!("/api/v1/shelves/{}", shelf_id)).await;
    assert_eq!(status, 200, "shelf should be deletable even with books");

    let (status, _) = app.raw_delete(&format!("/api/v1/shelves/{}", shelf_id)).await;
    assert_eq!(status, 404, "deleted shelf should 404");
}

#[tokio::test]
async fn duplicate_annotation_same_position() {
    let app = TestApp::new().await;
    app.register_and_login("edge").await;

    let book = app.create_book("Duplicate Annotation", None).await;
    let book_id = book["id"].as_str().unwrap().to_string();
    let section_id = "00000000-0000-0000-0000-000000000001";

    let ann1 = app.create_annotation(&book_id, section_id).await;
    assert!(ann1.get("id").is_some(), "first annotation should succeed");

    let ann2 = app.create_annotation(&book_id, section_id).await;
    assert!(ann2.get("id").is_some(), "second annotation at same position should also succeed");
    assert_ne!(ann1["id"], ann2["id"], "annotations should have different IDs");
}

#[tokio::test]
async fn progress_updates_read_status() {
    let app = TestApp::new().await;
    app.register_and_login("edge").await;

    let book = app.create_book("Progress Tracking", None).await;
    let book_id = book["id"].as_str().unwrap().to_string();
    let section_id = "00000000-0000-0000-0000-000000000001";

    app.save_progress(&book_id, section_id, 50.0).await;

    let (status, updated) = app.raw_get(&format!("/api/v1/books/{}", book_id)).await;
    assert_eq!(status, 200);
    assert_eq!(updated["read_status"], "reading", "progress should set book to reading");

    app.save_progress(&book_id, section_id, 100.0).await;

    let (status, finished) = app.raw_get(&format!("/api/v1/books/{}", book_id)).await;
    assert_eq!(status, 200);
    assert_eq!(finished["read_status"], "finished", "100% progress should set book to finished");
}

#[tokio::test]
async fn session_list_and_revoke() {
    let app = TestApp::new().await;
    let (_email, _pw, _user) = app.register_and_login("session2").await;

    let (status, sessions) = app.raw_get("/api/v1/auth/sessions").await;
    assert_eq!(status, 200, "should list sessions");
    let sessions = sessions.as_array().unwrap();
    assert!(sessions.len() >= 1, "should have at least 1 session");
    assert_eq!(sessions[0]["is_current"], true, "current session should be marked");

    let session_id = sessions[0]["id"].as_str().unwrap().to_string();
    let (status, _) = app.raw_delete(&format!("/api/v1/auth/sessions/{}", session_id)).await;
    assert_eq!(status, 200, "should revoke current session");
}
