mod common;

use common::TestApp;

#[tokio::test]
async fn register_login_logout() {
    let app = TestApp::new().await;

    let res = app.register("alice@test.com", "password123!", "Alice").await;
    assert!(res.is_ok(), "register should succeed");
    let user = res.unwrap();
    assert_eq!(user["email"], "alice@test.com");
    assert_eq!(user["display_name"], "Alice");
    assert_eq!(user["is_admin"], false);
    assert!(user["id"].as_str().unwrap().len() > 0);

    let res = app.login("alice@test.com", "password123!").await;
    assert!(res.is_ok(), "login should succeed");
    assert!(res.unwrap()["user"]["email"] == "alice@test.com");
}

#[tokio::test]
async fn register_duplicate_email() {
    let app = TestApp::new().await;

    app.register("dup@test.com", "password123!", "First").await.unwrap();
    let res = app.register("dup@test.com", "other456!", "Second").await;
    assert!(res.is_err(), "duplicate email should fail");
}

#[tokio::test]
async fn login_wrong_password() {
    let app = TestApp::new().await;

    app.register("bob@test.com", "correctpass!", "Bob").await.unwrap();
    let res = app.login("bob@test.com", "wrongpass!").await;
    assert!(res.is_err(), "wrong password should fail");
}

#[tokio::test]
async fn login_nonexistent_user() {
    let app = TestApp::new().await;
    let res = app.login("nobody@test.com", "password123!").await;
    assert!(res.is_err(), "nonexistent user should fail");
}

#[tokio::test]
async fn register_short_password() {
    let app = TestApp::new().await;
    let res = app.register("short@test.com", "1234567", "Short").await;
    assert!(res.is_ok(), "server accepts short password (validation is client-side)");
}

#[tokio::test]
async fn session_persists_across_calls() {
    let app = TestApp::new().await;
    let (_email, _pw, _user) = app.register_and_login("session").await;

    let (status, _) = app.raw_get("/api/v1/books").await;
    assert_eq!(status, 200, "authenticated request should work");
    let (status2, _) = app.raw_get("/api/v1/shelves").await;
    assert_eq!(status2, 200, "second call with same session works");
}

#[tokio::test]
async fn no_session_returns_401() {
    let app = TestApp::new().await;
    let (status, _) = app.raw_get("/api/v1/books").await;
    assert_eq!(status, 401, "unauthenticated request should fail");
}
