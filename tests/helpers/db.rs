use anyhow::Error;
use diesel_migrations::revert_latest_migration;
use rumba::{
    db::{establish_connection, Pool},
    settings::SETTINGS,
};

embed_migrations!();

pub fn get_pool() -> Pool {
    establish_connection(&SETTINGS.db.uri)
}

pub fn reset() -> Result<(), Error> {
    let connection = get_pool().get()?;
    while revert_latest_migration(&connection).is_ok() {}

    embedded_migrations::run(&connection).expect("error running migrations");
    Ok(())
}
