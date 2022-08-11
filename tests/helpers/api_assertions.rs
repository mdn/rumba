use actix_http::{
    body::{BoxBody, EitherBody},
    StatusCode,
};
use actix_web::dev::ServiceResponse;
use assert_json_diff::assert_json_include;
use slog::info;

use crate::helpers::read_json;

pub async fn assert_created_with_json_containing(
    res: ServiceResponse<EitherBody<BoxBody>>,
    expected_json: serde_json::Value,
) -> Result<(serde_json::Value), ()> {
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = read_json(res).await;
    assert_json_include!(actual: body, expected: expected_json);
    Ok(body)
}

pub async fn assert_ok_with_json_containing(
    res: ServiceResponse<EitherBody<BoxBody>>,
    expected_json: serde_json::Value,
) -> Result<(serde_json::Value), ()> {
    assert_eq!(res.status(), StatusCode::OK);
    let body = read_json(res).await;
    println!("{:}", body);
    assert_json_include!(actual: body, expected: expected_json);
    Ok(body)
}
