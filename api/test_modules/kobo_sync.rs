use crate::common::TestApp;

async fn create_kobo_token(app: &TestApp) -> (String, String) {
	let resp = app
		.client
		.post(app.url("/api/v1/admin/kobo-tokens"))
		.json(&serde_json::json!({ "device_name": "test-device" }))
		.send()
		.await
		.expect("create token");
	let status = resp.status().as_u16();
	let json: serde_json::Value = resp.json().await.expect("token json");
	assert_eq!(status, 200, "create token failed: {json}");
	let token = json["token"].as_str().unwrap().to_string();
	let id = json["id"].as_str().unwrap().to_string();
	(id, token)
}

fn make_epub() -> Vec<u8> {
	use std::io::Write;
	let mut buf = Vec::new();
	{
		let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
		let opts: zip::write::FileOptions<()> =
			zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
		zip.start_file("mimetype", opts).unwrap();
		zip.write_all(b"application/epub+zip").unwrap();
		zip.start_file("META-INF/container.xml", opts).unwrap();
		zip.write_all(b"<?xml version=\"1.0\"?><container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\"><rootfiles><rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>").unwrap();
		zip.start_file("OEBPS/content.opf", opts).unwrap();
		zip.write_all(b"<?xml version=\"1.0\"?><package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"book-id\"><metadata><dc:title xmlns:dc=\"http://purl.org/dc/elements/1.1/\">Kobo Test</dc:title><dc:language xmlns:dc=\"http://purl.org/dc/elements/1.1/\">en</dc:language></metadata><spine><itemref idref=\"x001\"/></spine><manifest><item id=\"x001\" href=\"content.xhtml\" media-type=\"application/xhtml+xml\"/></manifest></package>").unwrap();
		zip.start_file("OEBPS/content.xhtml", opts).unwrap();
		zip.write_all(b"<?xml version=\"1.0\"?><html xmlns=\"http://www.w3.org/1999/xhtml\"><head><title>Test</title></head><body><p>Kobo book content.</p></body></html>").unwrap();
	}
	buf
}

#[tokio::test]
async fn kobo_token_create_and_list() {
	let app = TestApp::new().await;
	app.register_and_login("kobo_tokens").await;

	let (id, token) = create_kobo_token(&app).await;
	assert_eq!(token.len(), 64, "token should be 64 hex chars");

	let resp = app.client.get(app.url("/api/v1/admin/kobo-tokens")).send().await.unwrap();
	assert_eq!(resp.status().as_u16(), 200);
	let list: serde_json::Value = resp.json().await.unwrap();
	let arr = list.as_array().unwrap();
	assert_eq!(arr.len(), 1);
	assert_eq!(arr[0]["id"], id);
}

#[tokio::test]
async fn kobo_token_revoke() {
	let app = TestApp::new().await;
	app.register_and_login("kobo_revoke").await;

	let (id, _token) = create_kobo_token(&app).await;

	let resp = app.client.delete(app.url(&format!("/api/v1/admin/kobo-tokens/{}", id))).send().await.unwrap();
	assert_eq!(resp.status().as_u16(), 200);

	let resp = app.client.get(app.url("/api/v1/admin/kobo-tokens")).send().await.unwrap();
	let list: serde_json::Value = resp.json().await.unwrap();
	assert!(list.as_array().unwrap().is_empty(), "token should be revoked");
}

#[tokio::test]
async fn kobo_init_invalid_token() {
	let app = TestApp::new().await;
	let resp = app.client.get(app.url("/api/kobo/badtoken/v1/initialization")).send().await.unwrap();
	assert_eq!(resp.status().as_u16(), 200, "kobo returns 200 even on error");
	let json: serde_json::Value = resp.json().await.unwrap();
	assert!(json["error"].as_str().is_some(), "should have error field: {json}");
}

#[tokio::test]
async fn kobo_init_success() {
	let app = TestApp::new().await;
	app.register_and_login("kobo_init").await;
	let (_id, token) = create_kobo_token(&app).await;

	let resp = app
		.client
		.get(app.url(&format!("/api/kobo/{}/v1/initialization", token)))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 200);
	let json: serde_json::Value = resp.json().await.unwrap();
	assert!(json["user"].is_object(), "should have user: {json}");
	assert_eq!(json["device_token"], token);
}

#[tokio::test]
async fn kobo_library() {
	let app = TestApp::new().await;
	app.register_and_login("kobo_lib").await;
	let (_id, token) = create_kobo_token(&app).await;

	let epub = make_epub();
	let upload = app.upload_file("kobo_test.epub", &epub).await;
	let job_id = upload["job_id"].as_str().unwrap();
	app.wait_for_job(job_id, std::time::Duration::from_secs(10)).await;
	let book_id = upload["book_id"].as_str().unwrap();

	let resp = app
		.client
		.get(app.url(&format!("/api/kobo/{}/v1/library", token)))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 200);
	let json: serde_json::Value = resp.json().await.unwrap();
	let books = json["books"].as_array().unwrap();
	assert!(!books.is_empty(), "library should have books: {json}");
	assert!(books.iter().any(|b| b["id"] == book_id), "should contain uploaded book");
}

#[tokio::test]
async fn kobo_library_empty() {
	let app = TestApp::new().await;
	app.register_and_login("kobo_empty").await;
	let (_id, token) = create_kobo_token(&app).await;

	let resp = app
		.client
		.get(app.url(&format!("/api/kobo/{}/v1/library", token)))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 200);
	let json: serde_json::Value = resp.json().await.unwrap();
	assert!(json["books"].as_array().unwrap().is_empty(), "should be empty");
}

#[tokio::test]
async fn kobo_entitlements() {
	let app = TestApp::new().await;
	app.register_and_login("kobo_ent").await;
	let (_id, token) = create_kobo_token(&app).await;

	let epub = make_epub();
	let upload = app.upload_file("kobo_ent.epub", &epub).await;
	let job_id = upload["job_id"].as_str().unwrap();
	app.wait_for_job(job_id, std::time::Duration::from_secs(10)).await;
	let book_id = upload["book_id"].as_str().unwrap();

	let resp = app
		.client
		.get(app.url(&format!("/api/kobo/{}/v1/entitlements", token)))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 200);
	let json: serde_json::Value = resp.json().await.unwrap();
	let ents = json["entitlements"].as_array().unwrap();
	assert!(!ents.is_empty(), "should have entitlements: {json}");
	assert!(ents.iter().any(|e| e["book_id"] == book_id));
}

#[tokio::test]
async fn kobo_reading_state_get_empty() {
	let app = TestApp::new().await;
	app.register_and_login("kobo_rs_empty").await;
	let (_id, token) = create_kobo_token(&app).await;

	let resp = app
		.client
		.get(app.url(&format!("/api/kobo/{}/v1/readingstate", token)))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 200);
	let json: serde_json::Value = resp.json().await.unwrap();
	assert!(json["reading_states"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn kobo_reading_state_save_and_get() {
	let app = TestApp::new().await;
	app.register_and_login("kobo_rs_sg").await;
	let (_id, token) = create_kobo_token(&app).await;

	let epub = make_epub();
	let upload = app.upload_file("kobo_rs.epub", &epub).await;
	let job_id = upload["job_id"].as_str().unwrap();
	app.wait_for_job(job_id, std::time::Duration::from_secs(10)).await;
	let book_id = upload["book_id"].as_str().unwrap();

	let save = app
		.client
		.post(app.url(&format!("/api/kobo/{}/v1/readingstate", token)))
		.json(&serde_json::json!({ "book_id": book_id, "percentage": 42.5 }))
		.send()
		.await
		.unwrap();
	assert_eq!(save.status().as_u16(), 200, "save failed: {:?}", save.text().await);

	let resp = app
		.client
		.get(app.url(&format!("/api/kobo/{}/v1/readingstate", token)))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 200);
	let json: serde_json::Value = resp.json().await.unwrap();
	let states = json["reading_states"].as_array().unwrap();
	assert_eq!(states.len(), 1, "should have one state: {json}");
	assert_eq!(states[0]["book_id"], book_id);
	assert!((states[0]["percentage"].as_f64().unwrap() - 42.5).abs() < 0.01);
}

#[tokio::test]
async fn kobo_reading_state_update() {
	let app = TestApp::new().await;
	app.register_and_login("kobo_rs_upd").await;
	let (_id, token) = create_kobo_token(&app).await;

	let epub = make_epub();
	let upload = app.upload_file("kobo_upd.epub", &epub).await;
	let job_id = upload["job_id"].as_str().unwrap();
	app.wait_for_job(job_id, std::time::Duration::from_secs(10)).await;
	let book_id = upload["book_id"].as_str().unwrap();

	app.client
		.post(app.url(&format!("/api/kobo/{}/v1/readingstate", token)))
		.json(&serde_json::json!({ "book_id": book_id, "percentage": 10.0 }))
		.send()
		.await
		.unwrap();

	app.client
		.post(app.url(&format!("/api/kobo/{}/v1/readingstate", token)))
		.json(&serde_json::json!({ "book_id": book_id, "percentage": 90.0 }))
		.send()
		.await
		.unwrap();

	let resp = app
		.client
		.get(app.url(&format!("/api/kobo/{}/v1/readingstate", token)))
		.send()
		.await
		.unwrap();
	let json: serde_json::Value = resp.json().await.unwrap();
	let states = json["reading_states"].as_array().unwrap();
	assert_eq!(states.len(), 1, "should still be one entry");
	assert!((states[0]["percentage"].as_f64().unwrap() - 90.0).abs() < 0.01, "should be updated");
}

#[tokio::test]
async fn kobo_cover_redirect() {
	let app = TestApp::new().await;
	app.register_and_login("kobo_cover").await;
	let (_id, token) = create_kobo_token(&app).await;

	let book = app.create_book("Cover Test", None).await;
	let book_id = book["id"].as_str().unwrap();

	let no_redirect_client = reqwest::Client::builder()
		.cookie_store(true)
		.redirect(reqwest::redirect::Policy::none())
		.build()
		.unwrap();
	let resp = no_redirect_client
		.get(app.url(&format!("/api/kobo/{}/v1/cover/{}", token, book_id)))
		.send()
		.await
		.unwrap();
	let status = resp.status().as_u16();
	let location = resp.headers().get("location").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
	assert!(
		status == 302 || status == 307 || status == 303,
		"cover should redirect, got {status}, location={location}"
	);
	assert!(location.contains("/api/v1/books/") && location.contains("/cover"),
		"should redirect to cover endpoint: {location}");
}

#[tokio::test]
async fn kobo_download() {
	let app = TestApp::new().await;
	app.register_and_login("kobo_dl").await;
	let (_id, token) = create_kobo_token(&app).await;

	let epub = make_epub();
	let upload = app.upload_file("kobo_dl.epub", &epub).await;
	let job_id = upload["job_id"].as_str().unwrap();
	app.wait_for_job(job_id, std::time::Duration::from_secs(10)).await;
	let book_id = upload["book_id"].as_str().unwrap();

	let resp = app
		.client
		.get(app.url(&format!("/api/kobo/{}/v1/download/{}", token, book_id)))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 200, "download failed: {:?}", resp.text().await);
	let ct = resp.headers().get("content-type").unwrap().to_str().unwrap().to_string();
	assert!(ct.contains("epub") || ct.contains("octet-stream"), "wrong content type: {ct}");
	let body = resp.bytes().await.unwrap();
	assert!(body.len() > 100, "download too small: {} bytes", body.len());
}

#[tokio::test]
async fn kobo_download_forbidden_other_user() {
	let app = TestApp::new().await;
	app.register("alice@kobo.test", "pass1!", "Alice").await.unwrap();
	app.login("alice@kobo.test", "pass1!").await.unwrap();
	let (_, _alice_token) = create_kobo_token(&app).await;

	let epub = make_epub();
	let upload = app.upload_file("alice.epub", &epub).await;
	let job_id = upload["job_id"].as_str().unwrap();
	app.wait_for_job(job_id, std::time::Duration::from_secs(10)).await;
	let alice_book = upload["book_id"].as_str().unwrap().to_string();

	let _ = app.logout().await;

	app.register("bob@kobo.test", "pass1!", "Bob").await.unwrap();
	app.login("bob@kobo.test", "pass1!").await.unwrap();
	let (_, bob_token) = create_kobo_token(&app).await;

	let resp = app
		.client
		.get(app.url(&format!("/api/kobo/{}/v1/download/{}", bob_token, alice_book)))
		.send()
		.await
		.unwrap();
	let status = resp.status().as_u16();
	let json: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
	assert!(
		status == 200 && json["error"].as_str().is_some() || status != 200,
		"should reject download from other user, got status={status}, json={json}"
	);
}

#[tokio::test]
async fn kobo_init_returns_user_info_for_valid_token() {
	let app = TestApp::new().await;
	app.register("carol@kobo.test", "pass1!", "Carol").await.unwrap();
	app.login("carol@kobo.test", "pass1!").await.unwrap();
	let (_, token) = create_kobo_token(&app).await;

	let resp = app
		.client
		.get(app.url(&format!("/api/kobo/{}/v1/initialization", token)))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 200);
	let json: serde_json::Value = resp.json().await.unwrap();
	assert!(json["user"].is_object(), "should return user: {json}");
	assert_eq!(json["device_token"], token);
}
