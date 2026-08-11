use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error(transparent)]
    Database(#[from] diesel::result::Error),
    #[error("pool error: {0}")]
    Pool(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error, message) = match self {
            AppError::NotFound | AppError::Database(diesel::result::Error::NotFound) => (
                StatusCode::NOT_FOUND,
                "not_found",
                "The requested resource was not found.".to_string(),
            ),
            // info, not warn: a rejected request is not an operational problem, but the access
            // log records only "status=422", and the span deliberately carries no query string
            // (it is client-controlled and can hold secrets), so without this line a 422 cannot
            // be explained after the fact. Volume is bounded: every request already logs once.
            // the field is `reason`, never `message`: tracing reserves `message` for the event
            // body, so a field of that name emits JSON with two "message" keys and the parser
            // keeps whichever comes last, silently dropping the event's own text
            AppError::Validation(message) => {
                tracing::info!(reason = %message, "validation error");
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation_error",
                    message,
                )
            }
            AppError::Conflict(message) => {
                tracing::warn!(reason = %message, "conflict");
                (StatusCode::CONFLICT, "conflict", message)
            }
            // never leak diesel::result::Error's Display to the client, it exposes schema details
            AppError::Database(err) => {
                tracing::error!(error = %err, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "An internal error occurred.".to_string(),
                )
            }
            AppError::Pool(err) => {
                tracing::error!(error = %err, "pool error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "An internal error occurred.".to_string(),
                )
            }
            AppError::Internal(err) => {
                tracing::error!(error = %err, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "An internal error occurred.".to_string(),
                )
            }
        };

        (
            status,
            Json(ErrorBody {
                error: error.to_string(),
                message,
            }),
        )
            .into_response()
    }
}
