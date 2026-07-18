use crate::common::TestApp;

#[tokio::test]
async fn koreader_save_progress() {
	let app = TestApp::new().await;
	let (_email, _pw, _user) = app.register_and_login("koreader_save").await;

	let book = app.create_book("KOReader Book", None).await;
	let book_id = book["id"].as_str().unwrap();

	let req = serde_json::json!({
		"document": book_id,
		"progress": 0.45,
		"device_id": "test-device",
		"status": "reading",
		"current_page": 45,
		"total_pages": 100,
		"timestamp": 1234567890
	});

	let resp = app
		.client
		.put(app.url("/api/v1/koreader/progress"))
		.json(&req)
		.send()
		.await
		.expect("koreader save");
	assert_eq!(resp.status().as_u16(), 200, "koreader save failed");

	let json: serde_json::Value = resp.json().await.expect("koreader save json");
	assert_eq!(json["progress"], 0.45);
	assert_eq!(json["status"], "reading");
	assert_eq!(json["document"], book_id);
}

#[tokio::test]
async fn koreader_get_progress() {
	let app = TestApp::new().await;
	let (_email, _pw, _user) = app.register_and_login("koreader_get").await;

	let book = app.create_book("KOReader Get Book", None).await;
	let book_id = book["id"].as_str().unwrap();

	let req = serde_json::json!({
		"document": book_id,
		"progress": 0.75,
		"device_id": "device-2",
		"status": "reading",
		"current_page": 75,
		"total_pages": 100,
		"timestamp": 1234567899
	});

	app.client
		.put(app.url("/api/v1/koreader/progress"))
		.json(&req)
		.send()
		.await
		.expect("save first");

	let resp = app
		.client
		.get(app.url(&format!("/api/v1/koreader/progress/{}", book_id)))
		.send()
		.await
		.expect("koreader get");
	assert_eq!(resp.status().as_u16(), 200);

	let json: serde_json::Value = resp.json().await.expect("koreader get json");
	let arr = json.as_array().expect("should be array");
	assert_eq!(arr.len(), 1, "should have one progress entry");
	assert!((arr[0]["progress"].as_f64().unwrap() - 0.75).abs() < 0.01);
}

#[tokio::test]
async fn koreader_update_progress() {
	let app = TestApp::new().await;
	let (_email, _pw, _user) = app.register_and_login("koreader_update").await;

	let book = app.create_book("KOReader Update Book", None).await;
	let book_id = book["id"].as_str().unwrap();

	let req = serde_json::json!({
		"document": book_id,
		"progress": 0.3,
		"device_id": "device-3",
		"status": "reading",
		"current_page": 30,
		"total_pages": 100,
		"timestamp": 1234567890
	});

	app.client
		.put(app.url("/api/v1/koreader/progress"))
		.json(&req)
		.send()
		.await
		.expect("first save");

	let req2 = serde_json::json!({
		"document": book_id,
		"progress": 0.9,
		"device_id": "device-3",
		"status": "reading",
		"current_page": 90,
		"total_pages": 100,
		"timestamp": 1234567999
	});

	app.client
		.put(app.url("/api/v1/koreader/progress"))
		.json(&req2)
		.send()
		.await
		.expect("second save");

	let resp = app
		.client
		.get(app.url(&format!("/api/v1/koreader/progress/{}", book_id)))
		.send()
		.await
		.expect("get progress");

	let json: serde_json::Value = resp.json().await.expect("get json");
	let arr = json.as_array().unwrap();
	assert_eq!(arr.len(), 1, "should still be one entry");
	assert!((arr[0]["progress"].as_f64().unwrap() - 0.9).abs() < 0.01, "should be updated to 0.9");
}

#[tokio::test]
async fn koreader_get_no_progress() {
	let app = TestApp::new().await;
	let (_email, _pw, _user) = app.register_and_login("koreader_none").await;

	let book = app.create_book("KOReader No Progress", None).await;
	let book_id = book["id"].as_str().unwrap();

	let resp = app
		.client
		.get(app.url(&format!("/api/v1/koreader/progress/{}", book_id)))
		.send()
		.await
		.expect("get empty progress");
	assert_eq!(resp.status().as_u16(), 200);

	let json: serde_json::Value = resp.json().await.expect("get json");
	let arr = json.as_array().expect("should be array");
	assert_eq!(arr.len(), 0, "should have no entries");
}
