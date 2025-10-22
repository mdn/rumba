use crate::helpers::app::drop_stubr;
use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::PostPayload;
use crate::helpers::http_client::TestHttpClient;
use crate::helpers::read_json;
use crate::helpers::set_tokens::invalid_token_from_json_file;
use crate::helpers::set_tokens::token_from_claim;
use crate::helpers::set_tokens::token_from_file;
use actix_http::StatusCode;
use actix_rt::time::sleep;
use actix_web::test;
use anyhow::anyhow;
use anyhow::Error;
use chrono::DateTime;
use diesel::prelude::*;
use rumba::db::model::SubscriptionChangeQuery;
use rumba::db::model::WebHookEventQuery;
use rumba::db::schema;
use rumba::db::types::FxaEvent;
use rumba::db::types::FxaEventStatus;
use rumba::db::types::Subscription;
use rumba::db::Pool;
use serde_json::json;
use serde_json::Value;
use std::thread;
use std::time::Duration;
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
        token_from_file("tests/data/set_tokens/set_token_subscription_state_change_to_10m.json");
    let pool = reset()?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(
        json["subscription_type"], "mdn_plus_5m",
        "Subscription type wrong"
    );

    let res = logged_in_client.trigger_webhook(&set_token).await;
    assert!(res.response().status().is_success());

    let whoami = logged_in_client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
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

    let res = logged_in_client.trigger_webhook(&set_token).await;
    assert!(res.response().status().is_success());

    // The second event must be ignored.
    assert_last_fxa_webhook(
        &pool,
        "TEST_SUB",
        FxaEvent::SubscriptionStateChange,
        FxaEventStatus::Ignored,
    )?;
    drop_stubr(stubr).await;
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn subscription_state_change_to_core_test_empty_subscription() -> Result<(), Error> {
    let set_token =
        token_from_file("tests/data/set_tokens/set_token_subscription_state_change_to_core.json");
    subscription_state_change_to_core_test(&set_token).await
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn subscription_state_change_to_core_test_inactive() -> Result<(), Error> {
    let set_token = token_from_file(
        "tests/data/set_tokens/set_token_subscription_state_change_to_core_inactive.json",
    );
    subscription_state_change_to_core_test(&set_token).await
}

async fn subscription_state_change_to_core_test(set_token: &str) -> Result<(), Error> {
    let pool = reset()?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
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
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
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
    let pool = reset()?;
    let set_token = token_from_file("tests/data/set_tokens/set_token_delete_user.json");

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country"], "Iceland");
    assert_eq!(json["geo"]["country_iso"], "IS");
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

    let res = logged_in_client.trigger_webhook(&set_token).await;
    assert!(res.response().status().is_success());

    let whoami = logged_in_client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(!whoami.response().status().is_success());

    assert_last_fxa_webhook(
        &pool,
        "TEST_SUB",
        FxaEvent::DeleteUser,
        FxaEventStatus::Processed,
    )?;
    drop_stubr(stubr).await;
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn invalid_set_test() -> Result<(), Error> {
    let set_token =
        invalid_token_from_json_file("tests/data/set_tokens/set_token_delete_user.json");
    let pool = reset()?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country"], "Iceland");
    assert_eq!(json["geo"]["country_iso"], "IS");
    assert_eq!(json["is_authenticated"], true);

    let res = logged_in_client.trigger_webhook(&set_token).await;

    assert_eq!(res.response().status(), StatusCode::OK);

    let mut conn = pool.get()?;
    let failed_token = schema::raw_webhook_events_tokens::table
        .select(schema::raw_webhook_events_tokens::token)
        .first::<String>(&mut conn)?;
    assert_eq!(failed_token, set_token);
    drop_stubr(stubr).await;
    Ok(())
}

#[actix_rt::test]
async fn whoami_test() -> Result<(), Error> {
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

    let pool = reset()?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["email"], "test@test.com");
    drop_stubr(stubr).await;

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
    let set_token = token_from_file("tests/data/set_tokens/set_token_profile_change.json");
    let pool = reset()?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    let res = logged_in_client.trigger_webhook(&set_token).await;
    assert!(res.response().status().is_success());

    let mut tries = 100;
    while tries > 0 {
        let whoami = logged_in_client
            .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
            .await;
        assert!(whoami.response().status().is_success());
        let json = read_json(whoami).await;
        assert_eq!(json["username"], "TEST_SUB");
        if json["email"] == "foo@bar.com" {
            break;
        }
        sleep(TEN_MS).await;
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
    drop_stubr(stubr).await;
    Ok(())
}

#[actix_rt::test]
async fn record_subscription_state_transitions_test() -> Result<(), Error> {
    let stubr = Stubr::start_blocking_with(
        vec!["tests/stubs", "tests/test_specific_stubs/core_user"],
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: true,
            verify: false,
        },
    );
    let pool = reset()?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    let whoami = logged_in_client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["subscription_type"], "core", "Subscription type wrong");

    // verify there are no state transitions in the table
    let mut conn = pool.get()?;
    let count = schema::user_subscription_transitions::table
        .count()
        .first::<i64>(&mut conn)?;
    assert_eq!(count, 0);

    // Create a transition to 5m and check if it is recorded.
    {
        let set_token =
            token_from_file("tests/data/set_tokens/set_token_subscription_state_change_to_5m.json");
        let res = logged_in_client.trigger_webhook(&set_token).await;
        assert!(res.response().status().is_success());

        // check the transition is recorded
        let transitions = schema::user_subscription_transitions::table
            .load::<SubscriptionChangeQuery>(&mut conn)?;
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].old_subscription_type, Subscription::Core);
        assert_eq!(
            transitions[0].new_subscription_type,
            Subscription::MdnPlus_5m
        );
        assert_eq!(transitions[0].user_id, 1);
        assert_eq!(
            transitions[0].created_at,
            DateTime::from_timestamp(1654425317, 0).unwrap().naive_utc()
        );
    }

    // Now create a later transition to 10m and check the table again.
    {
        let json_str = std::fs::read_to_string(
            "tests/data/set_tokens/set_token_subscription_state_change_to_10m.json",
        )
        .unwrap();
        let mut claim: Value = serde_json::from_str(&json_str).unwrap();
        // Add some time to the event to be sure it is after the previous event.
        claim["iat"] = json!(1654425317000i64 + 300000);
        claim["events"]["https://schemas.accounts.firefox.com/event/subscription-state-change"]
            ["changeTime"] = json!(1654425317000i64 + 300000);
        // We also add some unknown capability to the event to check that they are ignored correctly.
        claim["events"]["https://schemas.accounts.firefox.com/event/subscription-state-change"]
            ["capabilities"] = json!(["something_unknown", "mdn_plus_10m"]);
        let set_token = token_from_claim(&claim);

        let res = logged_in_client.trigger_webhook(&set_token).await;
        assert!(res.response().status().is_success());

        // check the transition is recorded
        let transitions = schema::user_subscription_transitions::table
            .order(schema::user_subscription_transitions::created_at)
            .load::<SubscriptionChangeQuery>(&mut conn)?;
        assert_eq!(transitions.len(), 2);
        assert_eq!(transitions[0].old_subscription_type, Subscription::Core);
        assert_eq!(
            transitions[0].new_subscription_type,
            Subscription::MdnPlus_5m
        );
        assert_eq!(
            transitions[1].old_subscription_type,
            Subscription::MdnPlus_5m
        );
        assert_eq!(
            transitions[1].new_subscription_type,
            Subscription::MdnPlus_10m
        );
        assert_eq!(transitions[0].user_id, 1);
        assert_eq!(transitions[1].user_id, 1);
        assert_eq!(
            transitions[0].created_at,
            DateTime::from_timestamp(1654425317, 0).unwrap().naive_utc()
        );
        assert_eq!(
            transitions[1].created_at,
            DateTime::from_timestamp(1654425617, 0).unwrap().naive_utc()
        );
    }

    // Now, reate an event where the new subscription is matching the old one.
    // We do not record those.
    {
        let json_str = std::fs::read_to_string(
            "tests/data/set_tokens/set_token_subscription_state_change_to_10m.json",
        )
        .unwrap();
        let mut claim: Value = serde_json::from_str(&json_str).unwrap();
        // Add some time to the event to be sure it is after the previous event.
        claim["iat"] = json!(1654425317000i64 + 600000);
        claim["events"]["https://schemas.accounts.firefox.com/event/subscription-state-change"]
            ["changeTime"] = json!(1654425317000i64 + 600000);
        // We also add some unknown capability to the event to check that they are ignored correctly.
        claim["events"]["https://schemas.accounts.firefox.com/event/subscription-state-change"]
            ["capabilities"] = json!(["mdn_plus_10m"]);
        let set_token = token_from_claim(&claim);

        let res = logged_in_client.trigger_webhook(&set_token).await;
        assert!(res.response().status().is_success());

        // check the transition is not recorded
        let transitions = schema::user_subscription_transitions::table
            .order(schema::user_subscription_transitions::created_at)
            .load::<SubscriptionChangeQuery>(&mut conn)?;
        assert_eq!(transitions.len(), 2);
        assert_eq!(
            transitions[0].created_at,
            DateTime::from_timestamp(1654425317, 0).unwrap().naive_utc()
        );
        assert_eq!(
            transitions[1].created_at,
            DateTime::from_timestamp(1654425317, 0).unwrap().naive_utc()
        );
    }

    drop_stubr(stubr).await;
    Ok(())
}
