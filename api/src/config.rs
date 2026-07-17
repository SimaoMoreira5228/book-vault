use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
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
    pub logging: LoggingConfig,
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
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
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
        Self {
            url: default_db_url(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_path: default_storage_path(),
            provider: default_storage_provider(),
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

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path =
            std::env::var("BOOKVAULT_CONFIG").unwrap_or_else(|_| "bookvault.toml".to_string());

        let path = PathBuf::from(&config_path);
        if path.exists() {
            let content = std::fs::read_to_string(&path).expect("Failed to read config file");
            toml::from_str(&content).expect("Failed to parse config file")
        } else {
            Config::default()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            storage: StorageConfig::default(),
            auth: AuthConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}
