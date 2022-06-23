use actix_http::StatusCode;
use actix_web::web::Data;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::{
    db::{self, model::DocumentMetadata},
    settings::SETTINGS,
};

use super::error::ApiError;

#[derive(Deserialize)]
pub enum Sorting {
    #[serde(rename = "title")]
    Title,
    #[serde(rename = "date")]
    Created,
}

#[derive(Deserialize)]
pub struct DocumentMetadataResponse {
    doc: DocumentMetadataExtract,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentMetadataExtract {
    pub mdn_url: String,
    pub parents: Option<Vec<db::model::CollectionParent>>,
    pub title: String,
}

pub async fn get_document_metadata(
    http_client: Data<Client>,
    url: &String,
) -> Result<DocumentMetadata, ApiError> {
    let document_url = Url::parse(&format!(
        "{}{}/index.json",
        SETTINGS.application.document_base_url, url
    ))
    .map_err(|_| ApiError::MalformedUrl)?;

    let document = http_client
        .get(document_url)
        .send()
        .await
        .map_err(|err: reqwest::Error| match err.status() {
            Some(StatusCode::NOT_FOUND) => ApiError::DocumentNotFound,
            _ => ApiError::Unknown,
        })?;

    let json: Value = document
        .json()
        .await
        .map_err(|_| ApiError::DocumentNotFound)?;

    let mut paths: Vec<String> = vec![];

    if let serde_json::Value::Array(val) = &json["doc"]["body"] {
        paths = val
            .iter()
            .filter_map(|element| {
                if element["type"] == "browser_compatibility" {
                    if let serde_json::Value::String(path) = &element["value"]["query"] {
                        return Some(path.clone());
                    }
                }
                None
            })
            .collect();
    };

    let metadata: DocumentMetadataResponse = serde_json::from_value(json)?;

    Ok(DocumentMetadata {
        mdn_url: metadata.doc.mdn_url,
        parents: metadata.doc.parents,
        title: metadata.doc.title,
        paths,
    })
}
