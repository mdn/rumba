#![warn(clippy::all)]

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_rt::Arbiter;
use actix_web::{cookie::SameSite, middleware::Logger, web::Data, App, HttpServer};
use diesel_migrations::MigrationHarness;
use elasticsearch::http::transport::Transport;
use elasticsearch::Elasticsearch;
use log::{debug, info};
use reqwest::Client as HttpClient;
use rumba::{add_services, db, fxa::LoginManager, settings::SETTINGS};

const MIGRATIONS: diesel_migrations::EmbeddedMigrations = diesel_migrations::embed_migrations!();

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
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

    HttpServer::new(move || {
        let policy = CookieIdentityPolicy::new(&SETTINGS.auth.auth_cookie_key)
            .name(&SETTINGS.auth.auth_cookie_name)
            .secure(SETTINGS.auth.auth_cookie_secure)
            .same_site(SameSite::Strict);
        let app = App::new()
            .wrap(Logger::default().exclude("/healthz"))
            .wrap(IdentityService::new(policy))
            .app_data(Data::clone(&pool))
            .app_data(Data::clone(&arbiter_handle))
            .app_data(Data::clone(&http_client))
            .app_data(Data::clone(&login_manager))
            .app_data(Data::clone(&elastic_client));
        add_services(app)
    })
    .bind(("0.0.0.0", SETTINGS.server.port))?
    .run()
    .await?;
    Ok(())
}
