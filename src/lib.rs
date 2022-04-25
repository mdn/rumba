use actix_web::{
    dev::{ServiceFactory, ServiceRequest},
    App, Error,
};

pub mod api;
pub mod db;
pub mod fxa;
pub mod settings;

pub fn add_services<T>(app: App<T>) -> App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = Error, InitError = ()>,
{
    app.service(api::healthz::healthz_app())
        .service(api::auth::auth_service())
}
