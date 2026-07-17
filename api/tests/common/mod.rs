use std::net::SocketAddr;
use std::sync::Arc;

use book_vault::{build_router, db, AppState, Config, SharedState, storage::LocalFsProvider};
use serde_json::Value;
use tokio::net::TcpListener;

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub struct TestApp {
    pub addr: SocketAddr,
    pub client: reqwest::Client,
    _db: tempfile::TempDir,
    _storage_dir: tempfile::TempDir,
    _shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

impl TestApp {
    pub async fn new() -> Self {
        let _db = tempfile::tempdir().expect("temp db dir");
        let db_file = _db.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_file.display());

        let _storage_dir = tempfile::tempdir().expect("temp storage dir");

        let db_conn = db::connect(&db_url).await.expect("db connect");
        db::run_migrations(&db_conn).await.expect("migrations");

        let storage = Arc::new(LocalFsProvider::new(_storage_dir.path().to_path_buf()));

        let config = Config::default();
        let state: SharedState = Arc::new(AppState {
            metadata_service: book_vault::metadata::service::MetadataService::new(),
            config,
            db: db_conn,
            storage,
        });

        let app = build_router(state);

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            let serve = axum::serve(listener, app);
            let _ = tokio::select! {
                _ = serve => {},
                _ = rx => {},
            };
        });

        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .expect("reqwest client");

        Self { addr, client, _db, _storage_dir, _shutdown: Some(tx) }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    pub async fn register(&self, email: &str, password: &str, display_name: &str) -> Result<Value, Value> {
        let body = serde_json::json!({ "email": email, "password": password, "display_name": display_name });
        let resp = self.client.post(self.url("/api/v1/auth/register")).json(&body).send().await.expect("register");
        let status = resp.status();
        let json: Value = resp.json().await.expect("register json");
        if status.is_success() || status.as_u16() == 201 { Ok(json) } else { Err(json) }
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<Value, Value> {
        let body = serde_json::json!({ "email": email, "password": password });
        let resp = self.client.post(self.url("/api/v1/auth/login")).json(&body).send().await.expect("login");
        let status = resp.status();
        let json: Value = resp.json().await.expect("login json");
        if status.is_success() { Ok(json) } else { Err(json) }
    }

    pub async fn register_and_login(&self, prefix: &str) -> (String, String, Value) {
        let c = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let email = format!("{}_{}@test.com", prefix, c);
        let pw = "testpass123!";
        let u = self.register(&email, pw, &format!("{} User {}", prefix, c)).await.expect("register");
        self.login(&email, pw).await.expect("login");
        (email, pw.to_string(), u)
    }

    pub async fn create_book(&self, title: &str, author: Option<&str>) -> Value {
        let mut body = serde_json::json!({ "title": title });
        if let Some(a) = author { body["author"] = Value::String(a.to_string()); }
        let resp = self.client.post(self.url("/api/v1/books")).json(&body).send().await.expect("create book");
        let s = resp.status();
        let j: Value = resp.json().await.expect("create book json");
        assert!(s.is_success() || s.as_u16() == 201, "create book {title} failed: {j}");
        j
    }

    pub async fn list_books(&self) -> Value {
        let resp = self.client.get(self.url("/api/v1/books")).send().await.expect("list books");
        assert!(resp.status().is_success());
        resp.json().await.expect("list books json")
    }

    pub async fn create_shelf(&self, name: &str, kind: &str, rule_ast: Option<Value>) -> Value {
        let mut body = serde_json::json!({ "name": name, "kind": kind });
        if let Some(ast) = rule_ast { body["rule_ast"] = ast; }
        let resp = self.client.post(self.url("/api/v1/shelves")).json(&body).send().await.expect("create shelf");
        let s = resp.status();
        let j: Value = resp.json().await.expect("create shelf json");
        assert!(s.is_success() || s.as_u16() == 201, "create shelf failed: {j}");
        j
    }

    pub async fn list_shelves(&self) -> Value {
        let resp = self.client.get(self.url("/api/v1/shelves")).send().await.expect("list shelves");
        assert!(resp.status().is_success());
        resp.json().await.expect("list shelves json")
    }

    pub async fn create_annotation(&self, book_id: &str, section_id: &str) -> Value {
        let body = serde_json::json!({
            "section_id": section_id,
            "block_index": 0,
            "start_offset": 0,
            "end_offset": 10,
            "color": "yellow",
            "note": "test annotation",
        });
        let resp = self
            .client
            .post(self.url(&format!("/api/v1/books/{}/annotations", book_id)))
            .json(&body)
            .send()
            .await
            .expect("create annotation");
        resp.json().await.expect("annotation json")
    }

    pub async fn save_progress(&self, book_id: &str, section_id: &str, pct: f64) {
        let body = serde_json::json!({
            "section_id": section_id,
            "block_index": 0,
            "char_offset": 0,
            "percentage": pct,
        });
        let resp = self
            .client
            .put(self.url(&format!("/api/v1/books/{}/progress", book_id)))
            .json(&body)
            .send()
            .await
            .expect("save progress");
        assert!(resp.status().is_success(), "save progress failed: {:?}", resp.text().await);
    }

    pub async fn add_book_to_shelf(&self, shelf_id: &str, book_id: &str) {
        let resp = self
            .client
            .post(self.url(&format!("/api/v1/shelves/{}/books", shelf_id)))
            .json(&serde_json::json!({ "book_id": book_id }))
            .send()
            .await
            .expect("add book");
        assert!(resp.status().is_success(), "add book failed: {:?}", resp.text().await);
    }

    pub async fn raw_get(&self, path: &str) -> (u16, Value) {
        let resp = self.client.get(self.url(path)).send().await.unwrap();
        let s = resp.status().as_u16();
        let j: Value = resp.json().await.unwrap_or(Value::Null);
        (s, j)
    }

    pub async fn raw_post(&self, path: &str, body: &Value) -> (u16, Value) {
        let resp = self.client.post(self.url(path)).json(body).send().await.unwrap();
        let s = resp.status().as_u16();
        let j: Value = resp.json().await.unwrap_or(Value::Null);
        (s, j)
    }

    pub async fn raw_put(&self, path: &str, body: &Value) -> (u16, Value) {
        let resp = self.client.put(self.url(path)).json(body).send().await.unwrap();
        let s = resp.status().as_u16();
        let j: Value = resp.json().await.unwrap_or(Value::Null);
        (s, j)
    }

    pub async fn raw_delete(&self, path: &str) -> (u16, Value) {
        let resp = self.client.delete(self.url(path)).send().await.unwrap();
        let s = resp.status().as_u16();
        let j: Value = resp.json().await.unwrap_or(Value::Null);
        (s, j)
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        if let Some(tx) = self._shutdown.take() {
            let _ = tx.send(());
        }
    }
}
