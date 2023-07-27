use crate::helpers::app::init_test;
use crate::helpers::read_json;
use anyhow::Error;
use serde_json::{json, Value};

#[actix_rt::test]
async fn test_experiments_config() -> Result<(), Error> {
    let (mut client, stubr) = init_test(vec!["tests/stubs"]).await?;
    let whoami = client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country"], "Iceland");
    assert_eq!(json["geo"]["country_iso"], "IS");

    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["is_authenticated"], true);
    assert_eq!(json["is_subscriber"], true);

    let experiments = client
        .post(
            "/api/v1/plus/settings/experiments/",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "config": { "gpt4": true }
            }))),
        )
        .await;
    assert_eq!(experiments.status(), 201);
    let json = read_json(experiments).await;
    assert_eq!(json["gpt4"], json!(null));
    let active_experiments = client.get("/api/v1/plus/settings/experiments/", None).await;
    assert!(active_experiments.response().status().is_success());
    let json = read_json(active_experiments).await;
    assert_eq!(json, Value::Null);

    let experiments = client
        .post(
            "/api/v1/plus/settings/experiments/",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "active": true,
                "config": { "gpt4": true }
            }))),
        )
        .await;
    assert_eq!(experiments.status(), 201);
    let json = read_json(experiments).await;
    assert_eq!(json["config"]["gpt4"], true);
    let active_experiments = client.get("/api/v1/plus/settings/experiments/", None).await;
    assert!(active_experiments.response().status().is_success());
    let json = read_json(active_experiments).await;
    assert_eq!(json["active"], true);
    assert_eq!(json["config"]["gpt4"], true);

    let experiments = client
        .post(
            "/api/v1/plus/settings/experiments/",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "config": { "full_doc": true }
            }))),
        )
        .await;
    assert_eq!(experiments.status(), 201);
    let json = read_json(experiments).await;
    assert_eq!(json["config"]["full_doc"], true);
    assert_eq!(json["config"]["gpt4"], true);
    let active_experiments = client.get("/api/v1/plus/settings/experiments/", None).await;
    assert!(active_experiments.response().status().is_success());
    let json = read_json(active_experiments).await;
    assert_eq!(json["config"]["full_doc"], true);
    assert_eq!(json["config"]["gpt4"], true);

    let experiments = client
        .post(
            "/api/v1/plus/settings/experiments/",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "config": { "gpt4": false }
            }))),
        )
        .await;
    assert_eq!(experiments.status(), 201);
    let json = read_json(experiments).await;
    assert_eq!(json["config"]["full_doc"], true);
    assert_eq!(json["config"]["gpt4"], false);
    let active_experiments = client.get("/api/v1/plus/settings/experiments/", None).await;
    assert!(active_experiments.response().status().is_success());
    let json = read_json(active_experiments).await;
    assert_eq!(json["config"]["full_doc"], true);
    assert_eq!(json["config"]["gpt4"], false);

    let experiments = client
        .post(
            "/api/v1/plus/settings/experiments/",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "active": false,
            }))),
        )
        .await;
    assert_eq!(experiments.status(), 201);
    let json = read_json(experiments).await;
    assert_eq!(json["active"], false);
    assert_eq!(json["config"]["full_doc"], true);
    assert_eq!(json["config"]["gpt4"], false);
    let active_experiments = client.get("/api/v1/plus/settings/experiments/", None).await;
    assert!(active_experiments.response().status().is_success());
    let json = read_json(active_experiments).await;
    assert_eq!(json, Value::Null);
    drop(stubr);
    Ok(())
}
