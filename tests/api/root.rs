use crate::helpers::api_assertions::{
    assert_created_with_json_containing, assert_ok_with_json_containing,
};
use crate::helpers::db::{get_pool, reset};
use crate::helpers::http_client::TestHttpClient;
use crate::helpers::wait_for_stubr;
use crate::helpers::{app::test_app_with_login, http_client::PostPayload};
use actix_http::StatusCode;
use actix_web::test;
use anyhow::Error;
use rumba::api::root::RootSetIsAdminQuery;
use rumba::db::users::{create_or_update_user, root_set_is_admin};
use serde_json::json;

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn find_user() -> Result<(), Error> {
    reset()?;
    wait_for_stubr()?;
    let pool = get_pool();
    let mut conn = pool.get()?;

    let app = test_app_with_login().await.unwrap();
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    let root = logged_in_client
        .get("/api/v1/root/?email=test@test.com", None)
        .await;
    assert_eq!(root.response().status(), StatusCode::FORBIDDEN);

    root_set_is_admin(
        &mut conn,
        RootSetIsAdminQuery {
            fxa_uid: "TEST_SUB".into(),
            is_admin: true,
        },
    )?;
    let root = logged_in_client
        .get("/api/v1/root/?email=test@test.com", None)
        .await;

    assert_ok_with_json_containing(
        root,
        json!({
            "email": "test@test.com",
            "is_admin": true
        }),
    )
    .await;

    create_or_update_user(
        &mut conn,
        rumba::fxa::FxAUser {
            email: "test2@test.com".into(),
            locale: None,
            display_name: None,
            avatar: None,
            avatar_default: true,
            amr_values: vec![],
            uid: "TEST_SUB2".into(),
            subscriptions: vec![],
        },
        "refresh",
    )?;

    let root = logged_in_client
        .get("/api/v1/root/?email=test2@test.com", None)
        .await;

    assert_ok_with_json_containing(
        root,
        json!({
            "email": "test2@test.com",
            "is_admin": false,
            "enforce_plus": null,
        }),
    )
    .await;

    let root = logged_in_client
        .post(
            "/api/v1/root/enforce-plus",
            None,
            Some(PostPayload::Json(json!({
                "fxa_uid": "TEST_SUB2",
                "enforce_plus": "mdn_plus_10y",
            }))),
        )
        .await;
    assert_created_with_json_containing(root, json!("updated")).await;

    let root = logged_in_client
        .get("/api/v1/root/?email=test2@test.com", None)
        .await;

    assert_ok_with_json_containing(
        root,
        json!({
            "email": "test2@test.com",
            "is_admin": false,
            "enforce_plus": "mdn_plus_10y",
        }),
    )
    .await;

    let root = logged_in_client
        .post(
            "/api/v1/root/enforce-plus",
            None,
            Some(PostPayload::Json(json!({
                "fxa_uid": "TEST_SUB2",
                "enforce_plus": "mdn_plus_10y",
            }))),
        )
        .await;
    assert_created_with_json_containing(root, json!("updated")).await;

    let root = logged_in_client
        .get("/api/v1/root/?email=test2@test.com", None)
        .await;

    assert_ok_with_json_containing(
        root,
        json!({
            "email": "test2@test.com",
            "is_admin": false,
            "enforce_plus": "mdn_plus_10y",
        }),
    )
    .await;

    let root = logged_in_client
        .post(
            "/api/v1/root/is-admin",
            None,
            Some(PostPayload::Json(json!({
                "fxa_uid": "TEST_SUB2",
                "is_admin": true,
            }))),
        )
        .await;
    assert_created_with_json_containing(root, json!("updated")).await;

    let root = logged_in_client
        .get("/api/v1/root/?email=test2@test.com", None)
        .await;

    assert_ok_with_json_containing(
        root,
        json!({
            "email": "test2@test.com",
            "is_admin": true,
            "enforce_plus": "mdn_plus_10y",
        }),
    )
    .await;

    let root = logged_in_client
        .post(
            "/api/v1/root/enforce-plus",
            None,
            Some(PostPayload::Json(json!({
                "fxa_uid": "TEST_SUB2",
                "enforce_plus": null,
            }))),
        )
        .await;
    assert_created_with_json_containing(root, json!("updated")).await;

    let root = logged_in_client
        .get("/api/v1/root/?email=test2@test.com", None)
        .await;

    assert_ok_with_json_containing(
        root,
        json!({
            "email": "test2@test.com",
            "is_admin": true,
            "enforce_plus": null,
        }),
    )
    .await;

    let root = logged_in_client
        .post(
            "/api/v1/root/is-admin",
            None,
            Some(PostPayload::Json(json!({
                "fxa_uid": "TEST_SUB2",
                "is_admin": false,
            }))),
        )
        .await;
    assert_created_with_json_containing(root, json!("updated")).await;

    let root = logged_in_client
        .get("/api/v1/root/?email=test2@test.com", None)
        .await;

    assert_ok_with_json_containing(
        root,
        json!({
            "email": "test2@test.com",
            "is_admin": false,
            "enforce_plus": null,
        }),
    )
    .await;
    Ok(())
}
