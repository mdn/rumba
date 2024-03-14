use config::{Config, ConfigError, Environment, File};

use harsh::Harsh;
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_with::{base64::Base64, serde_as};
use std::env;
use url::Url;

#[derive(Deserialize)]
pub struct DB {
    pub uri: String,
    pub supabase_uri: Option<String>,
}

#[derive(Deserialize)]
pub struct Server {
    pub host: String,
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
    pub login_cookie_name: String,
    pub auth_cookie_secure: bool,
    #[serde_as(as = "Base64")]
    pub cookie_key: [u8; 64],
    pub admin_update_bearer_token: String,
}

#[derive(Deserialize)]
pub struct Application {
    pub document_base_url: String,
    pub bcd_updates_url: Url,
    pub mdn_metadata_url: Url,
    pub subscriptions_limit_collections: i64,
    pub encoded_id_salt: String,
}

#[derive(Deserialize)]
pub struct Search {
    pub url: String,
    pub cache_max_age: u32,
    pub query_max_length: usize,
}

#[derive(Deserialize, Default)]
pub struct Logging {
    pub human_logs: bool,
}

#[derive(Deserialize, Default)]
pub struct Metrics {
    pub statsd_label: String,
    pub statsd_host: Option<String>,
    pub statsd_port: u16,
}

#[derive(Deserialize, Default)]
pub struct Sentry {
    pub dsn: String,
}

#[derive(Debug, Deserialize)]
pub struct Basket {
    pub api_key: String,
    pub basket_url: Url,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct AI {
    pub api_key: String,
    pub gemini_api_key: Option<String>,
    pub gemini_model: Option<String>,
    pub limit_reset_duration_in_sec: i64,
    #[serde_as(as = "Base64")]
    pub explain_sign_key: [u8; 32],
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Playground {
    pub github_token: String,
    #[serde_as(as = "Base64")]
    pub crypt_key: [u8; 32],
    pub flag_repo: String,
}

#[derive(Deserialize)]
pub struct Settings {
    pub db: DB,
    pub server: Server,
    pub auth: Auth,
    pub application: Application,
    pub search: Search,
    pub logging: Logging,
    pub metrics: Metrics,
    pub sentry: Option<Sentry>,
    pub basket: Option<Basket>,
    pub ai: Option<AI>,
    pub playground: Option<Playground>,
    #[serde(default)]
    pub skip_migrations: bool,
    pub maintenance: Option<String>,
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
        Err(err) => panic!("{:?}", err),
    }
});

pub static HARSH: Lazy<Harsh> = Lazy::new(|| {
    let harsh = Harsh::builder()
        .salt(SETTINGS.application.encoded_id_salt.clone())
        .length(4)
        .build();
    match harsh {
        Ok(harsh) => harsh,
        Err(err) => {
            panic!("{:?}", err);
        }
    }
});
