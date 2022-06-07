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
}
