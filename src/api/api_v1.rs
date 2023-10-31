use crate::api::ai_explain::{explain, explain_feedback};
use crate::api::ai_help::{
    ai_help, ai_help_delete_history, ai_help_feedback, ai_help_history, ai_help_list_history,
    ai_help_title_summary, quota,
};
use crate::api::experiments::{get_experiments, update_experiments};
use crate::api::info::information;
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
        .service(web::resource("/info").route(web::get().to(information)))
        .service(
            web::scope("/plus")
                .service(
                    web::scope("/ai")
                        .service(
                            web::scope("/help")
                                .service(web::resource("").route(web::post().to(ai_help)))
                                .service(web::resource("/quota").route(web::get().to(quota)))
                                .service(
                                    web::scope("/history")
                                        .service(
                                            web::resource("/list")
                                                .route(web::get().to(ai_help_list_history)),
                                        )
                                        .service(
                                            web::resource("/summary/{chat_id}")
                                                .route(web::post().to(ai_help_title_summary)),
                                        )
                                        .service(
                                            web::resource("/{chat_id}")
                                                .route(web::get().to(ai_help_history))
                                                .route(web::delete().to(ai_help_delete_history)),
                                        ),
                                )
                                .service(
                                    web::resource("/feedback")
                                        .route(web::post().to(ai_help_feedback)),
                                ),
                        )
                        // Keep for compat. TODO: remove.
                        .service(
                            web::scope("/ask")
                                .service(web::resource("").route(web::post().to(ai_help)))
                                .service(web::resource("/quota").route(web::get().to(quota)))
                                .service(
                                    web::resource("/feedback")
                                        .route(web::post().to(ai_help_feedback)),
                                ),
                        )
                        .service(
                            web::scope("/explain")
                                .service(web::resource("").route(web::post().to(explain)))
                                .service(
                                    web::resource("/feedback")
                                        .route(web::post().to(explain_feedback)),
                                ),
                        ),
                )
                .service(
                    web::scope("/settings")
                        .service(web::resource("/").route(web::post().to(update_settings)))
                        .service(
                            web::resource("/experiments/")
                                .route(web::post().to(update_experiments))
                                .route(web::get().to(get_experiments)),
                        ),
                )
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
            web::scope("/play")
                .app_data(json_cfg_1mb_limit)
                .service(web::resource("/").route(web::post().to(save)))
                .service(web::resource("/flag").route(web::post().to(flag)))
                .service(web::resource("/{gist_id}").route(web::get().to(load))),
        )
        .service(root_service())
}
