use actix_http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use async_openai::error::OpenAIError;
use thiserror::Error;

use crate::error::ErrorResponse;

#[derive(Error, Debug)]
pub enum AIError {
    #[error("OpenAI error: {0}")]
    OpenAIError(#[from] OpenAIError),
    #[error("SqlXError: {0}")]
    SqlXError(#[from] sqlx::Error),
    #[error("Flagged content")]
    FlaggedError,
    #[error("No user prompt")]
    NoUserPrompt,
    #[error("Token limit reached")]
    TokenLimit,
    #[error("Tiktoken Error: {0}")]
    TiktokenError(#[from] anyhow::Error),
}

impl ResponseError for AIError {
    fn status_code(&self) -> StatusCode {
        match &self {
            AIError::OpenAIError(_) | AIError::SqlXError(_) | AIError::TiktokenError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AIError::FlaggedError | AIError::NoUserPrompt | AIError::TokenLimit => {
                StatusCode::BAD_REQUEST
            }
        }
    }

    fn error_response(&self) -> HttpResponse<actix_http::body::BoxBody> {
        let status_code = self.status_code();
        let mut builder = HttpResponse::build(status_code);
        builder.json(ErrorResponse {
            code: status_code.as_u16(),
            message: status_code.canonical_reason().unwrap_or("Unknown"),
            error: "AI Error",
        })
    }
}
