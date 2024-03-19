use crate::db::ai_history::do_delete_old_ai_history;
use crate::db::v2::synchronize_bcd_updates_db::update_bcd;
use crate::db::Pool;
use crate::settings::SETTINGS;
use actix_rt::ArbiterHandle;
use actix_web::dev::{HttpServiceFactory, ServiceRequest};
use actix_web::web::Data;
use actix_web::HttpResponse;
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
        .service(web::resource("/ai-history/").route(web::post().to(delete_old_ai_history)))
}

pub async fn delete_old_ai_history(
    pool: Data<Pool>,
    arbiter: Data<ArbiterHandle>,
) -> Result<HttpResponse, ApiError> {
    if !arbiter.spawn(async move {
        if let Err(e) = do_delete_old_ai_history(pool).await {
            error!("{}", e);
        }
    }) {
        return Ok(HttpResponse::InternalServerError().finish());
    }
    Ok(HttpResponse::Accepted().finish())
}
