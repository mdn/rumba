use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::TestHttpClient;
use crate::helpers::{read_json, wait_for_stubr};
use actix_http::StatusCode;
use actix_web::test;
use anyhow::Error;
use assert_json_diff::assert_json_eq;
use diesel::prelude::*;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rumba::db::model::PlaygroundQuery;
use rumba::db::schema;
use serde_json::json;

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_playground() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut client = TestHttpClient::new(service).await;
    let save = client
        .post(
            "/api/v1/play/",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "html":"<h1>foo</h1>",
                "css":"h1 { font-size: 4rem; }",
                "js":"const foo = 1;","src":null
            }))),
        )
        .await;
    assert_eq!(save.status(), 201);
    let json = read_json(save).await;
    assert!(json["id"].is_string());
    let gist_id = json["id"].as_str().unwrap();
    let load = client
        .get(
            &format!(
                "/api/v1/play/{}",
                utf8_percent_encode(gist_id, NON_ALPHANUMERIC)
            ),
            None,
        )
        .await;
    assert_eq!(load.status(), 200);
    let json = read_json(load).await;
    assert_json_eq!(
        json,
        json!({"html":"<h1>foo</h1>","css":"h1 { font-size: 4rem; }","js":"const foo = 1;","src":null})
    );

    let mut conn = pool.get()?;
    let user_id = schema::users::table
        .filter(schema::users::fxa_uid.eq("TEST_SUB"))
        .select(schema::users::id)
        .first::<i64>(&mut conn)?;
    let d = diesel::delete(schema::users::table.filter(schema::users::id.eq(user_id)))
        .execute(&mut conn)?;
    assert_eq!(d, 1);
    let playground: PlaygroundQuery = schema::playground::table.first(&mut conn)?;
    assert_eq!(playground.user_id, None);
    assert_eq!(playground.deleted_user_id, Some(user_id));
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_invalid_id() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut client = TestHttpClient::new(service).await;
    let res = client.get("/api/v1/play/sssieddidxsx", None).await;
    // This used to panic, now it should just 400
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
