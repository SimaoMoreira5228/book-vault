use std::path::PathBuf;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default)]
pub struct Config {
	#[serde(default)]
	pub server: ServerConfig,
	#[serde(default)]
	pub database: DatabaseConfig,
	#[serde(default)]
	pub storage: StorageConfig,
	#[serde(default)]
	pub auth: AuthConfig,
	#[serde(default)]
	pub cors: CorsConfig,
	#[serde(default)]
	pub logging: LoggingConfig,
	#[serde(default)]
	pub integrations: IntegrationsConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
	#[serde(default = "default_host")]
	pub host: String,
	#[serde(default = "default_port")]
	pub port: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
	#[serde(default = "default_db_url")]
	pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StorageConfig {
	#[serde(default = "default_storage_path")]
	pub base_path: String,
	#[serde(default = "default_storage_provider")]
	pub provider: String,
	#[serde(default)]
	pub s3: S3Config,
}

#[derive(Clone, Debug, Deserialize)]
pub struct S3Config {
	#[serde(default = "default_s3_endpoint")]
	pub endpoint: String,
	#[serde(default = "default_s3_region")]
	pub region: String,
	#[serde(default = "default_s3_bucket")]
	pub bucket: String,
	#[serde(default = "default_s3_access_key")]
	pub access_key: String,
	#[serde(default = "default_s3_secret_key")]
	pub secret_key: String,
	#[serde(default)]
	pub path_style: bool,
}

fn default_s3_endpoint() -> String {
	String::new()
}
fn default_s3_region() -> String {
	"us-east-1".to_string()
}
fn default_s3_bucket() -> String {
	"bookvault".to_string()
}
fn default_s3_access_key() -> String {
	String::new()
}
fn default_s3_secret_key() -> String {
	String::new()
}

impl Default for S3Config {
	fn default() -> Self {
		Self {
			endpoint: default_s3_endpoint(),
			region: default_s3_region(),
			bucket: default_s3_bucket(),
			access_key: default_s3_access_key(),
			secret_key: default_s3_secret_key(),
			path_style: false,
		}
	}
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthConfig {
	#[serde(default = "default_session_ttl_days")]
	pub session_ttl_days: i64,
	#[serde(default = "default_session_idle_days")]
	pub session_idle_days: i64,
	#[serde(default = "default_auth_mode")]
	pub mode: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CorsConfig {
	#[serde(default = "default_cors_origin")]
	pub allowed_origin: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoggingConfig {
	#[serde(default = "default_log_level")]
	pub level: String,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct IntegrationsConfig {
	#[serde(default)]
	pub hardcover_api_key: String,
	#[serde(default)]
	pub email: EmailConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EmailConfig {
	#[serde(default = "default_email_enabled")]
	pub enabled: bool,
	#[serde(default = "default_email_host")]
	pub host: String,
	#[serde(default = "default_email_port")]
	pub port: u16,
	#[serde(default)]
	pub username: String,
	#[serde(default)]
	pub password: String,
	#[serde(default = "default_email_from")]
	pub from: String,
	#[serde(default = "default_email_tls")]
	pub tls_required: bool,
}

fn default_email_enabled() -> bool { false }
fn default_email_host() -> String { "localhost".to_string() }
fn default_email_port() -> u16 { 587 }
fn default_email_from() -> String { "bookvault@localhost".to_string() }
fn default_email_tls() -> bool { true }

impl Default for EmailConfig {
	fn default() -> Self {
		Self {
			enabled: default_email_enabled(),
			host: default_email_host(),
			port: default_email_port(),
			username: String::new(),
			password: String::new(),
			from: default_email_from(),
			tls_required: default_email_tls(),
		}
	}
}

fn default_host() -> String {
	"0.0.0.0".to_string()
}
fn default_port() -> u16 {
	8080
}
fn default_db_url() -> String {
	"sqlite://bookvault.db?mode=rwc".to_string()
}
fn default_storage_path() -> String {
	"./storage".to_string()
}
fn default_storage_provider() -> String {
	"local".to_string()
}
fn default_session_ttl_days() -> i64 {
	30
}
fn default_session_idle_days() -> i64 {
	7
}
fn default_auth_mode() -> String {
	"open".to_string()
}
fn default_cors_origin() -> String {
	"*".to_string()
}
fn default_log_level() -> String {
	"info".to_string()
}

impl Default for ServerConfig {
	fn default() -> Self {
		Self {
			host: default_host(),
			port: default_port(),
		}
	}
}

impl Default for DatabaseConfig {
	fn default() -> Self {
		Self { url: default_db_url() }
	}
}

impl Default for StorageConfig {
	fn default() -> Self {
		Self {
			base_path: default_storage_path(),
			provider: default_storage_provider(),
			s3: S3Config::default(),
		}
	}
}

impl Default for AuthConfig {
	fn default() -> Self {
		Self {
			session_ttl_days: default_session_ttl_days(),
			session_idle_days: default_session_idle_days(),
			mode: default_auth_mode(),
		}
	}
}

impl Default for CorsConfig {
	fn default() -> Self {
		Self {
			allowed_origin: default_cors_origin(),
		}
	}
}

impl Default for LoggingConfig {
	fn default() -> Self {
		Self {
			level: default_log_level(),
		}
	}
}

impl Config {
	pub fn load() -> Self {
		let config_path = std::env::var("BOOKVAULT_CONFIG").unwrap_or_else(|_| "bookvault.toml".to_string());

		let path = PathBuf::from(&config_path);
		if path.exists() {
			let content = std::fs::read_to_string(&path).expect("Failed to read config file");
			toml::from_str(&content).expect("Failed to parse config file")
		} else {
			Config::default()
		}
	}
}
