#[macro_use]
extern crate diesel;
extern crate core;
#[macro_use]
extern crate slog_scope;

use actix_web::{
    dev::{ServiceFactory, ServiceRequest},
    App, Error,
};

pub mod ai;
pub mod api;
pub mod db;
pub mod error;
pub mod fxa;
mod helpers;
pub mod ids;
pub mod logging;
pub mod metrics;
pub mod settings;
pub mod tags;
pub mod util;

pub fn add_services<T>(app: App<T>) -> App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = Error, InitError = ()>,
{
    app.service(api::healthz::healthz_app())
        .service(api::fxa_webhook::fxa_webhook_app())
        .service(api::auth::auth_service())
        .service(api::admin::admin_service())
        .service(api::api_v1::api_v1_service())
        .service(api::v2::api_v2::api_v2_service())
}
