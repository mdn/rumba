use actix_web::dev::HttpServiceFactory;
use actix_web::web;
use actix_web::HttpRequest;
use actix_web::HttpResponse;

use crate::api::error::ApiError;

async fn healthz(_: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().finish()
}

async fn error(_req: HttpRequest) -> Result<String, ApiError> {
    Err(ApiError::Artificial)
}

pub fn healthz_app() -> impl HttpServiceFactory {
    web::scope("/healthz")
        .service(web::resource("").to(healthz))
        .service(web::resource("/error").to(error))
}
