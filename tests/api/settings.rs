use crate::helpers::app::init_test;
use crate::helpers::read_json;
use anyhow::Error;
use serde_json::json;

#[actix_rt::test]
async fn test_core_settings() -> Result<(), Error> {
    let (mut client, stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/core_user"]).await?;
    let whoami = client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country_iso"], "IS");

    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["is_authenticated"], true);
    assert_eq!(json["is_subscriber"], false);

    let settings = client
        .post(
            "/api/v1/plus/settings/",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "no_ads": true
            }))),
        )
        .await;
    assert_eq!(settings.status(), 201);

    let whoami = client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["is_authenticated"], true);
    assert_eq!(json["is_subscriber"], false);
    assert_eq!(json["settings"]["no_ads"], false);
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_subscriber_settings() -> Result<(), Error> {
    let (mut client, stubr) = init_test(vec!["tests/stubs"]).await?;
    let whoami = client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["geo"]["country_iso"], "IS");

    assert_eq!(json["username"], "TEST_SUB");
    assert_eq!(json["is_authenticated"], true);
    assert_eq!(json["is_subscriber"], true);

    let settings = client
        .post(
            "/api/v1/plus/settings/",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "no_ads": true
            }))),
        )
        .await;
    assert_eq!(settings.status(), 201);

    let whoami = client
        .get("/api/v1/whoami", Some(vec![("X-Appengine-Country", "IS")]))
        .await;
    assert!(whoami.response().status().is_success());
    let json = read_json(whoami).await;
    assert_eq!(json["is_authenticated"], true);
    assert_eq!(json["is_subscriber"], true);
    assert_eq!(json["settings"]["no_ads"], true);
    drop(stubr);
    Ok(())
}
