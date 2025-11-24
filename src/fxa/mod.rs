pub mod error;
pub mod types;
use actix_web::web;
use openidconnect::core::CoreTokenResponse;
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreGenderClaim, CoreProviderMetadata,
};
use openidconnect::http::header::{ACCEPT, AUTHORIZATION};
use openidconnect::http::Method;
use openidconnect::http::StatusCode;
use openidconnect::{
    AdditionalClaims, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    RedirectUrl, Scope,
};
use openidconnect::{AsyncHttpClient, OAuth2TokenResponse, RefreshToken, UserInfoClaims};
use openidconnect::{EndpointMaybeSet, EndpointSet};
use serde::{Deserialize, Serialize};

use url::Url;

use crate::db::users::create_or_update_user;
use crate::db::Pool;
use crate::fxa::error::FxaError;

use crate::settings::SETTINGS;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FxAClaims {
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub avatar_default: bool,
    pub amr_values: Vec<String>,
    pub uid: String,
    #[serde(default)]
    pub subscriptions: Vec<types::Subscription>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FxAUser {
    pub email: String,
    pub locale: Option<String>,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub avatar_default: bool,
    pub amr_values: Vec<String>,
    pub uid: String,
    #[serde(default)]
    pub subscriptions: Vec<types::Subscription>,
}

impl From<UserInfoClaims<FxAClaims, CoreGenderClaim>> for FxAUser {
    fn from(claims: UserInfoClaims<FxAClaims, CoreGenderClaim>) -> Self {
        let fxa_claims = claims.additional_claims().clone();
        Self {
            email: claims.email().map(|e| e.to_string()).unwrap_or_default(),
            locale: claims.locale().map(|l| l.to_string()),
            display_name: fxa_claims.display_name,
            avatar: fxa_claims.avatar,
            avatar_default: fxa_claims.avatar_default,
            amr_values: fxa_claims.amr_values,
            uid: fxa_claims.uid,
            subscriptions: fxa_claims.subscriptions,
        }
    }
}

#[derive(Deserialize)]
pub struct AuthResponse {
    pub code: String,
    pub state: String,
}

impl AdditionalClaims for FxAClaims {}

pub struct LoginManager {
    pub login_client: CoreClient<
        EndpointSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointNotSet,
        EndpointMaybeSet,
        EndpointMaybeSet,
    >,
    pub metadata: CoreProviderMetadata,
    pub http_client: reqwest::Client,
}

impl LoginManager {
    pub async fn init() -> Result<Self, FxaError> {
        // Create HTTP client with redirects disabled for security (prevents SSRF)
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| FxaError::Oidc(e.into()))?;

        let provider_metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new(SETTINGS.auth.issuer_url.clone())?,
            &http_client,
        )
        .await
        .map_err(|e| FxaError::Oidc(e.into()))?;

        let login_client = CoreClient::from_provider_metadata(
            provider_metadata.clone(),
            ClientId::new(SETTINGS.auth.client_id.clone()),
            Some(ClientSecret::new(SETTINGS.auth.client_secret.clone())),
        )
        .set_redirect_uri(RedirectUrl::from_url(SETTINGS.auth.redirect_url.clone()));
        Ok(LoginManager {
            login_client,
            metadata: provider_metadata,
            http_client,
        })
    }

    pub fn login(&self, email: Option<String>) -> (Url, CsrfToken, Nonce) {
        let mut auth_req = self
            .login_client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new(SETTINGS.auth.scopes.clone()))
            .add_extra_param("access_type", "offline");
        if let Some(email) = email {
            auth_req = auth_req
                .add_extra_param("prompt", "none")
                .add_extra_param("login_hint", email);
        }
        let (auth_url, csrf_token, nonce) = auth_req.url();
        (auth_url, csrf_token, nonce)
    }

    pub async fn callback(
        &self,
        code: String,
        _nonce: Nonce,
        pool: &web::Data<Pool>,
    ) -> Result<String, FxaError> {
        let token_response = self
            .login_client
            .exchange_code(AuthorizationCode::new(code))
            .map_err(|e| FxaError::Oidc(anyhow::anyhow!("Token endpoint not available: {}", e)))?
            .request_async(&self.http_client)
            .await
            .map_err(|e| FxaError::Oidc(e.into()))?;

        let _id_token = token_response
            .extra_fields()
            .id_token()
            .ok_or(FxaError::IdTokenMissing)?;
        // let claims = id_token.claims(&self.login_client.id_token_verifier(), &nonce)?;

        let refresh_token = token_response
            .refresh_token()
            .map(|r| r.secret().clone())
            .unwrap_or_default();

        let user = self.get_fxa_user(token_response).await?;

        let uid = user.uid.clone();
        let mut pg_conn = pool.get()?;

        web::block(move || create_or_update_user(&mut pg_conn, user, &refresh_token)).await??;
        Ok(uid)
    }

    pub async fn get_fxa_user(
        &self,
        token_response: CoreTokenResponse,
    ) -> Result<FxAUser, FxaError> {
        let auth_value = format!("Bearer {}", token_response.access_token().secret());

        let userinfo_url = self
            .metadata
            .userinfo_endpoint()
            .ok_or_else(|| FxaError::Oidc(anyhow::anyhow!("UserInfo endpoint not available")))?;

        let req = openidconnect::http::Request::builder()
            .method(Method::GET)
            .uri(userinfo_url.url().as_str())
            .header(ACCEPT, "application/json")
            .header(AUTHORIZATION, auth_value)
            .body(Vec::new())
            .expect("Failed to build request");

        let http_response = self
            .http_client
            .call(req)
            .await
            .map_err(|e| FxaError::UserInfoError(openidconnect::UserInfoError::Request(e)))?;
        if http_response.status() != StatusCode::OK {
            // Convert http 1.0 StatusCode to actix_http StatusCode (http 0.2)
            let status = actix_http::StatusCode::from_u16(http_response.status().as_u16())
                .unwrap_or(actix_http::StatusCode::INTERNAL_SERVER_ERROR);
            return Err(FxaError::UserInfoBadStatus(status));
        }
        Ok(serde_json::from_slice(http_response.body())?)

        // All code above should just be the following but FxA doesn't add
        // the openid scope for the refreshed access token :/
        //
        // let claims: UserInfoClaims<FxAClaims, CoreGenderClaim> = self
        //     .login_client
        //     .user_info(token_response.access_token().clone(), None)
        //     .unwrap()
        //     .request_async(async_http_client)
        //     .await?;
        // Ok(claims.into())
    }

    pub async fn get_and_update_user_info_with_refresh_token(
        &self,
        pool: &web::Data<Pool>,
        refresh_token: String,
    ) -> Result<(), FxaError> {
        let res = self
            .login_client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
            .map_err(|e| FxaError::Oidc(anyhow::anyhow!("Token endpoint not available: {}", e)))?
            .add_extra_param("ttl", "300")
            .add_scope(Scope::new(SETTINGS.auth.scopes.clone()))
            .request_async(&self.http_client)
            .await
            .map_err(|e| FxaError::Oidc(e.into()))?;
        let user = self.get_fxa_user(res).await?;
        let mut pg_conn = pool.get()?;
        web::block(move || create_or_update_user(&mut pg_conn, user, &refresh_token)).await??;
        Ok(())
    }
}
