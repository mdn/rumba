use crate::helpers::api_assertions::assert_created_with_json_containing;
use crate::helpers::{
    api_assertions::assert_ok_with_json_containing, app::init_test, db::get_pool,
    http_client::PostPayload,
};
use anyhow::Error;
use chrono::NaiveDateTime;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use rumba::db::schema;
use serde_json::json;

#[actix_rt::test]
async fn test_adding_to_default_collection_updates_last_modifed() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let collections_base_url = "/api/v2/collections/";
    let whoamibase = "/api/v1/whoami";
    let mut whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(whoami, json!({ "settings": null })).await;
    client
        .post(
            format!("{}{}/items/", collections_base_url, "1").as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Interesting CSS1",
                "url": "/en-US/docs/Web/CSS1"
            }
            ))),
        )
        .await;

    let pool = get_pool();
    let mut conn = pool.get()?;
    let created = schema::collections::table
        .select(schema::collections::created_at)
        .first::<NaiveDateTime>(&mut conn)?;

    whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(
        whoami,
        json!({"settings" : { "collections_last_modified_time" : created }}),
    )
    .await;

    client
        .post(
            format!("{}{}/items/{}", collections_base_url, "1", "1").as_str(),
            None,
            Some(PostPayload::Json(json!({
                "notes" : "Adding notes to test",
            }
            ))),
        )
        .await;

    let updated = schema::collections::table
        .select(schema::collections::updated_at)
        .first::<NaiveDateTime>(&mut conn)?;

    whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(
        whoami,
        json!({"settings" : { "collections_last_modified_time" : updated }}),
    )
    .await;

    Ok(())
}

#[actix_rt::test]
async fn test_deleting_from_default_updates_last_modified() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let collections_base_url = "/api/v2/collections/";
    let whoamibase = "/api/v1/whoami";
    let mut whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(whoami, json!({ "settings": null })).await;
    client
        .post(
            format!("{}{}/items/", collections_base_url, "1").as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Interesting CSS1",
                "url": "/en-US/docs/Web/CSS1"
            }
            ))),
        )
        .await;

    let pool = get_pool();
    let mut conn = pool.get()?;
    let (id, created) = schema::collections::table
        .select((schema::collections::id, schema::collections::created_at))
        .first::<(i64, NaiveDateTime)>(&mut conn)?;

    whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(
        whoami,
        json!({"settings" : { "collections_last_modified_time" : created }}),
    )
    .await;

    client
        .delete(
            format!("{}{}/items/{}/", collections_base_url, "1", "1").as_str(),
            None,
        )
        .await;

    let deleted_at = schema::collections::table
        .select(schema::collections::deleted_at)
        .filter(schema::collections::id.eq(id))
        .first::<Option<NaiveDateTime>>(&mut conn)?;

    whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(
        whoami,
        json!({"settings" : { "collections_last_modified_time" : deleted_at.unwrap() }}),
    )
    .await;

    Ok(())
}

#[actix_rt::test]
async fn test_adding_deleting_to_collections_v1_updates_last_modifed() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v1/plus/collection/?url=/en-US/docs/Web/CSS";
    let whoamibase = "/api/v1/whoami";
    let mut whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(whoami, json!({ "settings": null })).await;

    client
        .post(
            base_url,
            None,
            Some(PostPayload::FormData(json!({
                "name": "CSS: Cascading Style Sheets",
                "notes": "Notes notes notes",
            }))),
        )
        .await;

    let pool = get_pool();
    let mut conn = pool.get()?;
    let (id, created) = schema::collections::table
        .select((schema::collections::id, schema::collections::created_at))
        .first::<(i64, NaiveDateTime)>(&mut conn)?;

    whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(
        whoami,
        json!({"settings" : { "collections_last_modified_time" : created }}),
    )
    .await;

    client
        .post(
            base_url,
            None,
            Some(PostPayload::FormData(json!({
                "notes": "NEW Notes notes notes",
            }))),
        )
        .await;
    let updated_at = schema::collections::table
        .select(schema::collections::created_at)
        .filter(schema::collections::id.eq(id))
        .first::<NaiveDateTime>(&mut conn)?;
    whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(
        whoami,
        json!({"settings" : { "collections_last_modified_time" : updated_at }}),
    )
    .await;

    client
        .post(
            base_url,
            None,
            Some(PostPayload::FormData(json!({
                "delete": true,
            }))),
        )
        .await;
    let deleted_at = schema::collections::table
        .select(schema::collections::deleted_at)
        .filter(schema::collections::id.eq(id))
        .first::<Option<NaiveDateTime>>(&mut conn)?;

    whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(
        whoami,
        json!({"settings" : { "collections_last_modified_time" : deleted_at.unwrap() }}),
    )
    .await;

    Ok(())
}

#[actix_rt::test]
async fn test_operations_on_other_collections_not_update_last_modified() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let collections_base_url = "/api/v2/collections/";
    let whoamibase = "/api/v1/whoami";
    let mut whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(whoami, json!({ "settings": null })).await;
    let res = client
        .post(
            format!("{}", collections_base_url).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "name" : "Other collection",
            }
            ))),
        )
        .await;
    let new_collection =
        assert_created_with_json_containing(res, json!({"name" : "Other collection"})).await;
    client
        .post(
            format!(
                "{}{}/items/",
                collections_base_url,
                new_collection["id"].as_str().unwrap()
            )
            .as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Interesting CSS1",
                "url": "/en-US/docs/Web/CSS1"
            }
            ))),
        )
        .await;
    whoami = client.get(whoamibase, None).await;
    assert_ok_with_json_containing(whoami, json!({ "settings": null })).await;

    Ok(())
}
