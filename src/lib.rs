use actix_web::{
    dev::{ServiceFactory, ServiceRequest},
    App, Error,
};

pub mod api;
pub mod db;
pub mod fxa;
pub mod settings;
pub mod util;

#[macro_use]
extern crate diesel;

pub fn add_services<T>(app: App<T>) -> App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = Error, InitError = ()>,
{
    app.service(api::healthz::healthz_app())
        .service(api::auth::auth_service())
        .service(api::admin::admin_service())
        .service(api::api_v1::api_v1_service())
}
