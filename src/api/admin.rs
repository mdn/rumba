use crate::db::Pool;
use crate::settings::SETTINGS;
use crate::util::BROWSERS;
use actix_web::dev::{HttpServiceFactory, ServiceRequest};
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{dev::Service as _, web, Error, HttpRequest, HttpResponse};
use actix_web_httpauth::extractors::basic::BasicAuth;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::extractors::AuthenticationError;
use actix_web_httpauth::middleware::HttpAuthentication;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

use super::error::ApiError;

#[derive(Deserialize)]
pub struct UpdateNotificationsRequest {
    pub filename: String,
}

#[derive(Deserialize, Debug, Clone)]
pub enum Browser {
    #[serde(rename = "chrome")]
    Chrome,
    #[serde(rename = "chrome_android")]
    ChromeAndroid,
    #[serde(rename = "deno")]
    Deno,
    #[serde(rename = "edge")]
    Edge,
    #[serde(rename = "firefox")]
    Firefox,
    #[serde(rename = "firefox_android")]
    FirefoxAndroid,
    #[serde(rename = "ie")]
    InternetExplorer,
    #[serde(rename = "nodejs")]
    NodeJs,
    #[serde(rename = "opera")]
    Opera,
    #[serde(rename = "opera_android")]
    OperaAndroid,
    #[serde(rename = "safari")]
    Safari,
    #[serde(rename = "safari_ios")]
    SafariIos,
    #[serde(rename = "samsunginternet_android")]
    SamsungInternetAndroid,
    #[serde(rename = "webview_android")]
    WebviewAnroid,
}

impl Browser {
    pub fn display_name(&self) -> &str {
        match *self {
            Browser::Chrome => "Chrome",
            Browser::ChromeAndroid => "Chrome Android",
            Browser::Deno => "Deno",
            Browser::Edge => "Edge",
            Browser::Firefox => "Firefox",
            Browser::FirefoxAndroid => "Firefox for Android",
            Browser::InternetExplorer => "Internet Explorer",
            Browser::NodeJs => "Node.js",
            Browser::Opera => "Opera",
            Browser::OperaAndroid => "Opera Android",
            Browser::Safari => "Safari",
            Browser::SafariIos => "Safari on iOS",
            Browser::SamsungInternetAndroid => "Samsung Internet",
            Browser::WebviewAnroid => "WebView Android",
        }
    }
    pub fn preview_name(&self) -> &str {
        match *self {
            Browser::Chrome => "Canary",
            Browser::ChromeAndroid => "Chrome Android",
            Browser::Deno => "Deno",
            Browser::Edge => "Edge",
            Browser::Firefox => "Nightly",
            Browser::FirefoxAndroid => "Firefox for Android",
            Browser::InternetExplorer => "Internet Explorer",
            Browser::NodeJs => "Node.js",
            Browser::Opera => "Opera",
            Browser::OperaAndroid => "Opera Android",
            Browser::Safari => "TP",
            Browser::SafariIos => "Safari on iOS",
            Browser::SamsungInternetAndroid => "Samsung Internet",
            Browser::WebviewAnroid => "WebView Android",
        }
    }

    pub fn browser_group(&self) -> &str {
        match *self {
            Browser::Chrome => "chrome",
            Browser::ChromeAndroid => "chrome",
            Browser::Deno => "deno",
            Browser::Edge => "chrome",
            Browser::Firefox => "firefox",
            Browser::FirefoxAndroid => "firefox",
            Browser::InternetExplorer => "ie",
            Browser::NodeJs => "nodejs",
            Browser::Opera => "opera",
            Browser::OperaAndroid => "opera",
            Browser::Safari => "safari",
            Browser::SafariIos => "safari",
            Browser::SamsungInternetAndroid => "samsunginternet_android",
            Browser::WebviewAnroid => "chrome",
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "event")]
pub enum DocumentChangeEvent {
    #[serde(rename = "added_stable")]
    AddedStable(AddRemoveStableEvent),
    #[serde(rename = "added_preview")]
    AddedPreview(AddedPreviewEvent),
    #[serde(rename = "added_subfeatures")]
    AddedSubFeatures(AddedSubFeaturesEvent),
    #[serde(rename = "added_nonnull")]
    AddedNonNull(AddedNonNullEvent),
    #[serde(rename = "content_updated")]
    ContentUpdated,
    #[serde(rename = "removed_stable")]
    RemovedStable(AddRemoveStableEvent),
    #[serde(other)]
    Unknown,
}

impl DocumentChangeEvent {
    pub fn get_event_copy(&self) -> &str {
        match *self {
            DocumentChangeEvent::AddedStable(_) => "Supported in",
            DocumentChangeEvent::AddedPreview(_) => "In development in",
            DocumentChangeEvent::AddedSubFeatures(_) => "",
            DocumentChangeEvent::AddedNonNull(_) => "",
            DocumentChangeEvent::ContentUpdated => "",
            DocumentChangeEvent::RemovedStable(_) => "Removed from",
            DocumentChangeEvent::Unknown => "",
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct AddedNonNullEvent {
    path: String,
    support_changes: Vec<SupportChange>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SupportChange {
    browser: String,
    support: Vec<Support>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Support {
    version_added: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AddedSubFeaturesEvent {
    path: String,
    subfeatures: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AddedPreviewEvent {
    path: String,
    description: Option<String>,
    browsers: Vec<BrowserItem>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AddRemoveStableEvent {
    path: String,
    description: Option<String>,
    browsers: Vec<BrowserItem>,
    mdn_url: Option<String>,
    spec_url: Option<Vec<String>>,
    status: Option<FeatureStatus>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FeatureStatus {
    experimental: bool,
    standard_track: bool,
    deprecated: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BrowserItem {
    browser: Browser,
    version: String,
    notes: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Changes {
    changes: Vec<DocumentChangeEvent>,
}

pub async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, Error> {
    if credentials.token() == "TEST_TOKEN" {
        Ok(req)
    } else {
        Err(Error::from(ApiError::InvalidBearer))
    }
}
#[derive(Debug)]
pub struct BcdNotification {
    path: String,
    text: String,
    data: DocumentChangeEvent,
}

pub async fn process_notification_update(
    req: HttpRequest,
    pool: web::Data<Pool>,
    client: web::Data<Client>,
    body: web::Json<UpdateNotificationsRequest>,
) -> Result<HttpResponse, ApiError> {
    let changes_json = get_update_json(client, &body).await?;
    let mut bcd_notifications: Vec<BcdNotification> = vec![];
    changes_json.iter().for_each(|event| {
        match (event) {
            DocumentChangeEvent::RemovedStable(val) | DocumentChangeEvent::AddedStable(val) => {
                let mut browsers_grouped_by_type: HashMap<&str, Vec<String>> = HashMap::new();
                group_by_browsers(&val.browsers, &mut browsers_grouped_by_type, false);
                let mut notifications = generate_bcd_notifications(event, &val.path, browsers_grouped_by_type);
                bcd_notifications.append(&mut notifications);
            }
            DocumentChangeEvent::AddedPreview(val) => {
                let mut browsers_grouped_by_type: HashMap<&str, Vec<String>> = HashMap::new();
                group_by_browsers(&val.browsers, &mut browsers_grouped_by_type, true);
                //Create one BCD update per browser group
                let mut new_notifications = &mut generate_bcd_notifications(event, &val.path, browsers_grouped_by_type);
                bcd_notifications.append(new_notifications);
            }
            DocumentChangeEvent::AddedSubFeatures(_) => {}
            DocumentChangeEvent::AddedNonNull(_) => {}
            DocumentChangeEvent::ContentUpdated => {}
            (_) => {}
            DocumentChangeEvent::Unknown => {}
        }
    });
    println!("{:?}", bcd_notifications);
    println!("{:?}", changes_json);
    Ok(HttpResponse::Ok().finish())
}

fn generate_bcd_notifications(event: &DocumentChangeEvent, path: &String, mut browsers_grouped_by_type: HashMap<&str, Vec<String>>) -> Vec<BcdNotification> {
    let mut bcd_notifications = vec!();
    //Create one BCD update per browser group
    for (_, browser_strings) in browsers_grouped_by_type.iter_mut() {
        let browser_string = get_pluralized_string(browser_strings);
        let update_string: String =
            format!("{} {}", event.get_event_copy(), browser_string);
        bcd_notifications.push(BcdNotification {
            path: path.to_owned(),
            text: update_string.to_owned(),
            data: event.to_owned(),
        });
    }
    bcd_notifications
}

fn get_pluralized_string(browser_strings: &mut Vec<String>) -> String {
    let browser_string = if browser_strings.len() > 1 {
        let last = browser_strings.pop().unwrap();
        format!("{} and {}", browser_strings.join(", "), last)
    } else {
        browser_strings[0].to_string()
    };
    browser_string
}

fn group_by_browsers<'a>(
    val: &'a Vec<BrowserItem>,
    mut browser_groups: &mut HashMap<&'a str, Vec<String>>,
    is_preview_feature: bool,
) {
    val.iter().for_each(|item| {
        //Normalize browser name
        let browser_name = if is_preview_feature {
            item.browser.display_name()
        } else {
            item.browser.preview_name()
        };
        let update_string = format!("{} {}", browser_name, item.version);
        // Group by 'browser group' and update string.
        if let Some(exists) = browser_groups.get_mut(item.browser.browser_group()) {
            exists.push(update_string);
        } else {
            browser_groups.insert(item.browser.browser_group(), vec![update_string]);
        }
    });
}

async fn get_update_json(
    client: Data<Client>,
    body: &Json<UpdateNotificationsRequest>,
) -> Result<Vec<DocumentChangeEvent>, ApiError> {
    let update_url = Url::parse(&format!(
        "{}/{}",
        SETTINGS.application.notifications_update_base_url, body.filename
    ))
    .map_err(|_| ApiError::MalformedUrl)?;

    let res = client
        .get(update_url)
        .send()
        .await
        .map_err(|err: reqwest::Error| match err.status() {
            Some(StatusCode::NOT_FOUND) => ApiError::DocumentNotFound,
            _ => ApiError::Unknown,
        })?
        .json()
        .await
        .map_err(|err| {
            println!("{:1}", err);
            ApiError::DocumentNotFound
        })?;
    Ok(res)
}

pub fn admin_service() -> impl HttpServiceFactory {
    web::scope("/admin-api")
        .wrap(HttpAuthentication::bearer(validator))
        .service(web::resource("/update/").route(web::post().to(process_notification_update)))
}
