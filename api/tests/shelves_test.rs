mod common;

use common::TestApp;

#[tokio::test]
async fn create_static_shelf() {
    let app = TestApp::new().await;
    app.register_and_login("shelf").await;

    let shelf = app.create_shelf("Favorites", "static", None).await;
    assert_eq!(shelf["name"], "Favorites");
    assert_eq!(shelf["kind"], "static");
    assert_eq!(shelf["book_count"], 0);
}

#[tokio::test]
async fn create_dynamic_shelf() {
    let app = TestApp::new().await;
    app.register_and_login("shelf").await;

    let ast = serde_json::json!({
        "operator": "and",
        "rules": [
            { "field": "read_status", "op": "equals", "value": "unread" }
        ]
    });
    let shelf = app.create_shelf("Unread Books", "dynamic", Some(ast)).await;
    assert_eq!(shelf["name"], "Unread Books");
    assert_eq!(shelf["kind"], "dynamic");
}

#[tokio::test]
async fn list_shelves_returns_created() {
    let app = TestApp::new().await;
    app.register_and_login("shelf").await;

    app.create_shelf("Shelf A", "static", None).await;
    app.create_shelf("Shelf B", "static", None).await;

    let list = app.list_shelves().await;
    let names: Vec<&str> = list.as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"Shelf A"));
    assert!(names.contains(&"Shelf B"));
}

#[tokio::test]
async fn add_book_to_static_shelf() {
    let app = TestApp::new().await;
    app.register_and_login("shelf").await;

    let shelf = app.create_shelf("Test Shelf", "static", None).await;
    let shelf_id = shelf["id"].as_str().unwrap().to_string();
    let book = app.create_book("Shelved Book", None).await;
    let book_id = book["id"].as_str().unwrap().to_string();

    app.add_book_to_shelf(&shelf_id, &book_id).await;

    let list = app.list_shelves().await;
    let s = list.as_array().unwrap().iter().find(|s| s["id"] == shelf["id"]).unwrap();
    assert_eq!(s["book_count"], 1, "shelf should have 1 book");
}

#[tokio::test]
async fn dynamic_shelf_book_count() {
    let app = TestApp::new().await;
    app.register_and_login("shelf").await;

    app.create_book("Unread One", None).await;
    app.create_book("Unread Two", None).await;

    let ast = serde_json::json!({
        "operator": "and",
        "rules": [
            { "field": "read_status", "op": "equals", "value": "unread" }
        ]
    });
    let shelf = app.create_shelf("Unread Shelf", "dynamic", Some(ast)).await;
    assert!(shelf["book_count"].as_u64().unwrap_or(0) >= 2, "dynamic shelf should count matching books");
}

#[tokio::test]
async fn dynamic_shelf_with_author_filter() {
    let app = TestApp::new().await;
    app.register_and_login("shelf").await;

    app.create_book("Book by Author1", Some("Author1")).await;
    app.create_book("Book by Author2", Some("Author2")).await;

    let ast = serde_json::json!({
        "operator": "and",
        "rules": [
            { "field": "author", "op": "contains", "value": "Author1" }
        ]
    });
    let shelf = app.create_shelf("Author1 Books", "dynamic", Some(ast)).await;
    assert_eq!(shelf["book_count"], 1, "should only count Author1's book");
}

#[tokio::test]
async fn delete_shelf() {
    let app = TestApp::new().await;
    app.register_and_login("shelf").await;

    let shelf = app.create_shelf("To Delete", "static", None).await;
    let id = shelf["id"].as_str().unwrap().to_string();

    let (status, _) = app.raw_delete(&format!("/api/v1/shelves/{}", id)).await;
    assert_eq!(status, 200, "delete shelf should succeed");

    let list = app.list_shelves().await;
    let ids: Vec<&str> = list.as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert!(!ids.contains(&id.as_str()), "deleted shelf should not appear");
}

#[tokio::test]
async fn cannot_add_to_others_shelf() {
    let app = TestApp::new().await;

    app.register("shelf_user_a@test.com", "password1!", "UserA").await.unwrap();
    app.login("shelf_user_a@test.com", "password1!").await.unwrap();
    let shelf = app.create_shelf("A's Shelf", "static", None).await;
    let shelf_id = shelf["id"].as_str().unwrap().to_string();

    app.register("shelf_user_b@test.com", "password2!", "UserB").await.unwrap();
    app.login("shelf_user_b@test.com", "password2!").await.unwrap();
    let (status, _) = app.raw_delete(&format!("/api/v1/shelves/{}", shelf_id)).await;
    assert_eq!(status, 403, "UserB cannot delete UserA's shelf");
}
