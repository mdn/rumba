use crate::db::error::DbError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Elastic error: {0}")]
    Elastic(#[from] elasticsearch::Error),
    #[error("Elastic error: {source}, with reason: {reason}")]
    ElasticContext {
        reason: String,
        #[source]
        source: elasticsearch::Error,
    },
    #[error("Failed to parse elastic response")]
    ParseResponse,
}

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
    #[error("Query error")]
    Query(#[from] actix_web::error::QueryPayloadError),
    #[error("Search error")]
    Search(#[from] SearchError),
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
            Self::Query(_) => "Query error",
            Self::Search(_) => "Search error",
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
            Self::InvalidSession => StatusCode::BAD_REQUEST,
            Self::DocumentNotFound => StatusCode::NOT_FOUND,
            Self::NotificationNotFound => StatusCode::NOT_FOUND,
            Self::MalformedUrl => StatusCode::BAD_REQUEST,
            Self::Query(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
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
    fn from(_: r2d2::Error) -> Self {
        ApiError::ServerError
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(_: serde_json::Error) -> Self {
        ApiError::JsonProcessingError
    }
}
