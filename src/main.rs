#![warn(clippy::all)]

use std::{rc::Rc, sync::Arc};

use actix_identity::IdentityMiddleware;
use actix_rt::Arbiter;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::{Key, SameSite},
    middleware::Logger,
    web::Data,
    App, HttpServer,
};
use const_format::formatcp;
use diesel_migrations::MigrationHarness;
use elasticsearch::http::transport::Transport;
use elasticsearch::Elasticsearch;
use reqwest::Client as HttpClient;
use rumba::{
    add_services,
    api::{
        error::{error_handler, ERROR_ID_HEADER_NAME_STR},
        session_migration_middleware::{CookieConfig, MigrateSessionCookie},
    },
    db,
    fxa::LoginManager,
    logging::{self, init_logging},
    metrics::{metrics_from_opts, MetricsData},
    settings::{Sentry, SETTINGS},
};
use slog_scope::{debug, info};

const MIGRATIONS: diesel_migrations::EmbeddedMigrations = diesel_migrations::embed_migrations!();

static LOG_FMT: &str = formatcp!(
    r#"%a "%r" %s %b "%{{Referer}}i" "%{{User-Agent}}i" eid:%{{{}}}o %T"#,
    ERROR_ID_HEADER_NAME_STR,
);

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    init_logging(!SETTINGS.logging.human_logs);
    info!("starting…");
    debug!("DEBUG logging enabled");

    let pool = db::establish_connection(&SETTINGS.db.uri);
    pool.get()?
        .run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");
    let pool = Data::new(pool);

    let http_client = Data::new(HttpClient::new());
    let login_manager = Data::new(LoginManager::init().await?);
    let arbiter = Arbiter::new();
    let arbiter_handle = Data::new(arbiter.handle());

    let elastic_transport = Transport::single_node(&SETTINGS.search.url)?;
    let elastic_client = Data::new(Elasticsearch::new(elastic_transport));
    let metrics = Data::new(MetricsData {
        client: Arc::new(metrics_from_opts()?),
    });
    let _guard = if let Some(Sentry { dsn }) = &SETTINGS.sentry {
        info!("initializing sentry");
        sentry::init(dsn.as_str())
    } else {
        sentry::init(sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        })
    };

    let session_cookie_key = Key::derive_from(&SETTINGS.auth.auth_cookie_key);

    HttpServer::new(move || {
        let app = App::new()
            .wrap(error_handler())
            .wrap(Logger::new(LOG_FMT).exclude("/healthz"))
            .wrap(sentry_actix::Sentry::new())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    session_cookie_key.clone(),
                )
                .cookie_name(SETTINGS.auth.auth_cookie_name.clone())
                .cookie_secure(SETTINGS.auth.auth_cookie_secure)
                .cookie_same_site(SameSite::Strict)
                .build(),
            )
            .wrap(MigrateSessionCookie {
                config: Rc::new(CookieConfig {
                    cookie_name: SETTINGS.auth.auth_cookie_name.clone(),
                    cookie_key: session_cookie_key.clone(),
                }),
            })
            .wrap(Logger::default().exclude("/healthz"))
            .app_data(Data::clone(&metrics))
            .app_data(Data::clone(&pool))
            .app_data(Data::clone(&arbiter_handle))
            .app_data(Data::clone(&http_client))
            .app_data(Data::clone(&login_manager))
            .app_data(Data::clone(&elastic_client));
        add_services(app)
    })
    .bind((SETTINGS.server.host.as_str(), SETTINGS.server.port))?
    .run()
    .await?;

    info!("Server closing");
    logging::reset_logging();

    Ok(())
}
