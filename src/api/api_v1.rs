use crate::api::collections::{collections, create_or_update_collections, delete_collection_item};
use crate::api::whoami::whoami;
use crate::settings::SETTINGS;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::HttpServiceFactory;
use actix_web::web;

pub fn api_v1_service() -> impl HttpServiceFactory {
    web::scope("/api/v1")
        .wrap(SessionMiddleware::new(
            CookieSessionStore::default(),
            Key::from(&SETTINGS.auth.auth_cookie_key),
        ))
        .service(
            web::scope("/plus").service(
                web::resource("/collection/")
                    .route(web::get().to(collections))
                    .route(web::post().to(create_or_update_collections))
                    .route(web::delete().to(delete_collection_item)),
            ),
        )
        .service(web::resource("/whoami").route(web::get().to(whoami)))
}