use actix_http::body::{BoxBody, EitherBody};
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_rt::Arbiter;
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
use rumba::logging::init_logging;
use rumba::settings::SETTINGS;

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
    let pool = Data::new(get_pool().clone());
    let login_manager = Data::new(LoginManager::init().await?);
    let client = Data::new(Client::new());
    init_logging(!SETTINGS.logging.human_logs);
    let policy = CookieIdentityPolicy::new(&[0; 32])
        .name(&SETTINGS.auth.auth_cookie_name)
        .secure(SETTINGS.auth.auth_cookie_secure);
    let arbiter = Arbiter::new();
    let arbiter_handle = Data::new(arbiter.handle());

    let app = App::new()
        .wrap(IdentityService::new(policy))
        .app_data(Data::clone(&arbiter_handle))
        .app_data(Data::clone(&pool))
        .app_data(Data::clone(&client))
        .app_data(Data::clone(&login_manager));
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
