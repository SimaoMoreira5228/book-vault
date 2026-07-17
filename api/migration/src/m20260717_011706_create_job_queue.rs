use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(JobQueue::Table)
					.if_not_exists()
					.col(uuid(JobQueue::Id).primary_key())
					.col(string(JobQueue::Kind).not_null())
					.col(string(JobQueue::Status).not_null())
					.col(json(JobQueue::Payload).not_null())
					.col(string_null(JobQueue::Error))
					.col(tiny_integer(JobQueue::RetryCount).default(0))
					.col(tiny_integer(JobQueue::MaxRetries).default(3))
					.col(timestamp_with_time_zone_null(JobQueue::ScheduledAt))
					.col(timestamp_with_time_zone_null(JobQueue::StartedAt))
					.col(timestamp_with_time_zone_null(JobQueue::CompletedAt))
					.col(timestamp_with_time_zone(JobQueue::CreatedAt).not_null())
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				IndexCreateStatement::new()
					.name("idx_jq_status")
					.table(JobQueue::Table)
					.col(JobQueue::Status)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(JobQueue::Table).to_owned()).await
	}
}

#[derive(DeriveIden)]
pub enum JobQueue {
	Table,
	Id,
	Kind,
	Status,
	Payload,
	Error,
	RetryCount,
	MaxRetries,
	ScheduledAt,
	StartedAt,
	CompletedAt,
	CreatedAt,
}
