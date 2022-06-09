use actix_http::body::{BoxBody, EitherBody};
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::web::Data;
use actix_web::{
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    App, Error,
};
use elasticsearch::http::transport::Transport;
use elasticsearch::Elasticsearch;
use reqwest::Client;
use rumba::add_services;
use rumba::fxa::LoginManager;
use rumba::settings::SETTINGS;
use std::sync::Arc;

use super::{db::get_pool, identity::TestIdentityPolicy};

pub async fn test_app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<EitherBody<BoxBody>>,
        Error = Error,
        Config = (),
        InitError = (),
    >,
> {
    let pool = get_pool();
    let app = App::new()
        .wrap(IdentityService::new(TestIdentityPolicy::new()))
        .app_data(pool);
    add_services(app)
}

pub async fn test_app_with_login() -> anyhow::Result<
    App<
        impl ServiceFactory<
            ServiceRequest,
            Response = ServiceResponse<EitherBody<BoxBody>>,
            Error = Error,
            Config = (),
            InitError = (),
        >,
    >,
> {
    let pool = get_pool();
    let login_manager = Arc::new(LoginManager::init(Client::new()).await?);
    let _result = env_logger::try_init();
    let policy = CookieIdentityPolicy::new(&[0; 32])
        .name(&SETTINGS.auth.auth_cookie_name)
        .secure(SETTINGS.auth.auth_cookie_secure);

    let app = App::new()
        .wrap(IdentityService::new(policy))
        .app_data(Data::new(pool.clone()))
        .app_data(Data::new(Client::new()))
        .app_data(Data::new(login_manager));
    Ok(add_services(app))
}

pub async fn test_app_only_search() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<EitherBody<BoxBody>>,
        Error = Error,
        Config = (),
        InitError = (),
    >,
> {
    let elastic_transport = Transport::single_node("http://localhost:4321").unwrap();
    let elastic_client = Elasticsearch::new(elastic_transport);

    let app = App::new()
        .wrap(IdentityService::new(TestIdentityPolicy::new()))
        .app_data(Data::new(elastic_client));
    add_services(app)
}
