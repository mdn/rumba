use crate::helpers::read_json;
use actix_http::{
    body::{BoxBody, EitherBody},
    StatusCode,
};
use actix_web::dev::ServiceResponse;
use assert_json_diff::assert_json_include;

pub async fn assert_created_with_json_containing(
    res: ServiceResponse<EitherBody<EitherBody<BoxBody>>>,
    expected_json: serde_json::Value,
) -> serde_json::Value {
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = read_json(res).await;
    assert_json_include!(actual: body, expected: expected_json);
    body
}

pub fn assert_created(res: ServiceResponse<EitherBody<EitherBody<BoxBody>>>) {
    assert_eq!(res.status(), StatusCode::CREATED);
}

pub async fn assert_ok_with_json_containing(
    res: ServiceResponse<EitherBody<EitherBody<BoxBody>>>,
    expected_json: serde_json::Value,
) -> serde_json::Value {
    assert_eq!(res.status(), StatusCode::OK);
    let body = read_json(res).await;
    assert_json_include!(actual: body, expected: expected_json);
    body
}

pub async fn assert_conflict_with_json_containing(
    res: ServiceResponse<EitherBody<EitherBody<BoxBody>>>,
    expected_json: serde_json::Value,
) -> serde_json::Value {
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let body = read_json(res).await;
    assert_json_include!(actual: body, expected: expected_json);
    body
}

pub fn assert_conflict(res: ServiceResponse<EitherBody<EitherBody<BoxBody>>>) {
    assert_eq!(res.status(), StatusCode::CONFLICT);
}
