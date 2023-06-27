use async_openai::error::OpenAIError;
use thiserror::Error;

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
