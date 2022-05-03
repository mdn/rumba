use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;
use serde::{Serialize};

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("unknown error")]
    Unknown,
    #[error("Invalid Session info")]
    InvalidSession,
}

impl ApiError {
    pub fn name(&self) -> String {
        match self {
            Self::Unknown => "Unknown".to_string(),
            Self::InvalidSession => "Invalid Session".to_string(),
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
