use crate::api::collections::{collections, create_or_update_collection_item, delete_collection_item};
use crate::api::settings::update_settings;
use crate::api::whoami::whoami;
use crate::settings::SETTINGS;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::HttpServiceFactory;
use actix_web::web;

use super::notifications::notifications;

pub fn api_v1_service() -> impl HttpServiceFactory {
    web::scope("/api/v1")
        .wrap(SessionMiddleware::new(
            CookieSessionStore::default(),
            Key::from(&SETTINGS.auth.auth_cookie_key),
        ))
        .service(
            web::scope("/plus")
                .service(
                    web::resource("/collection/")
                        .route(web::get().to(collections))
                        .route(web::post().to(create_or_update_collection_item))
                        .route(web::delete().to(delete_collection_item)),
                )
                .service(web::resource("/settings/").route(web::post().to(update_settings))),
        )
        .service(
            web::resource("/notifications")
                .route(web::get().to(notifications))                
        )

        .service(web::resource("/whoami").route(web::get().to(whoami)))
}
