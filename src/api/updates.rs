use actix_web::{
    web::{self, Data},
    HttpRequest, HttpResponse,
};
use reqwest::Client;
use serde_json::Value;

use crate::db::Pool;

use super::error::ApiError;

pub async fn update_bcd(
    pool: Data<Pool>,
    request: HttpRequest,
    client: web::Data<Client>,
) -> Result<HttpResponse, ApiError> {
    let res = client
        .get("https://registry.npmjs.org/@mdn/browser-compat-data")
        .send()
        .await
        .map_err(|_| ApiError::MalformedUrl)?;
    let json: Value = res
        .json()
        .await
        .map_err(|_| ApiError::JsonProcessingError)?;
    let version: Result<Vec<&str>,ApiError> = json["versions"]
        .as_object()    
        .ok_or(ApiError::Artificial)?
        .iter()        
        .map(|(_,v)| v["version"].as_str().ok_or(ApiError::Artificial))
        .collect();
    info!("{}", version.unwrap().join(","));
    
    Ok(HttpResponse::Ok().finish())
}
