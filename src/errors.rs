use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Product not found")]
    ProductNotFound,

    #[error("Insufficient stock")]
    InsufficientStock,

    #[error("Bid price too low")]
    BidPriceTooLow,

    #[error("Invalid bid increment")]
    InvalidBidIncrement,

    #[error("Lock expired")]
    LockExpired,

    #[error("Bid not found")]
    BidNotFound,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Internal server error")]
    InternalError,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AppError::ProductNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::InsufficientStock => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::BidPriceTooLow => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::InvalidBidIncrement => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::LockExpired => (StatusCode::GONE, self.to_string()),
            AppError::BidNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::DatabaseError(e) => {
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = Json(ErrorResponse {
            error: status.canonical_reason().unwrap_or("Error").to_string(),
            message,
        });

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
