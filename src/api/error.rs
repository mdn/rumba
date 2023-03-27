use crate::db::error::DbError;
use actix_http::header::HeaderValue;
use actix_web::http::header::HeaderName;
use actix_web::http::StatusCode;
use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::{HttpResponse, ResponseError};
use basket::BasketError;
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;
use validator::ValidationErrors;

pub const ERROR_ID_HEADER_NAME_STR: &str = "error-id";
static ERROR_ID_HEADER_NAME: HeaderName = HeaderName::from_static(ERROR_ID_HEADER_NAME_STR);

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
    #[error("Query string parsing failed for {key}: {message}")]
    Query { key: String, message: String },
}

#[derive(Error, Debug)]
pub enum FxaWebhookError {
    #[error("Json error: {0}")]
    JsonProcessing(#[from] serde_json::Error),
    #[error("Base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("Invalid SET")]
    InvalidSET,
    #[error("Invalid signature")]
    InvalidSignature(#[from] openidconnect::SignatureVerificationError),
}
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Artificial error")]
    Artificial,
    #[error("Unknown error")]
    Unknown,
    #[error("Invalid Session info")]
    InvalidSession,
    #[error("Database error")]
    ServerError,
    #[error("Document Not found")]
    DocumentNotFound,
    #[error("Collection with id {0} not found")]
    CollectionNotFound(String),
    #[error("Malformed Url")]
    MalformedUrl,
    #[error("Json error")]
    JsonProcessingError,
    #[error("Invalid Bearer")]
    InvalidBearer,
    #[error("Query error")]
    Query(#[from] actix_web::error::QueryPayloadError),
    #[error("Search error")]
    Search(#[from] SearchError),
    #[error("FxaWebhookError: {0}")]
    FxaWebhook(FxaWebhookError),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Blocking error")]
    BlockingError(#[from] actix_web::error::BlockingError),
    #[error("DB Error: {0}")]
    DbError(#[from] DbError),
    #[error("Validation error: {0}")]
    ValidationError(#[from] ValidationErrors),
    #[error("Subscription limit reached")]
    MultipleCollectionSubscriptionLimitReached,
    #[error("Login Required")]
    LoginRequiredForFeature(String),
    #[error("Newsletter error: {0}")]
    BasketError(#[from] BasketError),
    #[error("Unknown error: {0}")]
    Generic(String),
}

impl ApiError {
    pub fn name(&self) -> &str {
        match self {
            Self::Artificial => "Artificial",
            Self::Unknown => "Unknown",
            Self::InvalidSession => "Invalid Session",
            Self::ServerError => "Server error",
            Self::DocumentNotFound => "Document not found",
            Self::InvalidBearer => "Invalid bearer info",
            Self::MalformedUrl => "Malformed URL",
            Self::JsonProcessingError => "Error processing JSON document",
            Self::Query(_) => "Query error",
            Self::Search(_) => "Search error",
            Self::FxaWebhook(_) => "FxaWebhookError",
            Self::Unauthorized => "Unauthorized",
            Self::BlockingError(_) => "Blocking error",
            Self::CollectionNotFound(_) => "Collection not found",
            Self::DbError(_) => "DB error",
            Self::ValidationError(_) => "Validation Error",
            Self::MultipleCollectionSubscriptionLimitReached => "Subscription limit reached",
            Self::BasketError(_) => "Error managing newsletter",
            Self::Generic(err) => err,
            Self::LoginRequiredForFeature(_) => "Login Required",
        }
    }
}

#[derive(Serialize)]
struct ErrorResponse<'a> {
    code: u16,
    error: &'a str,
    message: &'a str,
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::InvalidSession => StatusCode::BAD_REQUEST,
            Self::DocumentNotFound => StatusCode::NOT_FOUND,
            Self::InvalidBearer => StatusCode::FORBIDDEN,
            Self::MalformedUrl => StatusCode::BAD_REQUEST,
            Self::Query(_) => StatusCode::BAD_REQUEST,
            Self::Search(SearchError::Query { .. }) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::CollectionNotFound(_) => StatusCode::BAD_REQUEST,
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::MultipleCollectionSubscriptionLimitReached => StatusCode::BAD_REQUEST,
            Self::LoginRequiredForFeature(_) => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let mut builder = HttpResponse::build(status_code);
        match self {
            Self::Search(SearchError::Query { key, message }) => builder.json(json!({
                "errors": {
                    key: [
                        {
                            "message": message,
                            "code": "invalid",
                        }
                    ]
                }
            })),
            ApiError::CollectionNotFound(id) => builder.json(ErrorResponse {
                code: status_code.as_u16(),
                message: format!("Collection with id {} not found", id).as_str(),
                error: self.name(),
            }),
            ApiError::ValidationError(errors) => builder.json(ErrorResponse {
                code: status_code.as_u16(),
                message: format!("Error validating input {0}", errors).as_str(),
                error: self.name(),
            }),
            ApiError::MultipleCollectionSubscriptionLimitReached => builder.json(ErrorResponse {
                code: status_code.as_u16(),
                message: "Subscription limit reached. Please upgrade",
                error: self.name(),
            }),
            ApiError::LoginRequiredForFeature(feature) => builder.json(ErrorResponse {
                code: status_code.as_u16(),
                message: format!("Please login to use feature: {0}", feature).as_str(),
                error: self.name(),
            }),
            _ if status_code == StatusCode::INTERNAL_SERVER_ERROR => builder.json(ErrorResponse {
                code: status_code.as_u16(),
                message: "internal server error",
                error: self.name(),
            }),
            _ => builder.json(ErrorResponse {
                code: status_code.as_u16(),
                message: &self.to_string(),
                error: self.name(),
            }),
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

fn log_error<B>(
    mut res: actix_web::dev::ServiceResponse<B>,
) -> actix_web::Result<ErrorHandlerResponse<B>> {
    if let Some(error) = res.response().error() {
        let uuid = Uuid::new_v4().as_hyphenated().to_string();
        let header_value =
            HeaderValue::from_str(&uuid).unwrap_or(HeaderValue::from_static("invalid-uuid"));
        warn!("{} - eid:{}", error, &uuid);
        res.headers_mut()
            .append(ERROR_ID_HEADER_NAME.clone(), header_value);
    }
    Ok(ErrorHandlerResponse::Response(res.map_into_left_body()))
}

pub fn error_handler<B>() -> ErrorHandlers<B>
where
    B: 'static,
{
    ErrorHandlers::new().handler(StatusCode::INTERNAL_SERVER_ERROR, log_error)
}
