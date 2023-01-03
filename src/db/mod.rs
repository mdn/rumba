pub mod documents;
pub mod error;
pub mod fxa_webhook;
#[allow(clippy::extra_unused_lifetimes)]
pub mod model;
pub mod notifications;
#[allow(unused_imports)]
pub mod schema;
pub mod settings;
pub mod types;
pub mod users;
pub mod v2;
pub mod watched_items;

use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection(database_url: &str) -> Pool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .max_size(25)
        .build(manager)
        .expect("Failed to create pool.")
}
