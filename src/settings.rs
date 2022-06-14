use config::{Config, ConfigError, Environment, File};

use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_with::{base64::Base64, serde_as};
use std::env;
use url::Url;

#[derive(Deserialize)]
pub struct DB {
    pub uri: String,
}

#[derive(Deserialize)]
pub struct Server {
    pub port: u16,
}

#[serde_as]
#[derive(Deserialize)]
pub struct Auth {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: String,
    pub redirect_url: Url,
    pub auth_cookie_name: String,
    pub auth_cookie_secure: bool,
    #[serde_as(as = "Base64")]
    pub auth_cookie_key: [u8; 64],
}

#[derive(Deserialize)]
pub struct Application {
    pub document_base_url: String,
    pub notifications_update_base_url: String,
    pub subscriptions_limit_watched_items: i64,
    pub subscriptions_limit_collections: i64,
}

#[derive(Deserialize)]
pub struct Settings {
    pub db: DB,
    pub server: Server,
    pub auth: Auth,
    pub application: Application,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let file = env::var("MDN_SETTINGS").unwrap_or_else(|_| String::from(".settings.toml"));
        let s = Config::builder()
            .add_source(File::with_name(&file))
            .add_source(Environment::with_prefix("mdn").separator("__"));
        s.build()?.try_deserialize()
    }
}

pub static SETTINGS: Lazy<Settings> = Lazy::new(|| {
    let settings = Settings::new();
    match settings {
        Ok(settings) => settings,
        Err(err) => {
            panic!("{:?}", err);
        }
    }
});
