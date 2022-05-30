use crate::api::collections::{collections, create_or_update_collection_item, delete_collection_item};
use crate::api::settings::update_settings;
use crate::api::whoami::whoami;
use crate::settings::SETTINGS;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::HttpServiceFactory;
use actix_web::web;

use super::notifications::{
    mark_all_as_read, mark_as_read, notifications, star_ids, toggle_starred, unstar_ids,
};

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
            web::scope("/notifications")
                .service(web::resource("/").route(web::get().to(notifications)))
                .service(web::resource("/{id}/mark-as-read/").route(web::post().to(mark_as_read)))
                .service(
                    web::resource("/all/mark-as-read/").route(web::post().to(mark_all_as_read)),
                )
                .service(
                    web::resource("/{id}/toggle-starred/").route(web::post().to(toggle_starred)),
                )
                .service(web::resource("/star-ids/").route(web::post().to(star_ids)))
                .service(web::resource("/unstar-ids/").route(web::post().to(unstar_ids))),
        )
        .service(web::resource("/whoami").route(web::get().to(whoami)))
}
