use crate::helpers::read_json;
use actix_http::StatusCode;
use assert_json_diff::assert_json_include;

use super::RumbaTestResponse;

pub async fn assert_created_with_json_containing(
    res: RumbaTestResponse,
    expected_json: serde_json::Value,
) -> serde_json::Value {
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = read_json(res).await;
    assert_json_include!(actual: body, expected: expected_json);
    body
}

pub fn assert_created(res: RumbaTestResponse) {
    assert_eq!(res.status(), StatusCode::CREATED);
}

pub fn assert_ok(res: RumbaTestResponse) {
    assert_eq!(res.status(), StatusCode::OK);
}

pub async fn assert_ok_with_json_containing(
    res: RumbaTestResponse,
    expected_json: serde_json::Value,
) -> serde_json::Value {
    assert_eq!(res.status(), StatusCode::OK);
    let body = read_json(res).await;
    assert_json_include!(actual: body, expected: expected_json);
    body
}

pub async fn assert_bad_request_with_json_containing(
    res: RumbaTestResponse,
    expected_json: serde_json::Value,
) -> serde_json::Value {
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = read_json(res).await;
    assert_json_include!(actual: body, expected: expected_json);
    body
}

pub async fn assert_conflict_with_json_containing(
    res: RumbaTestResponse,
    expected_json: serde_json::Value,
) -> serde_json::Value {
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let body = read_json(res).await;
    assert_json_include!(actual: body, expected: expected_json);
    body
}
