use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use lettre::message::header::{ContentDisposition, ContentType};
use std::str::FromStr;
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::{AsyncTransport, Message, Tokio1Executor};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::entities::book_ir;
use crate::db::entities::prelude::*;
use crate::export::{EpubExporter, Exporter, PdfExporter};
use crate::{AppError, SharedState};

#[derive(Deserialize)]
pub struct EmailBookRequest {
	pub to: String,
	#[serde(default = "default_email_format")]
	pub format: String,
}

fn default_email_format() -> String {
	"epub".to_string()
}

#[derive(Serialize)]
pub struct EmailBookResponse {
	pub message: String,
	pub to: String,
	pub format: String,
}

pub fn book_routes() -> Router<SharedState> {
	Router::new().route("/{id}/email", post(send_book_email))
}

pub fn status_routes() -> Router<SharedState> {
	Router::new().route("/status", get(email_status))
}

async fn email_status(State(state): State<SharedState>) -> Json<serde_json::Value> {
	let cfg = &state.config.integrations.email;
	let configured = !cfg.host.is_empty() && !cfg.username.is_empty() && !cfg.password.is_empty();
	Json(serde_json::json!({
		"enabled": cfg.enabled,
		"configured": configured,
	}))
}

async fn send_book_email(
	State(state): State<SharedState>,
	auth: AuthenticatedUser,
	Path(book_id): Path<Uuid>,
	Json(req): Json<EmailBookRequest>,
) -> Result<(StatusCode, Json<EmailBookResponse>), AppError> {
	let cfg = &state.config.integrations.email;
	if !cfg.enabled {
		return Err(AppError::BadRequest("Email delivery is not configured".into()));
	}

	let book = Books::find_by_id(book_id)
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book not found".into()))?;

	let library_ids = crate::routes::books::get_user_library_ids(&state.db, auth.user_id).await?;
	if !library_ids.contains(&book.library_id) {
		return Err(AppError::Forbidden("Access denied".into()));
	}

	let ir_row = BookIr::find()
		.filter(book_ir::Column::BookId.eq(book_id))
		.one(&state.db)
		.await?
		.ok_or_else(|| AppError::NotFound("Book IR not found".into()))?;

	let ir: crate::ir::BookIr = crate::ingest::deserialize_ir(&ir_row.payload)?;

	let fmt = if req.format == "pdf" { "pdf" } else { "epub" };
	let (bytes, mime_type, ext) = match fmt {
		"pdf" => {
			let exporter = PdfExporter;
			(exporter.export(&ir).await?, "application/pdf", "pdf")
		}
		_ => {
			let exporter = EpubExporter;
			(exporter.export(&ir).await?, "application/epub+zip", "epub")
		}
	};

	let filename = format!("{}_{}.{}", book.title.replace(' ', "_"), book_id, ext);
	let from_mailbox: Mailbox = cfg
		.from
		.parse()
		.map_err(|_| AppError::Internal("Invalid from address".into()))?;
	let to_mailbox: Mailbox = req
		.to
		.parse()
		.map_err(|_| AppError::BadRequest(format!("Invalid email: {}", req.to)))?;

	let text_part = SinglePart::builder()
		.header(ContentType::TEXT_PLAIN)
		.body(format!(
			"Here is your book \"{}\" sent from Book Vault.\n\nFormat: {}\n",
			book.title, ext
		));

	let ct = ContentType::from_str(mime_type)
		.map_err(|e| AppError::Internal(format!("Invalid MIME '{}': {}", mime_type, e)))?;
	let attach_part = SinglePart::builder()
		.header(ct,
		)
		.header(ContentDisposition::attachment(&filename))
		.body(bytes);

	let multipart = MultiPart::mixed().singlepart(text_part).singlepart(attach_part);

	let message = Message::builder()
		.from(from_mailbox)
		.to(to_mailbox)
		.subject(format!("Book Vault: {}", book.title))
		.multipart(multipart)
		.map_err(|e| AppError::Internal(format!("Email build: {}", e)))?;

	let creds = Credentials::new(cfg.username.clone(), cfg.password.clone());

	let mailer = if cfg.tls_required {
		AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
			.map_err(|e| AppError::Internal(format!("SMTP relay: {}", e)))?
			.port(cfg.port)
			.credentials(creds)
			.build()
	} else {
		AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host)
			.port(cfg.port)
			.credentials(creds)
			.build()
	};

	match mailer.send(message).await {
		Ok(_) => Ok((
			StatusCode::OK,
			Json(EmailBookResponse {
				message: "Email sent".to_string(),
				to: req.to,
				format: fmt.to_string(),
			}),
		)),
		Err(e) => Err(AppError::Internal(format!("SMTP send failed: {}", e))),
	}
}
