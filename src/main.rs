#![warn(clippy::all)]
#[macro_use]
extern crate diesel_migrations;

use std::sync::{Arc, RwLock};

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use log::{debug, info};
use rumba::{add_services, db, fxa::LoginManager, settings::SETTINGS};

embed_migrations!();

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    info!("starting…");
    debug!("DEBUG logging enabled");

    let pool = db::establish_connection(&SETTINGS.db.uri);
    embedded_migrations::run_with_output(&pool.get()?, &mut std::io::stdout())?;
    let login_manager = Arc::new(RwLock::new(LoginManager::init().await?));

    HttpServer::new(move || {
        let policy = CookieIdentityPolicy::new(&[0; 32])
            .name(&SETTINGS.auth.auth_cookie_name)
            .secure(SETTINGS.auth.auth_cookie_secure);
        let app = App::new()
            .wrap(Logger::default().exclude("/healthz"))
            .wrap(IdentityService::new(policy))
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(login_manager.clone()));
        add_services(app)
    })
    .bind(("0.0.0.0", SETTINGS.server.port))?
    .run()
    .await?;
    Ok(())
}
