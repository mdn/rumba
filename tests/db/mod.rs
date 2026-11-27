use anyhow::Error;
use rumba::db::{server_major_version, POSTGRES_MAJOR_VERSION};

use crate::helpers::db::get_pool;

#[test]
fn test_server_matches_expected_major_version() -> Result<(), Error> {
    let mut conn = get_pool().get()?;
    let major = server_major_version(&mut conn)?;
    assert_eq!(major, POSTGRES_MAJOR_VERSION);
    Ok(())
}
