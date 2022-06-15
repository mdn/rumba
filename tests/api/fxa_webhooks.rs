use std::thread;
use std::time::Duration;

use crate::helpers::app::test_app_with_login;
use crate::helpers::db::get_pool;
use crate::helpers::db::reset;
use crate::helpers::http_client::TestHttpClient;
use crate::helpers::read_json;
use actix_http::StatusCode;
use actix_web::test;
use anyhow::anyhow;
use anyhow::Error;
use diesel::prelude::*;
use rumba::db::model::WebHooksEventQuery;
use rumba::db::schema;
use rumba::db::types::FxaEvent;
use rumba::db::types::FxaEventStatus;
use stubr::{Config, Stubr};

const ONE_MS: std::time::Duration = Duration::from_millis(1);

fn assert_last_fxa_webhook_with_retry(
    fxa_uid: &str,
    typ: FxaEvent,
    status: FxaEventStatus,
) -> Result<(), Error> {
    let pool = get_pool();
    let mut conn = pool.get()?;

    let mut tries = 10;
    while tries > 0 {
        thread::sleep(ONE_MS);
        if let Some(row) = schema::webhook_events::table
            .first::<WebHooksEventQuery>(&mut conn)
            .optional()?
        {
            if fxa_uid == row.fxa_uid && typ == row.typ && status == row.status {
                return Ok(());
            }
        }
        tries -= 1;
        thread::sleep(ONE_MS);
    }
    Err(anyhow!("Timed out check fxa webhook row"))
}

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

    let res = logged_in_client.trigger_webhook(set_token).await;
    assert!(res.response().status().is_success());

    let mut tries = 10;
    while tries > 0 {
        thread::sleep(ONE_MS);

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

    assert_last_fxa_webhook_with_retry(
        "TEST_SUB",
        FxaEvent::SubscriptionStateChange,
        FxaEventStatus::Processed,
    )?;

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

    let res = logged_in_client.trigger_webhook(set_token).await;
    assert!(res.response().status().is_success());

    let mut tries = 10;
    while tries > 0 {
        thread::sleep(ONE_MS);

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

    assert_last_fxa_webhook_with_retry(
        "TEST_SUB",
        FxaEvent::SubscriptionStateChange,
        FxaEventStatus::Processed,
    )?;

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

    let res = logged_in_client.trigger_webhook(set_token).await;
    assert!(res.response().status().is_success());

    let mut tries = 10;
    while tries > 0 {
        thread::sleep(ONE_MS);

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

    assert_last_fxa_webhook_with_retry(
        "TEST_SUB",
        FxaEvent::DeleteUser,
        FxaEventStatus::Processed,
    )?;

    Err(anyhow!("Changes not applied after 10ms"))
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn invalid_set_test() -> Result<(), Error> {
    let set_token = include_str!("../data/set_tokens/set_token_delete_user_invalid.txt");
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

    let res = logged_in_client.trigger_webhook(set_token).await;

    assert_eq!(res.response().status(), StatusCode::BAD_REQUEST);

    Ok(())
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

    let res = logged_in_client.trigger_webhook(set_token).await;
    assert!(res.response().status().is_success());

    let mut tries = 10;
    while tries > 0 {
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

    assert_last_fxa_webhook_with_retry(
        "TEST_SUB",
        FxaEvent::ProfileChange,
        FxaEventStatus::Processed,
    )?;

    let res = logged_in_client.trigger_webhook(set_token).await;
    assert!(res.response().status().is_success());

    // The second event must be ignored.
    assert_last_fxa_webhook_with_retry(
        "TEST_SUB",
        FxaEvent::ProfileChange,
        FxaEventStatus::Ignored,
    )?;

    Err(anyhow!("Changes not applied after 10ms"))
}
