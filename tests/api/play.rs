use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::TestHttpClient;
use crate::helpers::{read_json, wait_for_stubr};
use actix_web::test;
use anyhow::Error;
use assert_json_diff::assert_json_eq;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
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

    let load = client
        .get(
            &format!(
                "/api/v1/play/{}",
                utf8_percent_encode(json["id"].as_str().unwrap(), NON_ALPHANUMERIC)
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
    Ok(())
}
