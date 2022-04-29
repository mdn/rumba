use anyhow::Error;
use diesel_migrations::MigrationHarness;
use rumba::{
    db::{establish_connection, Pool},
    settings::SETTINGS,
};

const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    diesel_migrations::embed_migrations!();

pub fn get_pool() -> Pool {
    establish_connection(&SETTINGS.db.uri)
}

pub fn reset() -> Result<(), Error> {
    let mut connection = get_pool().get()?;

    connection.revert_all_migrations(MIGRATIONS).expect("failed to revert migrations");
    connection.run_pending_migrations(MIGRATIONS).expect("failed to run migrations");
    Ok(())
}
