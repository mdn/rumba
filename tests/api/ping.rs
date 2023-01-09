use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::{PostPayload, TestHttpClient};
use crate::helpers::wait_for_stubr;
use actix_web::test;
use anyhow::Error;
use diesel::prelude::*;
use rumba::db::schema;
use serde_json::{json, Value};

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_empty() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    let res = logged_in_client
        .post("/api/v1/ping", None, Some(PostPayload::Form(json!({}))))
        .await;
    assert_eq!(res.response().status(), 201);

    let mut conn = pool.get()?;
    let activity_data = schema::activity_pings::table
        .select(schema::activity_pings::activity)
        .first::<Value>(&mut conn)?;
    assert_eq!(activity_data, json!({ "subscription_type": "mdn_plus_5m" }));

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_garbage() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    let res = logged_in_client
        .post(
            "/api/v1/ping",
            None,
            Some(PostPayload::Form(json!({ "foobar": "barfoo" }))),
        )
        .await;
    assert_eq!(res.response().status(), 201);

    let mut conn = pool.get()?;
    let activity_data = schema::activity_pings::table
        .select(schema::activity_pings::activity)
        .first::<Value>(&mut conn)?;
    assert_eq!(activity_data, json!({ "subscription_type": "mdn_plus_5m" }));

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_offline_disabled() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    let res = logged_in_client
        .post(
            "/api/v1/ping",
            None,
            Some(PostPayload::Form(json!({ "offline": false }))),
        )
        .await;
    assert_eq!(res.response().status(), 201);

    let mut conn = pool.get()?;
    let activity_data = schema::activity_pings::table
        .select(schema::activity_pings::activity)
        .first::<Value>(&mut conn)?;
    assert_eq!(activity_data, json!({ "subscription_type": "mdn_plus_5m" }));

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_offline_enabled() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    let res = logged_in_client
        .post(
            "/api/v1/ping",
            None,
            Some(PostPayload::Form(json!({ "offline": true }))),
        )
        .await;
    assert_eq!(res.response().status(), 201);

    let mut conn = pool.get()?;
    let activity_data = schema::activity_pings::table
        .select(schema::activity_pings::activity)
        .first::<Value>(&mut conn)?;
    assert_eq!(
        activity_data,
        json!({ "subscription_type": "mdn_plus_5m", "offline": true })
    );

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_offline_upsert() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    let res1 = logged_in_client
        .post(
            "/api/v1/ping",
            None,
            Some(PostPayload::Form(json!({ "offline": false }))),
        )
        .await;
    assert_eq!(res1.response().status(), 201);

    let mut conn = pool.get()?;
    let activity_data1 = schema::activity_pings::table
        .select(schema::activity_pings::activity)
        .first::<Value>(&mut conn)?;
    assert_eq!(
        activity_data1,
        json!({ "subscription_type": "mdn_plus_5m" })
    );

    let res2 = logged_in_client
        .post(
            "/api/v1/ping",
            None,
            Some(PostPayload::Form(json!({ "offline": true }))),
        )
        .await;
    assert_eq!(res2.response().status(), 201);

    let activity_data2 = schema::activity_pings::table
        .select(schema::activity_pings::activity)
        .first::<Value>(&mut conn)?;
    assert_eq!(
        activity_data2,
        json!({ "subscription_type": "mdn_plus_5m", "offline": true })
    );

    let res3 = logged_in_client
        .post(
            "/api/v1/ping",
            None,
            Some(PostPayload::Form(json!({ "offline": false }))),
        )
        .await;
    assert_eq!(res3.response().status(), 201);

    let activity_data3 = schema::activity_pings::table
        .select(schema::activity_pings::activity)
        .first::<Value>(&mut conn)?;
    assert_eq!(
        activity_data3,
        json!({ "subscription_type": "mdn_plus_5m", "offline": true })
    );

    let count = schema::activity_pings::table
        .count()
        .first::<i64>(&mut conn)?;
    assert_eq!(count, 1);

    drop(stubr);
    Ok(())
}
