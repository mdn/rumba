use crate::helpers::api_assertions::{
    assert_bad_request_with_json_containing, assert_conflict_with_json_containing, assert_created,
    assert_created_returning_json, assert_created_with_json_containing, assert_ok,
    assert_ok_with_json_containing,
};
use crate::helpers::app::init_test;
use crate::helpers::http_client::PostPayload;
use crate::helpers::read_json;

use actix_http::StatusCode;
use anyhow::Error;
use serde_json::json;

#[actix_rt::test]
async fn test_create_and_get_collection() -> Result<(), Error> {
    let (mut client, stubr) =
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
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_add_items_to_collection() -> Result<(), Error> {
    let (mut client, stubr) =
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

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_collection_name_conflicts() -> Result<(), Error> {
    let (mut client, stubr) =
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
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_collection_item_conflicts() -> Result<(), Error> {
    let (mut client, stubr) =
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
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_edit_item_in_collection() -> Result<(), Error> {
    let (mut client, stubr) =
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
    let item_id = body["items"][0]["id"].as_str().unwrap();

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
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_delete_item_in_collection() -> Result<(), Error> {
    let (mut client, stubr) =
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
    let body =
        assert_ok_with_json_containing(res, json!({"id":collection_1,"article_count": 1})).await;
    let item_id = body["items"][0]["id"].as_str().unwrap();

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
    assert_ok_with_json_containing(
        res,
        json!({"id": collection_1,"article_count": 0, "items": []}),
    )
    .await;

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_delete_collection() -> Result<(), Error> {
    let (mut client, stubr) =
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
    assert_ok_with_json_containing(res, json!({"id":collection_1,"article_count": 1})).await;

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
            "message": format!("Collection with id {} not found",collection_1)
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
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_no_modify_delete_default() -> Result<(), Error> {
    let (mut client, stubr) =
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

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
async fn test_long_collection_name_is_bad_request() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";
    let two_hundred_twenty = "This is a really long title that has over 1024 characeters 5 times this is 1100. What were we thinking creeating such a long title really? 1024 really is a lot of character. To make this easier let's repeat this 5 times.".to_owned();
    let mut one_thousand_one_hundred = "".to_owned();
    for _ in 0..5 {
        one_thousand_one_hundred.push_str(two_hundred_twenty.clone().as_str())
    }
    let res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": one_thousand_one_hundred,
                "description": "Test description"
            }))),
        )
        .await;
    assert_bad_request_with_json_containing(res, json!({"code":400,"error":"Validation Error","message":"Error validating input name: 'name' must be between 1 and 1024 chars"})).await;
    Ok(())
}

#[actix_rt::test]
async fn test_very_long_collection_description_is_bad_request() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let one_hundred_char = "This sentence is exactly one hundred characters long which isn't that long but long enough for this.".to_owned();
    let mut sixty_six_thousand = "".to_owned();
    for _ in 0..660 {
        sixty_six_thousand.push_str(one_hundred_char.clone().as_str())
    }

    let res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "This name is short enough",
                "description": sixty_six_thousand
            }))),
        )
        .await;
    assert_bad_request_with_json_containing(res, json!({"code":400,"error":"Validation Error","message":"Error validating input description: 'description' must not be longer than 65536 chars"})).await;
    Ok(())
}

#[actix_rt::test]
async fn test_long_collection_item_title_is_bad_request() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "This is a really nice title",
                "description": "Test description"
            }))),
        )
        .await;
    let body = assert_created_returning_json(res).await;

    let two_hundred_twenty = "This is a really long title that has over 1024 characeters 5 times this is 1100. What were we thinking creeating such a long title really? 1024 really is a lot of character. To make this easier let's repeat this 5 times.".to_owned();
    let mut one_thousand_one_hundred = "".to_owned();
    for _ in 0..5 {
        one_thousand_one_hundred.push_str(two_hundred_twenty.clone().as_str())
    }

    let res = client
        .post(
            format!("{}{}/items/", base_url, body["id"].as_str().unwrap()).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : one_thousand_one_hundred,
                "url": "/en-US/docs/Web/CSS1"
            }
            ))),
        )
        .await;
    assert_bad_request_with_json_containing(res, json!({"code":400,"error":"Validation Error","message":"Error validating input title: 'title' must be between 1 and 1024 chars"})).await;
    Ok(())
}

#[actix_rt::test]
async fn test_very_long_collection_item_notes_is_bad_request() -> Result<(), Error> {
    let (mut client, _stubr) =
        init_test(vec!["tests/stubs", "tests/test_specific_stubs/collections"]).await?;
    let base_url = "/api/v2/collections/";

    let one_hundred_char = "This sentence is exactly one hundred characters long which isn't that long but long enough for this.".to_owned();
    let mut sixty_six_thousand = "".to_owned();
    for _ in 0..660 {
        sixty_six_thousand.push_str(one_hundred_char.clone().as_str())
    }

    let res = client
        .post(
            base_url,
            None,
            Some(PostPayload::Json(json!({
                "name": "This name is short enough",
                "description": "Short description"
            }))),
        )
        .await;
    let body = assert_created_returning_json(res).await;

    let one_hundred_char = "This sentence is exactly one hundred characters long which isn't that long but long enough for this.".to_owned();
    let mut sixty_six_thousand = "".to_owned();
    for _ in 0..660 {
        sixty_six_thousand.push_str(one_hundred_char.clone().as_str())
    }

    let res = client
        .post(
            format!("{}{}/items/", base_url, body["id"].as_str().unwrap()).as_str(),
            None,
            Some(PostPayload::Json(json!({
                "title" : "Short and sweet",
                "url": "/en-US/docs/Web/CSS1",
                "notes": sixty_six_thousand
            }
            ))),
        )
        .await;
    assert_bad_request_with_json_containing(res, json!({"code":400,"error":"Validation Error","message":"Error validating input notes: 'notes' must not be longer than 65536 chars"})).await;
    Ok(())
}
