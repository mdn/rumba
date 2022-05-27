#![warn(clippy::all)]

use std::sync::Arc;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use diesel_migrations::MigrationHarness;
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

    let http_client = HttpClient::new();
    let login_manager = Arc::new(LoginManager::init(http_client.clone()).await?);

    HttpServer::new(move || {
        let policy = CookieIdentityPolicy::new(&[0; 32])
            .name(&SETTINGS.auth.auth_cookie_name)
            .secure(SETTINGS.auth.auth_cookie_secure);
        let app = App::new()
            .wrap(Logger::default().exclude("/healthz"))
            .wrap(IdentityService::new(policy))
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(http_client.clone()))
            .app_data(Data::new(login_manager.clone()));
        add_services(app)
    })
    .bind(("0.0.0.0", SETTINGS.server.port))?
    .run()
    .await?;
    Ok(())
}
