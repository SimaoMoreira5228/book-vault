pub mod entities;

use migration::MigratorTrait;
use sea_orm::Database;
use tracing::info;

pub async fn connect(url: &str) -> Result<sea_orm::DatabaseConnection, Box<dyn std::error::Error>> {
	let url = if url.contains("sqlite") && !url.contains("mode=rwc") {
		if url.contains('?') {
			format!("{}&mode=rwc", url)
		} else {
			format!("{}?mode=rwc", url)
		}
	} else {
		url.to_string()
	};

	info!("Connecting to database");
	let db = Database::connect(&url).await?;
	info!("Database connected");
	Ok(db)
}

pub async fn run_migrations(db: &sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
	migration::Migrator::up(db, None).await?;
	info!("Migrations applied");
	Ok(())
}
