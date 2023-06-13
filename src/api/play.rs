use actix_identity::Identity;
use actix_web::{web, HttpResponse};
use aes_gcm::{
    aead::{generic_array::GenericArray, rand_core::RngCore, Aead, OsRng},
    Aes256Gcm, KeyInit, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use octocrab::Octocrab;
use once_cell::sync::Lazy;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    api::error::{ApiError, PlaygroundError},
    db::{
        self,
        model::PlaygroundInsert,
        play::{create_playground, flag_playground},
        Pool,
    },
    settings::SETTINGS,
};

const FILENAME: &str = "playground.json";
const DESCRIPTION: &str = "Code shared from the MDN Playground";

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct PlayCode {
    html: Option<String>,
    css: Option<String>,
    js: Option<String>,
    src: Option<String>,
}
#[derive(Serialize)]
pub struct PlaySaveResponse {
    id: String,
}

#[derive(Debug)]
pub struct Gist {
    pub id: String,
    pub url: Url,
    pub code: PlayCode,
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
        let code = if files.len() != 1 {
            PlayCode::default()
        } else {
            let (_, content) = files.pop().unwrap();
            serde_json::from_str(&content.unwrap_or_default()).unwrap_or_default()
        };

        Gist {
            id: other.id,
            url: other.html_url,
            code,
        }
    }
}

pub async fn create_gist(
    client: &Octocrab,
    code: impl Into<String>,
) -> Result<Gist, PlaygroundError> {
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
    gist_id: String,
    id: String,
    reason: Option<String>,
) -> Result<(), PlaygroundError> {
    let repo = SETTINGS
        .playground
        .as_ref()
        .map(|p| p.flag_repo.as_str())
        .ok_or(PlaygroundError::SettingsError)?;
    let issues = client.issues("mdn", repo);
    let mut issue = issues.create(&format!("flag-{gist_id}"));
    if let Some(reason) = reason {
        issue = issue.body(&format!(
            "url: {}/en-US/play?id={}\n{reason}",
            &SETTINGS.application.document_base_url,
            utf8_percent_encode(&id, NON_ALPHANUMERIC)
        ));
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
    pool: web::Data<Pool>,
    github_client: web::Data<Option<Octocrab>>,
) -> Result<HttpResponse, ApiError> {
    if let Some(client) = &**github_client {
        if let Some(user_id) = id {
            let gist =
                create_gist(client, serde_json::to_string_pretty(&save.into_inner())?).await?;
            let mut conn = pool.get()?;
            let user = db::users::get_user(&mut conn, user_id.id().unwrap())?;
            create_playground(
                &mut conn,
                PlaygroundInsert {
                    user_id: Some(user.id),
                    gist: gist.id.clone(),
                    active: true,
                    ..Default::default()
                },
            )?;

            let id = encrypt(&gist.id)?;
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
        let gist = load_gist(client, &id).await?;
        Ok(HttpResponse::Ok().json(gist.code))
    } else {
        Ok(HttpResponse::NotImplemented().finish())
    }
}

pub async fn flag(
    flag: web::Json<PlayFlagRequest>,
    pool: web::Data<Pool>,
    github_client: web::Data<Option<Octocrab>>,
) -> Result<HttpResponse, ApiError> {
    if let Some(client) = &**github_client {
        let PlayFlagRequest { id, reason } = flag.into_inner();
        let gist_id = decrypt(&id)?;
        let mut conn = pool.get()?;
        flag_playground(&mut conn, &gist_id)?;
        create_flag_issue(client, gist_id, id, reason).await?;
        Ok(HttpResponse::Created().finish())
    } else {
        Ok(HttpResponse::NotImplemented().finish())
    }
}
