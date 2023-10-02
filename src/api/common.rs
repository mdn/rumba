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
        .get(document_url.to_owned())
        .send()
        .await
        .map_err(|err: reqwest::Error| match err.status() {
            Some(StatusCode::NOT_FOUND) => {
                warn!("Error NOT_FOUND fetching document {} ", &document_url);
                ApiError::DocumentNotFound
            }
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

#[derive(Serialize, Default)]
pub struct GeneratedChunkDelta {
    pub content: String,
}

#[derive(Serialize, Default)]
pub struct GeneratedChunkChoice {
    pub delta: GeneratedChunkDelta,
    pub finish_reason: Option<String>,
}
#[derive(Serialize)]
pub struct GeneratedChunk {
    pub choices: Vec<GeneratedChunkChoice>,
    pub id: i64,
}

impl Default for GeneratedChunk {
    fn default() -> Self {
        Self {
            choices: Default::default(),
            id: 1,
        }
    }
}

impl From<&str> for GeneratedChunk {
    fn from(content: &str) -> Self {
        GeneratedChunk {
            choices: vec![GeneratedChunkChoice {
                delta: GeneratedChunkDelta {
                    content: content.into(),
                },
                ..Default::default()
            }],
            ..Default::default()
        }
    }
}
