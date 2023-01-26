use std::fs;
use std::time::Duration;

use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::{PostPayload, TestHttpClient};
use crate::helpers::{read_json, wait_for_stubr};
use actix_rt::time::{sleep, timeout};
use actix_web::dev::Service;
use actix_web::test;
use anyhow::{anyhow, Error};
use chrono::{NaiveDate, Utc};
use diesel::dsl::count_distinct;
use diesel::query_dsl::methods::SelectDsl;
use diesel::RunQueryDsl;
use rumba::db::{self, Pool};
use serde_json::{json, Value};
use stubr::{Config, Stubr};

macro_rules! test_setup {
    () => {{
        let mut pool = reset()?;
        let stubr = Stubr::start_blocking_with(
            vec!["tests/stubs", "tests/test_specific_stubs/bcd_updates"],
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
        wait_for_updates_sync(&mut pool, &mut logged_in_client).await?;
        (logged_in_client, stubr)
    }};
}

#[actix_rt::test]
async fn test_bcd_updates_basic_pagination() -> Result<(), Error> {
    let (mut logged_in_client, _stubr) = test_setup!();
    let res = logged_in_client.get("/api/v2/updates/", None).await;
    let json = read_json(res).await;
    // 3 browsers x 3 browser releases = 9 total rows, (5 per page so 2 pages total)
    assert_eq!(json["last"].as_i64().unwrap(), 2);
    let mut results = Vec::new();
    for i in 1..3 {
        let res = logged_in_client
            .get(
                format!("{0}?page={1}", "/api/v2/updates/", i).as_str(),
                None,
            )
            .await;
        let json = read_json(res).await;
        results.push(json["data"].as_array().unwrap().clone());
    }
    let flattened: Vec<&Value> = results.iter().flatten().collect();

    //Check time ordering
    assert_eq!(flattened.len(), 9);
    let mut previous = Utc::now().naive_utc().date();
    for ele in flattened.iter() {
        let current =
            NaiveDate::parse_from_str(ele["release_date"].as_str().unwrap(), "%Y-%m-%d").unwrap();
        assert!(current <= previous);
        previous = current;
    }
    //Check a specific case for added/removed
    //Firefox 107 has 15 added 2 removed
    let mut firefox_107: Value = flattened
        .into_iter()
        .filter(|val| {
            val["browser"].as_str().unwrap().eq("firefox")
                && val["version"].as_str().unwrap().eq("107")
        })
        .collect::<Vec<&Value>>()
        .first_mut()
        .unwrap()
        .clone();

    let events = firefox_107["events"].take();
    let added = &events["added"];
    let removed = &events["removed"];

    firefox_107["events"].take();

    assert_eq!(
        json!(
            {
                "type": "browser_grouping",
                "browser": "firefox",
                "version": "107",
                "name": "Firefox",
                "engine": "Gecko",
                "engine_version": "107",
                "events": null,
                "release_date": "2022-11-15",
                "release_notes": ""
              }

        ),
        firefox_107
    );
    //Check a specific case for removed
    assert_eq!(added.as_array().unwrap().len(), 15);
    assert!(added.as_array().unwrap().contains(&json!(
        {
            "path": "api.CSSFontPaletteValuesRule",
            "compat": {
              "mdn_url": null,
              "source_file": "api/CSSFontPaletteValuesRule.json",
              "spec_url": "https://w3c.github.io/csswg-drafts/css-fonts-4/#om-fontpalettevalues",
              "status": {
                "deprecated": false,
                "experimental": false,
                "standard_track": true
              },
              "engines": []
            }
          }
    )));
    assert_eq!(removed.as_array().unwrap().len(), 2);
    assert!(removed.as_array().unwrap().contains(&json!(
        {
            "path": "mathml.elements.ms.lquote_rquote_attributes",
            "compat": {
              "mdn_url": null,
              "source_file": "mathml/elements/ms.json",
              "spec_url": null,
              "status": {
                "deprecated": true,
                "experimental": false,
                "standard_track": false
              },
              "engines": []
            }
          }
    )));
    Ok(())
}

#[actix_rt::test]
async fn test_bcd_updates_filter_by_collections() -> Result<(), Error> {
    let (mut logged_in_client, _stubr) = test_setup!();
    let res = logged_in_client.get("/api/v2/collections/", None).await;
    let vals = read_json(res).await;
    let default_id = &vals.as_array().unwrap()[0]["id"];
    logged_in_client.post(format!("/api/v2/collections/{:1}/items/", default_id.as_str().unwrap()).as_str(), None, Some(PostPayload::Json(json!({"url":"/en-US/docs/Web/API/CaptureController","title":"CaptureController","notes":""})))).await;
    let res = logged_in_client
        .get(
            format!(
                "/api/v2/updates/?collections={}",
                default_id.as_str().unwrap()
            )
            .as_str(),
            None,
        )
        .await;
    let res_json = read_json(res).await;
    let file = fs::File::open("tests/data/updates_response_collections.json")
        .expect("Json snapshot opened");
    let snapshot_json: Value = serde_json::from_reader(file).expect("Error reading snapshot json");
    assert_eq!(res_json, snapshot_json);
    Ok(())
}

async fn wait_for_updates_sync(
    pool: &mut Pool,
    logged_in_client: &mut TestHttpClient<
        impl Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<
                actix_http::body::EitherBody<actix_http::body::BoxBody>,
            >,
            Error = actix_web::Error,
        >,
    >,
) -> Result<(), Error> {
    sync(logged_in_client).await;
    timeout(Duration::from_millis(10_000), async {
        let mut val = 0;

        while val <= 0 {
            sleep(Duration::from_millis(100)).await;
            val = db::schema_manual::bcd_updates_view::table
                .select(count_distinct(db::schema_manual::bcd_updates_view::browser))
                .first(&mut pool.get()?)?;
        }
        Ok::<(), Error>(())
    })
    .await
    .map_err(|_| anyhow!("Updates not ready after 10 seconds"))??;
    Ok(())
}

async fn sync(
    logged_in_client: &mut TestHttpClient<
        impl Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<
                actix_http::body::EitherBody<actix_http::body::BoxBody>,
            >,
            Error = actix_web::Error,
        >,
    >,
) {
    logged_in_client
        .post(
            "/admin-api/v2/updates/",
            Some(vec![("Authorization", "Bearer TEST_TOKEN")]),
            None,
        )
        .await;
}
