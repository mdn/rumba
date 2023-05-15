use actix_identity::Identity;
use actix_web::{web, HttpResponse};
use aes_gcm::{
    aead::{generic_array::GenericArray, rand_core::RngCore, Aead, OsRng},
    Aes256Gcm, KeyInit, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
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

pub(crate) const NONCE_LEN: usize = 12;

fn encrypt(gist_id: &str) -> Result<String, PlaygroundError> {
    let settings = &SETTINGS
        .playground
        .as_ref()
        .ok_or(PlaygroundError::SettingsError)?;
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&settings.crypt_key));
    let mut nonce = vec![0; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let nonce = Nonce::from_slice(&nonce);
    let mut data = cipher.encrypt(nonce, gist_id.as_bytes())?;
    data.extend_from_slice(nonce.as_slice());

    Ok(STANDARD.encode(data))
}

fn decrypt(encoded: &str) -> Result<String, PlaygroundError> {
    let settings = &SETTINGS
        .playground
        .as_ref()
        .ok_or(PlaygroundError::SettingsError)?;
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&settings.crypt_key));
    let data = STANDARD.decode(encoded)?;
    let (enc, nonce) = data.split_at(data.len() - NONCE_LEN);
    let nonce = Nonce::from_slice(nonce);
    let data = cipher.decrypt(nonce, enc)?;

    Ok(String::from_utf8(data)?)
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
            0 | 1 => files
                .into_iter()
                .map(|(_, content)| content.unwrap_or_default())
                .collect(),
            _ => files
                .into_iter()
                .map(|(name, content)| format!("// {}\n{}\n\n", name, content.unwrap_or_default()))
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
        .ok_or(PlaygroundError::SettingsError)?;
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
        let id = encrypt(&gist.id)?;
        println!("save: {id}");
        return Ok(HttpResponse::Created().json(PlaySaveResponse { id }));
    }
    Ok(HttpResponse::Unauthorized().finish())
}

pub async fn load(gist_id: web::Path<String>) -> Result<HttpResponse, ApiError> {
    let id = decrypt(&gist_id.into_inner())?;
    println!("load: {id}");
    let gist = load_gist(&id).await?;
    Ok(HttpResponse::Created().json(PlayLoadResponse { code: gist.code }))
}
