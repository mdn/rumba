use crate::helpers::api_assertions::{
    assert_created_with_json_containing, assert_ok_with_json_containing,
};
use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::{PostPayload, TestHttpClient};
use crate::helpers::read_json;
use actix_http::body::{BoxBody, EitherBody};
use actix_http::{Request, StatusCode};
use actix_web::dev::{Service, ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::test;
use anyhow::Error;
use assert_json_diff::assert_json_include;
use serde_json::{json, Value};

use std::thread;
use std::time::Duration;
use stubr::{Config, Stubr};

// /en-US/docs/Web/CSS -> URL

#[actix_rt::test]
async fn test_create_and_get_collection() -> Result<(), Error> {
    let (mut client, _) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "Test",
                "description": "Test description"
            }))),
        )
        .await;

    let body = assert_created_with_json_containing(
        res,
        json!(
            {
                "name": "Test",
                "description": "Test description",
                "article_count" : 0
            }
        ),
    )
    .await
    .unwrap();

    let get_res = client
        .get(
            format!("{}{}/", base_url, body["id"].as_str().unwrap()).as_str(),
            None,
        )
        .await;

    assert_ok_with_json_containing(
        get_res,
        json!(
            {
               "id": body["id"].as_str(),
               "name": "Test",
               "description": "Test description",
               "article_count" : 0,
               "items": []
            }
        ),
    )
    .await
    .unwrap();
    Ok(())
}

#[actix_rt::test]
async fn test_add_items_to_collection() -> Result<(), Error> {
    let (mut client, _stubr) =
    init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
let base_url = "/api/v2/collections/";

let res = client
    .post(
        base_url,
        None,
        Some(PostPayload::Json(json!({
            "name": "Test",
            "description": "Test description"
        }))),
    )
    .await;

let body = assert_created_with_json_containing(
    res,json!(
        {
            "name": "Test",
            "description": "Test description",
            "article_count" : 0
        }
    ),
)
.await
.unwrap();
_stubr.uri();
for i in 1..12 {
let mut create_res = client
    .post(
        format!("{}{}/items/", base_url, body["id"].as_str().unwrap()).as_str(),
        None,
        Some(PostPayload::Json(json!({
            "name" : format!("Interesting CSS{}",i),
            "url": format!("/en-US/docs/Web/CSS{}",i)
        }
        )))
    )
    .await;
 assert_eq!(create_res.status(), StatusCode::CREATED);   
}
 
Ok(())
}

#[actix_rt::test]
async fn test_edit_item_in_collection() -> Result<(), Error> {
    reset()?;
    Ok(())
}

#[actix_rt::test]
async fn test_get_collection_detail() -> Result<(), Error> {
    reset()?;
    Ok(())
}

async fn init_test(
    custom_stubs: Vec<&str>,
) -> Result<
    (
        TestHttpClient<
            impl Service<
                Request,
                Response = ServiceResponse<EitherBody<BoxBody>>,
                Error = actix_web::Error,
            >,
        >,
        Stubr,
    ),
    anyhow::Error,
> {
    reset()?;
    let _stubr = Stubr::start_blocking_with(
        custom_stubs,
        Config {
            port: Some(4321),
            latency: None,
            global_delay: None,
            verbose: Some(true),
        },
    );
    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    Ok((logged_in_client, _stubr))
}
