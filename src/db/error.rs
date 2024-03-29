use crate::fxa::error::FxaError;
use r2d2::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error(transparent)]
    DieselResult(diesel::result::Error),
    #[error(transparent)]
    Conflict(diesel::result::Error),
    #[error(transparent)]
    NotFound(diesel::result::Error),
    #[error(transparent)]
    R2D2Error(r2d2::Error),
    #[error(transparent)]
    FxAError(#[from] FxaError),
    #[error("Json error")]
    JsonProcessingError,
}

impl From<r2d2::Error> for DbError {
    fn from(e: Error) -> Self {
        DbError::R2D2Error(e)
    }
}

impl From<diesel::result::Error> for DbError {
    fn from(e: diesel::result::Error) -> Self {
        match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => DbError::Conflict(e),
            diesel::result::Error::NotFound => DbError::NotFound(e),
            _ => DbError::DieselResult(e),
        }
    }
}
