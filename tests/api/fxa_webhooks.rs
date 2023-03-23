use std::thread;
use std::time::Duration;

use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::PostPayload;
use crate::helpers::http_client::TestHttpClient;
use crate::helpers::read_json;
use crate::helpers::wait_for_stubr;
use actix_http::StatusCode;
use actix_web::test;
use anyhow::anyhow;
use anyhow::Error;
use diesel::prelude::*;
use rumba::db::model::WebHookEventQuery;
use rumba::db::schema;
use rumba::db::types::FxaEvent;
use rumba::db::types::FxaEventStatus;
use rumba::db::Pool;
use serde_json::json;
use stubr::{Config, Stubr};

const TEN_MS: std::time::Duration = Duration::from_millis(10);

fn assert_last_fxa_webhook(
    pool: &Pool,
    fxa_uid: &str,
    typ: FxaEvent,
    status: FxaEventStatus,
) -> Result<(), Error> {
    let mut conn = pool.get()?;

    let rows = schema::webhook_events::table.get_results::<WebHookEventQuery>(&mut conn)?;
    if let Some(row) = rows.last() {
        if fxa_uid == row.fxa_uid && typ == row.typ && status == row.status {
            return Ok(());
        }
    }

    Err(anyhow!(
        "no row matching: {}, {:?}, {:?}",
        fxa_uid,
        typ,
        status
    ))
}

fn assert_last_fxa_webhook_with_retry(
    pool: &Pool,
    fxa_uid: &str,
    typ: FxaEvent,
    status: FxaEventStatus,
) -> Result<(), Error> {
    let mut tries = 10;
    while tries > 0 {
        if assert_last_fxa_webhook(pool, fxa_uid, typ, status).is_ok() {
            return Ok(());
        }
        tries -= 1;
        thread::sleep(TEN_MS);
    }
    Err(anyhow!("Timed out check fxa webhook row"))
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn subscription_state_change_to_10m_test() -> Result<(), Error> {
    let set_token =
        include_str!("../data/set_tokens/set_token_subscription_state_change_to_10m.txt");
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("X-Appengine-Country", "Iceland")]),
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

    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("X-Appengine-Country", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["subscription_type"], "mdn_plus_10m");

    assert_last_fxa_webhook(
        &pool,
        "TEST_SUB",
        FxaEvent::SubscriptionStateChange,
        FxaEventStatus::Processed,
    )?;

    let res = logged_in_client.trigger_webhook(set_token).await;
    assert!(res.response().status().is_success());

    // The second event must be ignored.
    assert_last_fxa_webhook(
        &pool,
        "TEST_SUB",
        FxaEvent::SubscriptionStateChange,
        FxaEventStatus::Ignored,
    )?;

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn subscription_state_change_to_core_test_empty_subscription() -> Result<(), Error> {
    let set_token =
        include_str!("../data/set_tokens/set_token_subscription_state_change_to_core.txt");
    subscription_state_change_to_core_test(set_token).await
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn subscription_state_change_to_core_test_inactive() -> Result<(), Error> {
    let set_token =
        include_str!("../data/set_tokens/set_token_subscription_state_change_to_core_inactive.txt");
    subscription_state_change_to_core_test(set_token).await
}

async fn subscription_state_change_to_core_test(set_token: &str) -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("X-Appengine-Country", "Iceland")]),
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

    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("X-Appengine-Country", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["subscription_type"], "core");
    assert_eq!(json["is_subscriber"], false);

    assert_last_fxa_webhook(
        &pool,
        "TEST_SUB",
        FxaEvent::SubscriptionStateChange,
        FxaEventStatus::Processed,
    )?;

    Ok(())
}

#[actix_rt::test]
async fn delete_user_test() -> Result<(), Error> {
    let set_token = include_str!("../data/set_tokens/set_token_delete_user.txt");
    let pool = reset()?;
    let stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/collections"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: true,
            verify: false,
        },
    );
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("X-Appengine-Country", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country"], "Iceland");
    assert_eq!(json["is_authenticated"], true);

    /*
    // Let's check the cascade delete. This will create a multiple collection item that is tied to the user.
    // When the set token is sent to delete the user it should cascade and thus not violate the fk ref:
        multiple_collections (
            ...
            user_id    BIGSERIAL references users (id) ON DELETE CASCADE,
            ...
        )
    */
    let payload = serde_json::json!({
        "title" : "Interesting CSS",
        "url": "/en-US/docs/Web/CSS"
    });

    let base_url = "/api/v2/collections/";

    let default_collection = read_json(logged_in_client.get(base_url, None).await).await;
    let default_collection_id = default_collection.as_array().unwrap()[0]["id"]
        .as_str()
        .unwrap();

    let create_res = logged_in_client
        .post(
            format!("{:1}{:2}/items/", base_url, default_collection_id).as_str(),
            None,
            Some(PostPayload::Json(payload)),
        )
        .await;
    assert_eq!(create_res.status(), 201);

    let res = logged_in_client
        .post("/api/v1/ping", None, Some(PostPayload::Form(json!({}))))
        .await;
    assert_eq!(res.response().status(), 201);

    let res = logged_in_client.trigger_webhook(set_token).await;
    assert!(res.response().status().is_success());

    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("X-Appengine-Country", "Iceland")]),
        )
        .await;
    assert!(!whoami.response().status().is_success());

    assert_last_fxa_webhook(
        &pool,
        "TEST_SUB",
        FxaEvent::DeleteUser,
        FxaEventStatus::Processed,
    )?;

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn invalid_set_test() -> Result<(), Error> {
    let set_token = include_str!("../data/set_tokens/set_token_delete_user_invalid.txt");
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("X-Appengine-Country", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country"], "Iceland");
    assert_eq!(json["is_authenticated"], true);

    let res = logged_in_client.trigger_webhook(set_token).await;

    assert_eq!(res.response().status(), StatusCode::OK);

    let mut conn = pool.get()?;
    let failed_token = schema::raw_webhook_events_tokens::table
        .select(schema::raw_webhook_events_tokens::token)
        .first::<String>(&mut conn)?;
    assert_eq!(failed_token, set_token);
    drop(stubr);
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
            verbose: true,
            verify: false,
        },
    );
    wait_for_stubr().await?;

    let set_token = include_str!("../data/set_tokens/set_token_profile_change.txt");
    let pool = reset()?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get(
            "/api/v1/whoami",
            Some(vec![("X-Appengine-Country", "Iceland")]),
        )
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["email"], "test@test.com");

    drop(stubr);

    let stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/fxa_webhooks"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: true,
            verify: false,
        },
    );
    wait_for_stubr().await?;

    thread::sleep(TEN_MS);

    let res = logged_in_client.trigger_webhook(set_token).await;
    assert!(res.response().status().is_success());

    let mut tries = 100;
    while tries > 0 {
        let whoami = logged_in_client
            .get(
                "/api/v1/whoami",
                Some(vec![("X-Appengine-Country", "Iceland")]),
            )
            .await;
        assert!(whoami.response().status().is_success());
        let json = read_json(whoami).await;
        assert_eq!(json["username"], "TEST_SUB");
        if json["email"] == "foo@bar.com" {
            break;
        }
        thread::sleep(TEN_MS);
        tries -= 1;
    }

    if tries == 0 {
        return Err(anyhow!("Changes not applied after 1s"));
    }

    assert_last_fxa_webhook_with_retry(
        &pool,
        "TEST_SUB",
        FxaEvent::ProfileChange,
        FxaEventStatus::Processed,
    )?;

    drop(stubr);
    Ok(())
}
