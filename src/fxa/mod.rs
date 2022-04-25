use anyhow::anyhow;
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::reqwest::async_http_client;
use openidconnect::{
    AdditionalClaims, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    RedirectUrl, Scope,
};
use openidconnect::{OAuth2TokenResponse, TokenResponse};
use reqwest::{Client, IntoUrl, Method, RequestBuilder};
use serde::{Deserialize, Serialize};

use url::Url;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FxAUser {
    pub email: String,
    pub locale: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub avatar_default: bool,
    pub amr_values: Vec<String>,
    pub uid: String,
    #[serde(default)]
    pub subscriptions: Vec<String>,
}

#[derive(Deserialize)]
pub struct AuthResponse {
    pub code: String,
    pub state: String,
}

impl AdditionalClaims for FxAUser {}

pub struct LoginManager {
    login_client: CoreClient,
    http_client: Client,
    user_info_endpoint: Url,
}
use anyhow::Error;

use crate::settings::SETTINGS;

impl LoginManager {
    pub async fn init() -> Result<Self, Error> {
        let provider_metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new(SETTINGS.auth.issuer_url.clone())?,
            async_http_client,
        )
        .await?;
        let user_info_endpoint = provider_metadata.userinfo_endpoint().unwrap().url().clone();

        let login_client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(SETTINGS.auth.client_id.clone()),
            Some(ClientSecret::new(SETTINGS.auth.client_secret.clone())),
        )
        .set_redirect_uri(RedirectUrl::from_url(SETTINGS.auth.redirect_url.clone()));

        Ok(LoginManager {
            login_client,
            http_client: Client::new(),
            user_info_endpoint,
        })
    }

    pub fn login(&mut self) -> (Url, CsrfToken) {
        let (auth_url, csrf_token, _nonce) = self
            .login_client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new(SETTINGS.auth.scopes.clone()))
            .add_extra_param("access_type", "offline")
            .url();
        (auth_url, csrf_token)
    }

    pub async fn callback(&mut self, code: String) -> Result<String, Error> {
        println!("{}", code);
        let token_response = self
            .login_client
            .exchange_code(AuthorizationCode::new(code))
            .request_async(async_http_client)
            .await?;
        println!("token");
        let id_token = token_response
            .id_token()
            .ok_or_else(|| anyhow!("Server did not return an ID token"))?;
        println!("{}", id_token.to_string());
        //let claims = id_token.claims(&self.login_client.id_token_verifier(), &nonce)?;

        let access_token = token_response.access_token().secret().clone();
        println!("access: {:?}", &access_token);
        let refresh_token = token_response
            .refresh_token()
            .map(|r| r.secret().clone())
            .unwrap_or_default();
        println!("refresh: {:?}", refresh_token);

        let res = self
            .request(Method::GET, self.user_info_endpoint.clone(), &access_token)
            .send()
            .await?;
        let user: FxAUser = res.json().await?;

        let uid = user.uid.clone();
        println!("{:#?}", serde_json::to_string_pretty(&user));
        //TODO: add user to db
        Ok(uid)
    }
    fn request<U: IntoUrl>(&self, method: Method, url: U, bearer: &str) -> RequestBuilder {
        self.http_client.request(method, url).bearer_auth(bearer)
    }
}
