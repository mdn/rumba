use actix_identity::Identity;
use actix_web::{web, HttpResponse};
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    api::error::{ApiError, PlaygroundError},
    settings::SETTINGS,
};

const FILENAME: &str = "playground.html";
const DESCRIPTION: &str = "Code shared from the MDN Playground";

#[derive(Deserialize)]
pub struct PlaySaveRequest {
    code: String,
}
#[derive(Serialize)]
pub struct PlaySaveResponse {
    id: String,
}

#[derive(Serialize)]
pub struct PlayLoadResponse {
    code: String,
}

pub struct Gist {
    pub id: String,
    pub url: Url,
    pub code: String,
}

impl From<octocrab::models::gists::Gist> for Gist {
    fn from(other: octocrab::models::gists::Gist) -> Self {
        let mut files: Vec<_> = other
            .files
            .into_iter()
            .map(|(name, file)| (name, file.content))
            .collect();

        files.sort_by(|(name1, _), (name2, _)| name1.cmp(name2));

        let code = match files.len() {
            0 | 1 => files.into_iter().map(|(_, content)| content).collect(),
            _ => files
                .into_iter()
                .map(|(name, content)| format!("// {}\n{}\n\n", name, content))
                .collect(),
        };

        Gist {
            id: other.id,
            url: other.html_url,
            code,
        }
    }
}

pub async fn create_gist(code: String) -> Result<Gist, PlaygroundError> {
    github()?
        .gists()
        .create()
        .description(DESCRIPTION)
        .public(false)
        .file(FILENAME, code)
        .send()
        .await
        .map(Into::into)
        .map_err(Into::into)
}

pub async fn load_gist(id: &str) -> Result<Gist, PlaygroundError> {
    let github = github()?;

    github
        .gists()
        .get(id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

fn github() -> Result<Octocrab, PlaygroundError> {
    let token = SETTINGS
        .playground
        .as_ref()
        .map(|p| p.github_token.clone())
        .ok_or(PlaygroundError::TokenError)?;
    octocrab::OctocrabBuilder::new()
        .personal_token(token)
        .build()
        .map_err(Into::into)
}

pub async fn save(
    save: web::Json<PlaySaveRequest>,
    id: Option<Identity>,
) -> Result<HttpResponse, ApiError> {
    if id.is_some() {
        let gist = create_gist(save.into_inner().code).await?;
        return Ok(HttpResponse::Created().json(PlaySaveResponse { id: gist.id }));
    }
    Ok(HttpResponse::Unauthorized().finish())
}

pub async fn load(gist_id: web::Path<String>) -> Result<HttpResponse, ApiError> {
    let gist = load_gist(&gist_id).await?;
    Ok(HttpResponse::Created().json(PlayLoadResponse { code: gist.code }))
}
