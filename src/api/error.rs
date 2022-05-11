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
}

impl ApiError {
    pub fn name(&self) -> String {
        match self {
            Self::Unknown => "Unknown".to_string(),
            Self::InvalidSession => "Invalid Session".to_string(),
            Self::ServerError => "Server error".to_string(),
            Self::DocumentNotFound => "Document not found".to_string(),
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
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = ErrorResponse {
            code: status_code.as_u16(),
            message: self.to_string(),
            error: self.name(),
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

impl From<r2d2::Error> for ApiError {
    fn from(_: Error) -> Self {
        ApiError::ServerError
    }
}
