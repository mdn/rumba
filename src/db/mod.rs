pub mod documents;
pub mod error;
pub mod fxa_webhook;
#[allow(clippy::extra_unused_lifetimes)]
pub mod model;
pub mod ping;
#[allow(unused_imports)]
pub mod schema;
pub mod schema_manual;
pub mod settings;
pub mod types;
pub mod users;
pub mod v2;

use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use sqlx::postgres::PgPoolOptions;

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
    PgPoolOptions::new()
    .max_connections(25)
    .connect(database_url).await.expect("Failed to create supa pool")
}

