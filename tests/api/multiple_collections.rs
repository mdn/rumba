use crate::helpers::api_assertions::{
    assert_bad_request_with_json_containing, assert_conflict_with_json_containing, assert_created,
    assert_created_with_json_containing, assert_ok, assert_ok_with_json_containing,
};
use crate::helpers::app::init_test;
use crate::helpers::http_client::PostPayload;
use crate::helpers::read_json;

use actix_http::StatusCode;
use anyhow::Error;
use serde_json::json;

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
    .await;

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
    .await;
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
        res,
        json!(
            {
                "name": "Test",
                "description": "Test description",
                "article_count" : 0
            }
        ),
    )
    .await;

    let c_id = body["id"].as_str().unwrap();

    for i in 1..12 {
        let create_res = client
            .post(
                format!("{}{}/items/", base_url, c_id).as_str(),
                None,
                Some(PostPayload::Json(json!({
                    "title" : format!("Interesting CSS{}",i),
                    "url": format!("/en-US/docs/Web/CSS{}",i)
                }
                ))),
            )
            .await;
        assert_eq!(create_res.status(), StatusCode::CREATED);
    }

    let res = client
        .get(format!("{}{}/", base_url, c_id).as_str(), None)
        .await;
    assert_ok_with_json_containing(
        res,
        json!({
            "article_count": 11,
            "items": [
                {"url" : "/en-US/docs/Web/CSS11"},
                {"url" : "/en-US/docs/Web/CSS10"},
                {"url" : "/en-US/docs/Web/CSS9"},
                {"url" : "/en-US/docs/Web/CSS8"},
                {"url" : "/en-US/docs/Web/CSS7"},
                {"url" : "/en-US/docs/Web/CSS6"},
                {"url" : "/en-US/docs/Web/CSS5"},
                {"url" : "/en-US/docs/Web/CSS4"},
                {"url" : "/en-US/docs/Web/CSS3"},
                {"url" : "/en-US/docs/Web/CSS2"},
                ]

        }),
    )
    .await;

    let res = client
        .get(
            format!("{}{}/?offset=10&limit=1", base_url, c_id).as_str(),
            None,
        )
        .await;
    assert_ok_with_json_containing(
        res,
        json!({
            "article_count": 11,
            "items": [
                {"url" : "/en-US/docs/Web/CSS1"},
                ]

        }),
    )
    .await;

    Ok(())
}

#[actix_rt::test]
async fn test_collection_name_conflicts() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let mut res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "Test",
                "description": "Test description"
            }))),
        )
        .await;

    assert_created_with_json_containing(
        res,
        json!(
            {
                "name": "Test",
                "description": "Test description",
                "article_count" : 0
            }
        ),
    )
    .await;

    res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "Test",
                "description": "Test description"
            }))),
        )
        .await;
    //Same name should be a conflict
    assert_conflict_with_json_containing(
        res,
        json!({
            "error" : "Collection with name 'Test' already exists"
        }),
    )
    .await;

    res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "Test 2",
                "description": "Test description"
            }))),
        )
        .await;
    assert_created_with_json_containing(res, json!({"name":"Test 2"})).await;
    Ok(())
}

#[actix_rt::test]
async fn test_collection_item_conflicts() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let mut res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "Test",
                "description": "Test description"
            }))),
        )
        .await;
    let res_1 = assert_created_with_json_containing(res, json!({"name":"Test"})).await;
    let collection_1 = res_1["id"].as_str().unwrap();
    res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "Test 2",
                "description": "Test description"
            }))),
        )
        .await;

    let res_2 = assert_created_with_json_containing(res, json!({"name":"Test 2"})).await;
    let collection_2 = res_2["id"].as_str().unwrap();

    res = client
        .post(
            format!("{}{}/items/", base_url, collection_1).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Interesting CSS1",
                "url": "/en-US/docs/Web/CSS1"
            }
            ))),
        )
        .await;
    assert_created(res);
    //Test SAME Collection_item different collection is ok
    res = client
        .post(
            format!("{}{}/items/", base_url, collection_2).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Interesting CSS1",
                "url": "/en-US/docs/Web/CSS1"
            }
            ))),
        )
        .await;
    assert_created(res);

    //Test SAME collection_item (by url) Same collection is conflict
    res = client
        .post(
            format!("{}{}/items/", base_url, collection_2).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Interesting CSS1",
                "url": "/en-US/docs/Web/CSS1"
            }
            ))),
        )
        .await;
    assert_conflict_with_json_containing(
        res,
        json!({
            "error" : "Collection item already exists in collection"
        }),
    )
    .await;
    Ok(())
}

#[actix_rt::test]
async fn test_edit_item_in_collection() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let mut res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "Test",
                "description": "Test description"
            }))),
        )
        .await;
    let res_1 = assert_created_with_json_containing(res, json!({"name":"Test"})).await;
    let collection_1 = res_1["id"].as_str().unwrap();

    res = client
        .post(
            format!("{}{}/items/", base_url, collection_1).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Interesting CSS1",
                "url": "/en-US/docs/Web/CSS1"
            }
            ))),
        )
        .await;

    assert_created(res);
    res = client
        .get(format!("{}{}/", base_url, collection_1).as_str(), None)
        .await;
    let body = read_json(res).await;
    let item_id = body["items"][0]["id"].as_i64().unwrap();

    res = client
        .post(
            format!("{}{}/items/{}/", base_url, collection_1, item_id).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Interesting CSS1 Custom name",
                "notes": "Cool notes"
            }
            ))),
        )
        .await;
    assert_ok(res);
    res = client
        .get(
            format!("{}{}/items/{}/", base_url, collection_1, item_id).as_str(),
            None,
        )
        .await;
    assert_ok_with_json_containing(
        res,
        json!({
            "id" : item_id,
            "notes" : "Cool notes",
            "title" : "Interesting CSS1 Custom name"
        }),
    )
    .await;
    Ok(())
}

#[actix_rt::test]
async fn test_delete_item_in_collection() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let mut res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "Test",
                "description": "Test description"
            }))),
        )
        .await;
    let res_1 = assert_created_with_json_containing(res, json!({"name":"Test"})).await;
    let collection_1 = res_1["id"].as_str().unwrap();

    res = client
        .post(
            format!("{}{}/items/", base_url, collection_1).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Interesting CSS1",
                "url": "/en-US/docs/Web/CSS1"
            }
            ))),
        )
        .await;

    assert_created(res);
    res = client
        .get(format!("{}{}/", base_url, collection_1).as_str(), None)
        .await;
    let body = assert_ok_with_json_containing(res, json!({"id":"2","article_count": 1})).await;
    let item_id = body["items"][0]["id"].as_i64().unwrap();

    res = client
        .delete(
            format!("{}{}/items/{}/", base_url, collection_1, item_id).as_str(),
            None,
        )
        .await;
    assert_ok(res);
    res = client
        .get(format!("{}{}/", base_url, collection_1).as_str(), None)
        .await;
    assert_ok_with_json_containing(res, json!({"id":"2","article_count": 0, "items": []})).await;

    Ok(())
}

#[actix_rt::test]
async fn test_delete_collection() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let mut res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "Test",
                "description": "Test description"
            }))),
        )
        .await;
    let res_1 = assert_created_with_json_containing(res, json!({"name":"Test"})).await;
    let collection_1 = res_1["id"].as_str().unwrap();

    res = client
        .post(
            format!("{}{}/items/", base_url, collection_1).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Interesting CSS1",
                "url": "/en-US/docs/Web/CSS1"
            }
            ))),
        )
        .await;

    assert_created(res);
    res = client
        .get(format!("{}{}/", base_url, collection_1).as_str(), None)
        .await;
    assert_ok_with_json_containing(res, json!({"id":"2","article_count": 1})).await;

    //Delete collection
    res = client
        .delete(format!("{}{}/", base_url, collection_1).as_str(), None)
        .await;
    assert_ok(res);
    res = client
        .get(format!("{}{}/", base_url, collection_1).as_str(), None)
        .await;
    assert_bad_request_with_json_containing(
        res,
        json!({
            "code": 400,
            "error": "Collection not found",
            "message": "Collection with id 2 not found"
        }),
    )
    .await;
    //Recreate the collection.
    res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "Test",
                "description": "Test description"
            }))),
        )
        .await;
    let recreated = assert_created_with_json_containing(res, json!({"name":"Test"})).await;
    let collection_1 = recreated["id"].as_str().unwrap();

    res = client
        .post(
            format!("{}{}/items/", base_url, collection_1).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Interesting CSS1",
                "url": "/en-US/docs/Web/CSS1"
            }
            ))),
        )
        .await;

    assert_created(res);
    Ok(())
}

#[actix_rt::test]
async fn test_no_modify_delete_default() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let all = client.get(base_url, None).await;
    let json = assert_ok_with_json_containing(all, json!([{"name":"Default"}])).await;
    let default_collection_id = json[0]["id"].as_str().unwrap();
    let res = client
        .post(
            format!("{}{}/", base_url, default_collection_id).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "name" : "Default",
                "notes": "This is the default collection. I can add notes :)"
            }
            ))),
        )
        .await;
    assert_ok(res);
    let res = client
        .post(
            format!("{}{}/", base_url, default_collection_id).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "name" : "Changing Default name oh noes!",
            }
            ))),
        )
        .await;
    assert_bad_request_with_json_containing(
        res,
        json!({"error": "Cannot modify default collection"}),
    )
    .await;

    let delete_res = client
        .delete(
            format!("{}{}/", base_url, default_collection_id).as_str(),
            None,
        )
        .await;
    assert_bad_request_with_json_containing(
        delete_res,
        json!({"error": "Cannot delete default collection"}),
    )
    .await;

    Ok(())
}
