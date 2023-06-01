use actix_identity::Identity;
use actix_web::{web, HttpResponse};
use aes_gcm::{
    aead::{generic_array::GenericArray, rand_core::RngCore, Aead, OsRng},
    Aes256Gcm, KeyInit, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use octocrab::Octocrab;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    api::{
        error::{ApiError, PlaygroundError},
    },
    settings::SETTINGS,
};

const FILENAME: &str = "playground.html";
const DESCRIPTION: &str = "Code shared from the MDN Playground";

#[derive(Deserialize, Serialize)]
pub struct PlayCode {
    code: String,
}
#[derive(Serialize)]
pub struct PlaySaveResponse {
    id: String,
}

pub struct Gist {
    pub id: String,
    pub url: Url,
    pub code: String,
}

#[derive(Deserialize)]
pub struct PlayFlagRequest {
    id: String,
    reason: Option<String>,
}

pub(crate) const NONCE_LEN: usize = 12;
static CIPHER: Lazy<Option<Aes256Gcm>> = Lazy::new(|| {
    SETTINGS
        .playground
        .as_ref()
        .map(|playground| Aes256Gcm::new(GenericArray::from_slice(&playground.crypt_key)))
});

fn encrypt(gist_id: &str) -> Result<String, PlaygroundError> {
    if let Some(cipher) = &*CIPHER {
        let mut nonce = vec![0; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce);
        let nonce = Nonce::from_slice(&nonce);
        let mut data = cipher.encrypt(nonce, gist_id.as_bytes())?;
        data.extend_from_slice(nonce.as_slice());

        Ok(STANDARD.encode(data))
    } else {
        Err(PlaygroundError::SettingsError)
    }
}

fn decrypt(encoded: &str) -> Result<String, PlaygroundError> {
    if let Some(cipher) = &*CIPHER {
        let data = STANDARD.decode(encoded)?;
        let (enc, nonce) = data.split_at(data.len() - NONCE_LEN);
        let nonce = Nonce::from_slice(nonce);
        let data = cipher.decrypt(nonce, enc)?;

        Ok(String::from_utf8(data)?)
    } else {
        Err(PlaygroundError::SettingsError)
    }
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

pub async fn create_gist(client: &Octocrab, code: String) -> Result<Gist, PlaygroundError> {
    client
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

pub async fn create_flag_issue(
    client: &Octocrab,
    id: String,
    reason: Option<String>,
) -> Result<(), PlaygroundError> {
    let repo = SETTINGS
        .playground
        .as_ref()
        .map(|p| p.flag_repo.as_str())
        .ok_or(PlaygroundError::SettingsError)?;
    let issues = client.issues("mdn", repo);
    let mut issue = issues.create(&format!("flag-{id}"));
    if let Some(reason) = reason {
        issue = issue.body(&reason);
    }
    issue.send().await.map(|_| ()).map_err(Into::into)
}

pub async fn load_gist(client: &Octocrab, id: &str) -> Result<Gist, PlaygroundError> {
    client
        .gists()
        .get(id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

pub async fn save(
    save: web::Json<PlayCode>,
    id: Option<Identity>,
    github_client: web::Data<Option<Octocrab>>,
) -> Result<HttpResponse, ApiError> {
    if let Some(client) = &**github_client {
        if id.is_some() {
            let gist = create_gist(client, save.into_inner().code).await?;
            let id = encrypt(&gist.id)?;
            println!("save: {id}");
            Ok(HttpResponse::Created().json(PlaySaveResponse { id }))
        } else {
            Ok(HttpResponse::Unauthorized().finish())
        }
    } else {
        Ok(HttpResponse::NotImplemented().finish())
    }
}

pub async fn load(
    gist_id: web::Path<String>,
    github_client: web::Data<Option<Octocrab>>,
) -> Result<HttpResponse, ApiError> {
    if let Some(client) = &**github_client {
        let id = decrypt(&gist_id.into_inner())?;
        println!("load: {id}");
        let gist = load_gist(client, &id).await?;
        Ok(HttpResponse::Created().json(PlayCode { code: gist.code }))
    } else {
        Ok(HttpResponse::NotImplemented().finish())
    }
}

pub async fn flag(
    flag: web::Json<PlayFlagRequest>,
    github_client: web::Data<Option<Octocrab>>,
) -> Result<HttpResponse, ApiError> {
    if let Some(client) = &**github_client {
        let PlayFlagRequest { id, reason } = flag.into_inner();
        let gist_id = decrypt(&id)?;
        create_flag_issue(client, gist_id, reason).await?;
        Ok(HttpResponse::Created().finish())
    } else {
        Ok(HttpResponse::NotImplemented().finish())
    }
}
