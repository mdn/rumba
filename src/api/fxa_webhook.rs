use std::collections::BTreeMap;

use actix_rt::ArbiterHandle;
use actix_web::{dev::HttpServiceFactory, web, HttpRequest, HttpResponse};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use base64;
use chrono::{DateTime, Utc};
use openidconnect::{
    core::{CoreJsonWebKey, CoreJwsSigningAlgorithm},
    Audience, JsonWebKey,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    api::error::FxaWebhookError,
    db::{
        error::DbError,
        fxa_webhook::{
            delete_profile_from_webhook, log_failed_webhook_event, update_profile_from_webhook,
        },
    },
    helpers::{deserialize_string_or_vec, serde_utc_milliseconds, serde_utc_seconds_f},
};
use crate::{
    db::{fxa_webhook::update_subscription_state_from_webhook, Pool},
    fxa::{types::Subscription, LoginManager},
};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionStateChange {
    pub capabilities: Vec<Subscription>,
    pub is_active: bool,
    #[serde(with = "serde_utc_milliseconds")]
    pub change_time: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PasswordChange {
    #[serde(with = "serde_utc_milliseconds")]
    pub change_time: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProfileChange {
    pub email: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeleteUser {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FxAEvents {
    #[serde(rename = "https://schemas.accounts.firefox.com/event/subscription-state-change")]
    pub subscription_state_change: Option<SubscriptionStateChange>,
    #[serde(rename = "https://schemas.accounts.firefox.com/event/password-change")]
    pub password_change: Option<PasswordChange>,
    #[serde(rename = "https://schemas.accounts.firefox.com/event/profile-change")]
    pub profile_change: Option<ProfileChange>,
    #[serde(rename = "https://schemas.accounts.firefox.com/event/delete-user")]
    pub delete_user: Option<DeleteUser>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FxASetTokenPayload {
    pub events: FxAEvents,
    #[serde(rename = "iss")]
    pub issuer: openidconnect::IssuerUrl,
    #[serde(
        default,
        rename = "aud",
        deserialize_with = "deserialize_string_or_vec"
    )]
    pub audiences: Vec<Audience>,
    #[serde(rename = "iat", with = "serde_utc_seconds_f")]
    pub issue_time: DateTime<Utc>,
    #[serde(rename = "jti")]
    pub jwt_id: Uuid,
    #[serde(rename = "sub")]
    pub fxa_uid: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FxASetTokenHeader {
    #[serde(rename = "alg")]
    pub algorithm: CoreJwsSigningAlgorithm,
}

fn verify(raw_token: &str, key: &CoreJsonWebKey) -> Result<FxASetTokenPayload, FxaWebhookError> {
    let parts = raw_token.split('.').collect::<Vec<_>>();

    if parts.len() != 3 {
        return Err(FxaWebhookError::InvalidSET);
    }

    let header_json = base64::decode_config(parts[0], base64::URL_SAFE_NO_PAD)?;
    let header: FxASetTokenHeader = serde_json::from_slice(&header_json)?;

    let raw_payload = base64::decode_config(parts[1], base64::URL_SAFE_NO_PAD)?;
    let payload: FxASetTokenPayload = serde_json::from_slice(&raw_payload)?;

    let signature = base64::decode_config(parts[2], base64::URL_SAFE_NO_PAD)?;

    let signing_input = format!("{}.{}", parts[0], parts[1]);
    key.verify_signature(&header.algorithm, signing_input.as_bytes(), &signature)?;
    Ok(payload)
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct EventsClaim {
    events: BTreeMap<String, Value>,
}

async fn process_event(
    conn_pool: web::Data<Pool>,
    payload: FxASetTokenPayload,
    login_manager: web::Data<LoginManager>,
    arbiter: web::Data<ArbiterHandle>,
) -> Result<(), DbError> {
    if payload.events.password_change.is_some() {
        debug!("skipped password change event for {}", payload.fxa_uid);
    }
    if let Some(profile_change) = payload.events.profile_change {
        update_profile_from_webhook(
            conn_pool.clone(),
            arbiter,
            payload.fxa_uid.clone(),
            login_manager,
            profile_change,
            payload.issue_time,
        )
        .await?;
    }
    if let Some(subscription_state_change) = payload.events.subscription_state_change {
        update_subscription_state_from_webhook(
            conn_pool.clone(),
            payload.fxa_uid.clone(),
            subscription_state_change,
            payload.issue_time,
        )?;
    }
    if payload.events.delete_user.is_some() {
        delete_profile_from_webhook(
            conn_pool.clone(),
            payload.fxa_uid.clone(),
            payload.issue_time,
        )?;
    }
    Ok(())
}

async fn set_token(
    _req: HttpRequest,
    auth: BearerAuth,
    login_manager: web::Data<LoginManager>,
    arbiter: web::Data<ArbiterHandle>,
    pool: web::Data<Pool>,
) -> HttpResponse {
    let mut error = None;
    let mut fail = false;
    for key in login_manager.metadata.jwks().keys() {
        match verify(auth.token(), key) {
            Ok(payload) => {
                let fxa_uid = payload.fxa_uid.clone();
                return match process_event(pool.clone(), payload, login_manager, arbiter).await {
                    Ok(_) => HttpResponse::Ok().finish(),
                    Err(e) => {
                        // This means either our db connections has issues or our worker thread.
                        // Sending a non 200 to trigger a retry.
                        fail = true;
                        error = Some(format!(
                            "Error processing webhook event for {}: {}",
                            fxa_uid, e
                        ));
                        break;
                    }
                };
            }
            Err(e) => warn!("Error validating SET: {}", e),
        }
    }

    // if we didn't set an error but end up here we failed to validate.
    let error = error.unwrap_or_else(|| "Unable to validate SET".to_string());
    error!("{}", error);
    if let Err(e) = log_failed_webhook_event(pool, auth.token(), &error) {
        error!("Unable to log failed_webhook_event: {}", e);
    }

    if fail {
        HttpResponse::ServiceUnavailable().finish()
    } else {
        HttpResponse::Ok().finish()
    }
}

pub fn fxa_webhook_app() -> impl HttpServiceFactory {
    web::scope("/events").service(web::resource("/fxa").to(set_token))
}
