use actix_http::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FxaError {
    #[error(transparent)]
    Oidc(#[from] anyhow::Error),
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
    #[error(transparent)]
    DbConnectionError(#[from] r2d2::Error),
    #[error(transparent)]
    DbResultError(#[from] diesel::result::Error),
    #[error(transparent)]
    BlockingError(#[from] actix_web::error::BlockingError),
    #[error("Error fetching user info: {0}")]
    UserInfoError(
        #[from] openidconnect::UserInfoError<openidconnect::HttpClientError<reqwest::Error>>,
    ),
    #[error("Bad status getting user info: {0}")]
    UserInfoBadStatus(StatusCode),
    #[error("Error deserializing user info: {0}")]
    UserInfoDeserialize(#[from] serde_json::Error),
    #[error("Id token missing")]
    IdTokenMissing,
}
