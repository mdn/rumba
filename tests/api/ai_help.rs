use std::time::Duration;

use crate::helpers::api_assertions::assert_ok_with_json_containing;
use crate::helpers::app::init_test;
use actix_http::StatusCode;
use actix_rt::time::sleep;
use anyhow::Error;
use rumba::settings::SETTINGS;
use serde_json::json;

#[actix_rt::test]
async fn test_quota() -> Result<(), Error> {
    let (mut client, stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/core_user"]).await?;

    let quota = client.get("/api/v1/plus/ai/ask/quota", None).await;
    assert_ok_with_json_containing(quota, json!({"quota": { "count": 0, "limit": 5}})).await;

    let ask = client
        .post(
            "/api/v1/plus/ai/ask",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "messages": [{ "role": "user", "content": "Foo?" }]
            }))),
        )
        .await;
    assert_eq!(ask.status(), StatusCode::NOT_IMPLEMENTED);
    let quota = client.get("/api/v1/plus/ai/ask/quota", None).await;
    assert_ok_with_json_containing(quota, json!({"quota": { "count": 1, "limit": 5}})).await;

    for _ in 0..4 {
        let ask = client
            .post(
                "/api/v1/plus/ai/ask",
                None,
                Some(crate::helpers::http_client::PostPayload::Json(json!({
                    "messages": [{ "role": "user", "content": "Foo?" }]
                }))),
            )
            .await;
        assert_eq!(ask.status(), StatusCode::NOT_IMPLEMENTED);
    }

    let quota = client.get("/api/v1/plus/ai/ask/quota", None).await;
    assert_ok_with_json_containing(
        quota,
        json!({"quota": { "count": 5, "limit": 5, "remaining": 0}}),
    )
    .await;

    let ask = client
        .post(
            "/api/v1/plus/ai/ask",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "messages": [{ "role": "user", "content": "Foo?" }]
            }))),
        )
        .await;
    assert_ok_with_json_containing(ask, json!(null)).await;
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_quota_rest() -> Result<(), Error> {
    let (mut client, stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/core_user"]).await?;

    let quota = client.get("/api/v1/plus/ai/ask/quota", None).await;
    assert_ok_with_json_containing(quota, json!({"quota": { "count": 0, "limit": 5}})).await;

    let ask = client
        .post(
            "/api/v1/plus/ai/ask",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "messages": [{ "role": "user", "content": "Foo?" }]
            }))),
        )
        .await;
    assert_eq!(ask.status(), StatusCode::NOT_IMPLEMENTED);
    let quota = client.get("/api/v1/plus/ai/ask/quota", None).await;
    assert_ok_with_json_containing(quota, json!({"quota": { "count": 1, "limit": 5}})).await;

    for _ in 0..4 {
        let ask = client
            .post(
                "/api/v1/plus/ai/ask",
                None,
                Some(crate::helpers::http_client::PostPayload::Json(json!({
                    "messages": [{ "role": "user", "content": "Foo?" }]
                }))),
            )
            .await;
        assert_eq!(ask.status(), StatusCode::NOT_IMPLEMENTED);
    }

    let quota = client.get("/api/v1/plus/ai/ask/quota", None).await;
    assert_ok_with_json_containing(
        quota,
        json!({"quota": { "count": 5, "limit": 5, "remaining": 0}}),
    )
    .await;

    let ask = client
        .post(
            "/api/v1/plus/ai/ask",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "messages": [{ "role": "user", "content": "Foo?" }]
            }))),
        )
        .await;
    assert_ok_with_json_containing(ask, json!(null)).await;

    sleep(Duration::from_secs(
        SETTINGS
            .ai
            .as_ref()
            .map(|ai| ai.limit_reset_duration)
            .unwrap()
            .try_into()
            .unwrap(),
    ))
    .await;

    let quota = client.get("/api/v1/plus/ai/ask/quota", None).await;
    assert_ok_with_json_containing(
        quota,
        json!({"quota": { "count": 0, "limit": 5, "remaining": 5}}),
    )
    .await;
    drop(stubr);
    Ok(())
}