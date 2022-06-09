use crate::db::error::DbError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use r2d2::Error;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("unknown error")]
    Unknown,
    #[error("Invalid Session info")]
    InvalidSession,
    #[error("Database error")]
    ServerError,
    #[error("Document Not found")]
    DocumentNotFound,
    #[error("Notification Not found")]
    NotificationNotFound,
    #[error("Malformed Url")]
    MalformedUrl,
    #[error("Json error")]
    JsonProcessingError,
}

impl ApiError {
    pub fn name(&self) -> &str {
        match self {
            Self::Unknown => "Unknown",
            Self::InvalidSession => "Invalid Session",
            Self::ServerError => "Server error",
            Self::DocumentNotFound => "Document not found",
            Self::MalformedUrl => "Malformed URL",
            Self::NotificationNotFound => "Notification not found",
            Self::JsonProcessingError => "Error processing JSON document",
        }
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    error: String,
    message: String,
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidSession => StatusCode::BAD_REQUEST,
            Self::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::DocumentNotFound => StatusCode::NOT_FOUND,
            Self::NotificationNotFound => StatusCode::NOT_FOUND,
            Self::MalformedUrl => StatusCode::BAD_REQUEST,
            Self::JsonProcessingError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = ErrorResponse {
            code: status_code.as_u16(),
            message: self.to_string(),
            error: self.name().to_string(),
        };
        HttpResponse::build(status_code).json(error_response)
    }
}

impl From<DbError> for ApiError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::DieselResult(_) => ApiError::Unknown,
            DbError::R2D2Error(_) => ApiError::Unknown,
        }
    }
}

impl From<diesel::result::Error> for ApiError {
    fn from(_: diesel::result::Error) -> Self {
        ApiError::Unknown
    }
}

impl From<r2d2::Error> for ApiError {
    fn from(_: Error) -> Self {
        ApiError::ServerError
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(_: serde_json::Error) -> Self {
        ApiError::JsonProcessingError
    }
}
