use anyhow::Error;
use diesel_migrations::MigrationHarness;
use rumba::{
    db::{establish_connection, Pool},
    settings::SETTINGS,
};

use once_cell::sync::OnceCell;

const MIGRATIONS: diesel_migrations::EmbeddedMigrations = diesel_migrations::embed_migrations!();
static CONN_POOL: OnceCell<Pool> = OnceCell::new();

pub fn get_pool() -> &'static Pool {
    if let Some(val) = CONN_POOL.get() {
        val
    } else {
        CONN_POOL.set(establish_connection(&SETTINGS.db.uri));
        return CONN_POOL.get().unwrap();
    }
}

pub fn reset() -> Result<(), Error> {
    let mut connection = get_pool().get()?;

    connection
        .revert_all_migrations(MIGRATIONS)
        .expect("failed to revert migrations");
    connection
        .run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");
    Ok(())
}
