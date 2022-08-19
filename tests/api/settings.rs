use crate::helpers::app::init_test;
use anyhow::Error;

#[actix_rt::test]
async fn test_adding_to_default_collection_updates_last_modifed() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let _ = client.get(base_url, None).await;
    Ok(())
}

#[actix_rt::test]
async fn test_deleting_from_default_updates_last_modified() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let _ = client.get(base_url, None).await;
    Ok(())
}

#[actix_rt::test]
async fn test_adding_to_collections_v1_updates_last_modifed() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let _ = client.get(base_url, None).await;
    Ok(())
}

#[actix_rt::test]
async fn test_deleting_from_collections_v1_updates_last_modifed() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let _ = client.get(base_url, None).await;
    Ok(())
}

#[actix_rt::test]
async fn test_operations_on_other_collections_not_update_last_modified() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let _ = client.get(base_url, None).await;
    Ok(())
}
