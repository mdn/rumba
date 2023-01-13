use crate::api::newsletter::{is_subscribed, subscribe_handler, unsubscribe_handler};
use crate::api::ping::ping;
use crate::api::root::root_service;
use crate::api::search::search;
use crate::api::settings::update_settings;
use crate::api::whoami::whoami;
use actix_web::dev::HttpServiceFactory;
use actix_web::web;

use super::notifications::{
    delete_by_id, delete_many, mark_all_as_read, mark_as_read, notifications, star_ids,
    toggle_starred, undo_delete_by_id, unstar_ids,
};
use super::watched_items::{get_watched_items, unwatch_many, update_watched_item};

pub fn api_v1_service() -> impl HttpServiceFactory {
    web::scope("/api/v1")
        .service(
            web::scope("/plus")
                .service(web::resource("/settings/").route(web::post().to(update_settings)))
                .service(
                    web::resource("/newsletter/")
                        .route(web::get().to(is_subscribed))
                        .route(web::delete().to(unsubscribe_handler))
                        .route(web::post().to(subscribe_handler)),
                )
                .service(
                    web::scope("/notifications")
                        .service(web::resource("/").route(web::get().to(notifications)))
                        .service(
                            web::resource("/all/mark-as-read/")
                                .route(web::post().to(mark_all_as_read)),
                        )
                        .service(
                            web::resource("/{id}/mark-as-read/")
                                .route(web::post().to(mark_as_read)),
                        )
                        .service(
                            web::resource("/{id}/toggle-starred/")
                                .route(web::post().to(toggle_starred)),
                        )
                        .service(web::resource("/star-ids/").route(web::post().to(star_ids)))
                        .service(web::resource("/unstar-ids/").route(web::post().to(unstar_ids)))
                        .service(web::resource("/{id}/delete/").route(web::post().to(delete_by_id)))
                        .service(
                            web::resource("/{id}/undo-deletion/")
                                .route(web::post().to(undo_delete_by_id)),
                        )
                        .service(web::resource("/delete-ids/").route(web::post().to(delete_many))),
                )
                .service(
                    web::resource("/watching/")
                        .route(web::post().to(update_watched_item))
                        .route(web::get().to(get_watched_items)),
                )
                .service(web::resource("/unwatch-many/").route(web::post().to(unwatch_many))),
        )
        .service(web::resource("/search").route(web::get().to(search)))
        .service(web::resource("/whoami").route(web::get().to(whoami)))
        .service(web::resource("/ping").route(web::post().to(ping)))
        .service(root_service())
}
