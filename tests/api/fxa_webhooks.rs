use std::thread;
use std::time::Duration;

use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::TestHttpClient;
use crate::helpers::read_json;
use actix_web::test;
use anyhow::anyhow;
use anyhow::Error;
use stubr::{Config, Stubr};

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn subscription_state_change_to_10m_test() -> Result<(), Error> {
    let set_token =
        include_str!("../data/set_tokens/set_token_subscription_state_change_to_10m.txt");
    reset()?;
    let app = test_app_with_login().await?;
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
    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(
        json["subscription_type"], "mdn_plus_5m",
        "Subscription type wrong"
    );

    logged_in_client.trigger_webhook(set_token).await;

    let mut tries = 10;
    while tries > 0 {
        let one_ms = Duration::from_millis(1);
        thread::sleep(one_ms);

        let whoami = logged_in_client
            .get(
                "/api/v1/whoami",
                Some(vec![("CloudFront-Viewer-Country-Name", "Iceland")]),
            )
            .await;
        assert!(whoami.response().status().is_success());
        let json = read_json(whoami).await;
        assert_eq!(json["username"], "TEST_SUB");
        if json["subscription_type"] == "mdn_plus_10m" {
            return Ok(());
        }
        tries -= 1;
    }
    Err(anyhow!("Changes not applied after 10ms"))
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn subscription_state_change_to_core_test() -> Result<(), Error> {
    let set_token =
        include_str!("../data/set_tokens/set_token_subscription_state_change_to_core.txt");
    reset()?;
    let app = test_app_with_login().await?;
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
    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(
        json["subscription_type"], "mdn_plus_5m",
        "Subscription type wrong"
    );

    logged_in_client.trigger_webhook(set_token).await;

    let mut tries = 10;
    while tries > 0 {
        let one_ms = Duration::from_millis(1);
        thread::sleep(one_ms);

        let whoami = logged_in_client
            .get(
                "/api/v1/whoami",
                Some(vec![("CloudFront-Viewer-Country-Name", "Iceland")]),
            )
            .await;
        assert!(whoami.response().status().is_success());
        let json = read_json(whoami).await;
        assert_eq!(json["username"], "TEST_SUB");
        if json["subscription_type"] == "core" {
            assert_eq!(json["is_subscriber"], false);
            return Ok(());
        }
        tries -= 1;
    }
    Err(anyhow!("Changes not applied after 10ms"))
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn delete_user_test() -> Result<(), Error> {
    let set_token = include_str!("../data/set_tokens/set_token_delete_user.txt");
    reset()?;
    let app = test_app_with_login().await?;
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
    assert_eq!(json["is_authenticated"], true);

    logged_in_client.trigger_webhook(set_token).await;

    let mut tries = 10;
    while tries > 0 {
        let one_ms = Duration::from_millis(1);
        thread::sleep(one_ms);

        let whoami = logged_in_client
            .get(
                "/api/v1/whoami",
                Some(vec![("CloudFront-Viewer-Country-Name", "Iceland")]),
            )
            .await;
        if !whoami.response().status().is_success() {
            return Ok(());
        }
        tries -= 1;
    }
    Err(anyhow!("Changes not applied after 10ms"))
}

#[actix_rt::test]
async fn change_profile_test() -> Result<(), Error> {
    let stubr = Stubr::start_blocking_with(
        vec!["tests/stubs"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );

    let set_token = include_str!("../data/set_tokens/set_token_profile_change.txt");
    reset()?;
    let app = test_app_with_login().await?;
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
    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["email"], "test@test.com");

    drop(stubr);

    let _stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/fxa_webhooks"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );

    logged_in_client.trigger_webhook(set_token).await;

    let mut tries = 10;
    while tries > 0 {
        let one_ms = Duration::from_millis(1);
        thread::sleep(one_ms);

        let whoami = logged_in_client
            .get(
                "/api/v1/whoami",
                Some(vec![("CloudFront-Viewer-Country-Name", "Iceland")]),
            )
            .await;
        assert!(whoami.response().status().is_success());
        let json = read_json(whoami).await;
        assert_eq!(json["username"], "TEST_SUB");
        if json["email"] == "foo@bar.com" {
            return Ok(());
        }
        tries -= 1;
    }
    Err(anyhow!("Changes not applied after 10ms"))
}
