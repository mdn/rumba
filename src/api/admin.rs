use crate::db::v2::ai_history::delete_old_ai_history;
use crate::db::v2::synchronize_bcd_updates_db::update_bcd;
use crate::settings::SETTINGS;
use actix_web::dev::{HttpServiceFactory, ServiceRequest};
use actix_web::{web, Error};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;

use super::error::ApiError;

pub async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    if credentials.token() == SETTINGS.auth.admin_update_bearer_token {
        Ok(req)
    } else {
        Err((Error::from(ApiError::InvalidBearer), req))
    }
}

pub fn admin_service() -> impl HttpServiceFactory {
    web::scope("/admin-api")
        .wrap(HttpAuthentication::bearer(validator))
        .service(web::resource("/v2/updates/").route(web::post().to(update_bcd)))
        .service(web::resource("/v2/ai-history/").route(web::post().to(delete_old_ai_history)))
}
