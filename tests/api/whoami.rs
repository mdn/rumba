use crate::helpers::db::reset;
use crate::helpers::http_client::TestHttpClient;
use crate::helpers::{app::test_app_with_login, http_client::PostPayload};
use crate::helpers::{read_json, wait_for_stubr};
use actix_web::test;
use anyhow::Error;
use serde_json::json;
use stubr::{Config, Stubr};

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn whoami_anonymous_test() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await.unwrap();
    let service = test::init_service(app).await;
    let request = test::TestRequest::get()
        .uri("/api/v1/whoami")
        .insert_header(("CloudFront-Viewer-Country-Name", "Iceland"))
        .to_request();
    let whoami = test::call_service(&service, request).await;

    assert!(whoami.status().is_success());

    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country"], "Iceland");
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn whoami_logged_in_test() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("CloudFront-Viewer-Country-Name", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country"], "Iceland");

    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["is_authenticated"], true);
    assert_eq!(json["email"], "test@test.com");
    assert_eq!(
        json["avatar_url"],
        "https://i1.sndcdn.com/avatars-000460644402-0iiiub-t500x500.jpg"
    );
    assert_eq!(json["is_subscriber"], true);
    assert_eq!(
        json["subscription_type"], "mdn_plus_5m",
        "Subscription type wrong"
    );
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn whoami_settings_test() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("CloudFront-Viewer-Country-Name", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country"], "Iceland");

    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["is_authenticated"], true);
    assert_eq!(json["email"], "test@test.com");
    assert_eq!(
        json["avatar_url"],
        "https://i1.sndcdn.com/avatars-000460644402-0iiiub-t500x500.jpg"
    );
    assert_eq!(json["is_subscriber"], true);
    assert_eq!(
        json["subscription_type"], "mdn_plus_5m",
        "Subscription type wrong"
    );
    assert_eq!(json["settings"], json!(null));
    let settings = logged_in_client
        .post(
            "/api/v1/plus/settings/",
            None,
            Some(PostPayload::Json(
                json!({"col_in_search": true, "multiple_collections": true}),
            )),
        )
        .await;

    assert_eq!(settings.status(), 201);
    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("CloudFront-Viewer-Country-Name", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["settings"]["col_in_search"], true);
    assert_eq!(json["settings"]["locale_override"], serde_json::Value::Null);
    assert_eq!(json["settings"]["multiple_collections"], true);

    let settings = logged_in_client
        .post(
            "/api/v1/plus/settings/",
            None,
            Some(PostPayload::Json(
                json!({"col_in_search": false, "locale_override": "zh-TW"}),
            )),
        )
        .await;

    assert_eq!(settings.status(), 201);

    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("CloudFront-Viewer-Country-Name", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["settings"]["col_in_search"], false);
    assert_eq!(json["settings"]["locale_override"], "zh-TW");
    assert_eq!(json["settings"]["multiple_collections"], true);

    let settings = logged_in_client
        .post(
            "/api/v1/plus/settings/",
            None,
            Some(PostPayload::Json(json!({"multiple_collections": false}))),
        )
        .await;

    assert_eq!(settings.status(), 201);

    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("CloudFront-Viewer-Country-Name", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["settings"]["col_in_search"], false);
    assert_eq!(json["settings"]["locale_override"], "zh-TW");
    assert_eq!(json["settings"]["multiple_collections"], false);

    Ok(())
}

#[actix_rt::test]
async fn whoami_multiple_subscriptions_test() -> Result<(), Error> {
    let pool = reset()?;

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/whoami"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("CloudFront-Viewer-Country-Name", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country"], "Iceland");

    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["is_authenticated"], true);
    assert_eq!(json["email"], "test@test.com");
    assert_eq!(
        json["avatar_url"],
        "https://i1.sndcdn.com/avatars-000460644402-0iiiub-t500x500.jpg"
    );
    assert_eq!(json["is_subscriber"], true);
    assert_eq!(json["subscription_type"], "mdn_plus_5y");
    Ok(())
}
