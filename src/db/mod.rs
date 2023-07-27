pub mod ai;
pub mod documents;
pub mod error;
pub mod experiments;
pub mod fxa_webhook;
#[allow(clippy::extra_unused_lifetimes)]
pub mod model;
pub mod ping;
pub mod play;
#[allow(unused_imports)]
pub mod schema;
pub mod schema_manual;
pub mod settings;
pub mod types;
pub mod users;
pub mod v2;

use std::str::FromStr;

use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions,
};

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection(database_url: &str) -> Pool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .max_size(25)
        .build(manager)
        .expect("Failed to create pool.")
}

pub type SupaPool = sqlx::PgPool;

pub async fn establish_supa_connection(database_url: &str) -> SupaPool {
    let options = PgConnectOptions::from_str(database_url)
        .expect("Failed to create supa connect options")
        .disable_statement_logging();
    PgPoolOptions::new()
        .max_connections(25)
        .connect_with(options)
        .await
        .expect("Failed to create supa pool")
}
