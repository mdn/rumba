use crate::api::ai::ask;
use crate::api::newsletter::{
    is_subscribed, subscribe_anonymous_handler, subscribe_handler, unsubscribe_handler,
};
use crate::api::ping::ping;
use crate::api::play::{flag, load, save};
use crate::api::root::root_service;
use crate::api::search::search;
use crate::api::settings::update_settings;
use crate::api::whoami::whoami;
use actix_web::dev::HttpServiceFactory;
use actix_web::web;

pub fn api_v1_service() -> impl HttpServiceFactory {
    let json_cfg_1mb_limit = web::JsonConfig::default()
        // limit request payload size to 1MB
        .limit(1_048_576);
    web::scope("/api/v1")
        .service(
            web::scope("/plus")
                .service(web::resource("/settings/").route(web::post().to(update_settings)))
                .service(
                    web::resource("/newsletter/")
                        .route(web::get().to(is_subscribed))
                        .route(web::delete().to(unsubscribe_handler))
                        .route(web::post().to(subscribe_handler)),
                ),
        )
        .service(web::resource("/search").route(web::get().to(search)))
        .service(web::resource("/whoami").route(web::get().to(whoami)))
        .service(web::resource("/ping").route(web::post().to(ping)))
        .service(web::resource("/newsletter").route(web::post().to(subscribe_anonymous_handler)))
        .service(web::scope("/ai").service(web::resource("/ask").route(web::post().to(ask))))
        .service(
            web::scope("/play")
                .app_data(json_cfg_1mb_limit)
                .service(web::resource("/").route(web::post().to(save)))
                .service(web::resource("/flag").route(web::post().to(flag)))
                .service(web::resource("/{gist_id}").route(web::get().to(load))),
        )
        .service(root_service())
}
