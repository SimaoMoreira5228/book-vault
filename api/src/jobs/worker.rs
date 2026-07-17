use crate::db::entities::books;
use crate::db::entities::job_queue;
use crate::db::entities::prelude::*;
use crate::routes::events::notify;
use crate::{AppError, SharedState};
use sea_orm::{
    ColumnTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

pub struct JobWorker {
    state: SharedState,
    concurrency: usize,
}

impl JobWorker {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            concurrency: 2,
        }
    }

    pub fn with_concurrency(mut self, n: usize) -> Self {
        self.concurrency = n;
        self
    }

    pub async fn run_forever(self) {
        let cleanup_state = self.state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                if let Err(e) = Self::cleanup_sources(&cleanup_state).await {
                    error!("Source cleanup error: {e}");
                }
            }
        });

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        loop {
            let permit = semaphore.clone().acquire_owned().await;
            match permit {
                Ok(p) => {
                    let state = self.state.clone();
                    tokio::spawn(async move {
                        let _guard = p;
                        if let Err(e) = Self::process_one(&state).await {
                            error!("Job worker error: {e}");
                        }
                    });
                }
                Err(_) => break,
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn cleanup_sources(state: &SharedState) -> Result<(), AppError> {
        let books = Books::find()
            .filter(books::Column::KeepSource.eq(Some(false)))
            .filter(
                books::Column::ReadStatus
                    .ne("pending")
                    .and(books::Column::ReadStatus.ne("unread")),
            )
            .all(&state.db)
            .await?;

        for book in &books {
            if state.storage.exists(&book.id.to_string()).await.unwrap_or(false) {
                state.storage.delete(&book.id.to_string()).await?;
                info!("Deleted source file for book {} ({})", book.id, book.title);
            }
        }
        Ok(())
    }

    async fn process_one(state: &SharedState) -> Result<(), AppError> {
        let job = Self::claim_job(state).await?;
        if let Some(job) = job {
            let kind = job.kind.clone();
            info!("Processing job {}: {}", job.id, kind);
            Self::update_status(state, job.id, "processing", None).await?;
            notify(format!(r#"{{"job_id":"{}","status":"processing","kind":"{}"}}"#, job.id, kind));

            let result = Self::dispatch(state, &kind, &job).await;

            match result {
                Ok(()) => {
                    Self::update_status(state, job.id, "completed", None).await?;
                    notify(format!(r#"{{"job_id":"{}","status":"completed","kind":"{}"}}"#, job.id, kind));
                    info!("Job {} completed", job.id);
                }
                Err(e) => {
                    let new_count = job.retry_count + 1;
                    if new_count >= job.max_retries {
                        Self::update_status(state, job.id, "dead_letter", Some(&e.to_string())).await?;
                        notify(format!(r#"{{"job_id":"{}","status":"dead_letter","kind":"{}"}}"#, job.id, kind));
                        error!("Job {} dead-lettered: {}", job.id, e);
                    } else {
                        Self::update_status_with_retry(state, job.id, "pending", Some(&e.to_string()), new_count).await?;
                        warn!("Job {} failed (retry {}/{}): {}", job.id, new_count, job.max_retries, e);
                        notify(format!(r#"{{"job_id":"{}","status":"retrying","kind":"{}"}}"#, job.id, kind));
                    }
                }
            }
        }
        Ok(())
    }

    async fn claim_job(state: &SharedState) -> Result<Option<job_queue::Model>, AppError> {
        let db = &state.db;
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();

        let job = JobQueue::find()
            .filter(job_queue::Column::Status.eq("pending"))
            .filter(
                job_queue::Column::ScheduledAt
                    .is_null()
                    .or(job_queue::Column::ScheduledAt.lte(now)),
            )
            .order_by_asc(job_queue::Column::CreatedAt)
            .one(db)
            .await?;

        if let Some(ref j) = job {
            let mut active: job_queue::ActiveModel = j.clone().into();
            active.status = Set("processing".to_string());
            active.started_at = Set(Some(now));
            JobQueue::update(active).exec(db).await?;
        }

        Ok(job)
    }

    async fn update_status(
        state: &SharedState,
        job_id: uuid::Uuid,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let job = JobQueue::find_by_id(job_id)
            .one(&state.db)
            .await?
            .ok_or_else(|| AppError::NotFound("Job not found".into()))?;

        let mut active: job_queue::ActiveModel = job.into();
        active.status = Set(status.to_string());
        if let Some(e) = error {
            active.error = Set(Some(e.to_string()));
        }
        if status == "completed" {
            active.completed_at = Set(Some(now));
        }
        JobQueue::update(active).exec(&state.db).await?;
        Ok(())
    }

    async fn update_status_with_retry(
        state: &SharedState,
        job_id: uuid::Uuid,
        status: &str,
        error: Option<&str>,
        retry_count: i8,
    ) -> Result<(), AppError> {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let job = JobQueue::find_by_id(job_id)
            .one(&state.db)
            .await?
            .ok_or_else(|| AppError::NotFound("Job not found".into()))?;

        let mut active: job_queue::ActiveModel = job.into();
        active.status = Set(status.to_string());
        active.retry_count = Set(retry_count);
        active.error = Set(Some(error.unwrap_or("").to_string()));
        let base_secs = Duration::from_secs(5).as_secs();
        let multiplier = 1u64 << std::cmp::min(retry_count as u64, 4);
        let backoff = (base_secs * multiplier) as i64;
        active.scheduled_at = Set(Some(now + chrono::Duration::seconds(backoff)));
        JobQueue::update(active).exec(&state.db).await?;
        Ok(())
    }

    async fn dispatch(
        state: &SharedState,
        kind: &str,
        job: &job_queue::Model,
    ) -> Result<(), AppError> {
        match kind {
            "ingest_epub" => crate::ingest::epub::ingest(&state, job).await,
            "ingest_pdf" => crate::ingest::pdf::ingest(&state, job).await,
            "ingest_cbz" => crate::ingest::cbz::ingest(&state, job).await,
            "ingest_mobi" => crate::ingest::mobi::ingest(&state, job).await,
            other => Err(AppError::Internal(format!("Unknown job kind: {other}"))),
        }
    }
}
