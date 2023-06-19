use actix_http::body::BoxBody;
use actix_http::Request;
use actix_identity::IdentityMiddleware;
use actix_rt::Arbiter;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::Service;
use actix_web::test;
use actix_web::web::Data;
use actix_web::{
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    App, Error,
};
use basket::Basket;
use elasticsearch::http::transport::Transport;
use elasticsearch::Elasticsearch;
use octocrab::OctocrabBuilder;
use reqwest::Client;
use rumba::add_services;
use rumba::api::error::error_handler;
use rumba::db::{Pool, SupaPool};
use rumba::fxa::LoginManager;
use rumba::settings::SETTINGS;
use slog::{slog_o, Drain};
use stubr::{Config, Stubr};

use super::db::reset;
use super::http_client::TestHttpClient;
use super::RumbaTestResponse;

pub async fn test_app(
    pool: &Pool,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<BoxBody>,
        Error = Error,
        Config = (),
        InitError = (),
    >,
> {
    let app = App::new().app_data(pool.clone());
    add_services(app)
}

pub async fn test_app_with_login(
    pool: &Pool,
) -> anyhow::Result<
    App<
        impl ServiceFactory<
            ServiceRequest,
            Response = RumbaTestResponse,
            Error = Error,
            Config = (),
            InitError = (),
        >,
    >,
> {
    let pool = Data::new(pool.clone());
    let login_manager = Data::new(LoginManager::init().await?);
    let client = Data::new(Client::new());
    init_logging();
    let arbiter = Arbiter::new();
    let arbiter_handle = Data::new(arbiter.handle());
    let session_cookie_key = Key::derive_from(&SETTINGS.auth.cookie_key);
    let github_client = Data::new(Some(
        OctocrabBuilder::new()
            .base_uri("http://localhost:4321")
            .unwrap()
            .build()?,
    ));
    let basket_client = Data::new(
        SETTINGS
            .basket
            .as_ref()
            .map(|b| Basket::new(&b.api_key, b.basket_url.clone())),
    );

    let openai_client = Data::new(None::<async_openai::Client>);
    let supabase_pool = Data::new(None::<SupaPool>);

    let app = App::new()
        .wrap(error_handler())
        .wrap(IdentityMiddleware::default())
        .wrap(
            SessionMiddleware::builder(CookieSessionStore::default(), session_cookie_key)
                .cookie_name(SETTINGS.auth.auth_cookie_name.clone())
                .cookie_secure(false)
                .build(),
        )
        .app_data(Data::clone(&arbiter_handle))
        .app_data(Data::clone(&openai_client))
        .app_data(Data::clone(&supabase_pool))
        .app_data(Data::clone(&github_client))
        .app_data(Data::clone(&pool))
        .app_data(Data::clone(&client))
        .app_data(Data::clone(&basket_client))
        .app_data(Data::clone(&login_manager));
    Ok(add_services(app))
}

pub async fn test_app_only_search() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<BoxBody>,
        Error = Error,
        Config = (),
        InitError = (),
    >,
> {
    let elastic_transport = Transport::single_node("http://localhost:4321").unwrap();
    let elastic_client = Elasticsearch::new(elastic_transport);

    let app = App::new().app_data(Data::new(elastic_client));
    add_services(app)
}

fn init_logging() {
    let decorator = slog_term::PlainSyncDecorator::new(slog_term::TestStdoutWriter);
    let drain = std::sync::Mutex::new(slog_envlogger::new(
        slog_term::FullFormat::new(decorator).build(),
    ))
    .fuse();
    let logger = slog::Logger::root(drain, slog_o!());

    // XXX: cancel slog_scope's NoGlobalLoggerSet for now, it's difficult to
    // prevent it from potentially panicing during tests. reset_logging resets
    // the global logger during shutdown anyway:
    // https://github.com/slog-rs/slog/issues/169
    slog_scope::set_global_logger(logger).cancel_reset();
    slog_stdlog::init().ok();
}

pub async fn init_test(
    custom_stubs: Vec<&str>,
) -> Result<
    (
        TestHttpClient<
            impl Service<Request, Response = RumbaTestResponse, Error = actix_web::Error>,
        >,
        Stubr,
    ),
    anyhow::Error,
> {
    let pool = reset()?;
    let stubr = Stubr::start_blocking_with(
        custom_stubs,
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: true,
            verify: false,
        },
    );
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let logged_in_client = TestHttpClient::new(service).await;
    Ok((logged_in_client, stubr))
}
