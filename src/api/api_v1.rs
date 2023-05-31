use crate::api::chat::{ask, explain_chat, generate_example};
use crate::api::newsletter::{
    is_subscribed, subscribe_anonymous_handler, subscribe_handler, unsubscribe_handler,
};
use crate::api::ping::ping;
use crate::api::root::root_service;
use crate::api::search::search;
use crate::api::settings::update_settings;
use crate::api::whoami::whoami;
use actix_web::dev::HttpServiceFactory;
use actix_web::web;

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
                ),
        )
        .service(web::resource("/search").route(web::get().to(search)))
        .service(web::resource("/whoami").route(web::get().to(whoami)))
        .service(web::resource("/ping").route(web::post().to(ping)))
        .service(web::resource("/newsletter").route(web::post().to(subscribe_anonymous_handler)))
        .service(
            web::scope("/chat")
                .service(web::resource("/explain").route(web::post().to(explain_chat)))
                .service(web::resource("/stream").route(web::post().to(ask)))
                .service(web::resource("/generate").route(web::post().to(generate_example))),
        )
        .service(root_service())
}
