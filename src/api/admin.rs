use crate::db;
use crate::db::model::NotificationDataInsert;
use crate::db::v2::synchronize_bcd_updates_db::update_bcd;
use crate::db::Pool;
use crate::settings::SETTINGS;
use crate::util::normalize_uri;
use actix_web::dev::{HttpServiceFactory, ServiceRequest};
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use db::notifications::{create_notification_data, create_notifications_for_users};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use url::Url;

use super::error::ApiError;

#[derive(Deserialize)]
pub struct UpdateNotificationsRequest {
    pub filename: String,
}

#[derive(Deserialize, PartialEq, Eq, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Browser {
    Chrome,
    ChromeAndroid,
    Deno,
    Edge,
    Firefox,
    FirefoxAndroid,
    #[serde(rename = "ie")]
    InternetExplorer,
    #[serde(rename = "nodejs")]
    NodeJs,
    Opera,
    OperaAndroid,
    Safari,
    SafariIos,
    SamsungInternetAndroid,
    WebviewAndroid,
    #[serde(other)]
    Unknown,
}

#[derive(Eq, Hash, PartialEq)]
pub enum BrowserGroup {
    Chrome,
    Deno,
    Firefox,
    InternetExplorer,
    NodeJs,
    Opera,
    Safari,
    SamsungInternetAndroid,
    Unknown,
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
            Browser::WebviewAndroid => "WebView Android",
            Browser::Unknown => "Unknown",
        }
    }
    pub fn preview_name(&self) -> &str {
        match *self {
            Browser::Chrome => "Canary",
            Browser::Firefox => "Nightly",
            Browser::Safari => "TP",
            _ => self.display_name(),
        }
    }

    pub fn browser_group(&self) -> BrowserGroup {
        match *self {
            Browser::Chrome => BrowserGroup::Chrome,
            Browser::ChromeAndroid => BrowserGroup::Chrome,
            Browser::Deno => BrowserGroup::Deno,
            Browser::Edge => BrowserGroup::Chrome,
            Browser::Firefox => BrowserGroup::Firefox,
            Browser::FirefoxAndroid => BrowserGroup::Firefox,
            Browser::InternetExplorer => BrowserGroup::InternetExplorer,
            Browser::NodeJs => BrowserGroup::NodeJs,
            Browser::Opera => BrowserGroup::Opera,
            Browser::OperaAndroid => BrowserGroup::Opera,
            Browser::Safari => BrowserGroup::Safari,
            Browser::SafariIos => BrowserGroup::Safari,
            Browser::SamsungInternetAndroid => BrowserGroup::SamsungInternetAndroid,
            Browser::WebviewAndroid => BrowserGroup::Chrome,
            Browser::Unknown => BrowserGroup::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    ContentUpdated(ContentUpdatedEvent),
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
            DocumentChangeEvent::AddedNonNull(_) => "More complete compatibility data added for",
            DocumentChangeEvent::ContentUpdated(_) => "",
            DocumentChangeEvent::RemovedStable(_) => "Removed from",
            DocumentChangeEvent::Unknown => "",
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AddedNonNullEvent {
    pub path: String,
    pub support_changes: Vec<SupportChange>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ContentUpdatedEvent {
    pub page_url: String,
    pub pr_url: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SupportChange {
    pub browser: Browser,
    pub support: Vec<Support>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Support {
    version_added: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AddedSubFeaturesEvent {
    path: String,
    subfeatures: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AddedPreviewEvent {
    path: String,
    description: Option<String>,
    browsers: Vec<BrowserItem>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AddRemoveStableEvent {
    path: String,
    description: Option<String>,
    browsers: Vec<BrowserItem>,
    mdn_url: Option<String>,
    spec_url: Option<Vec<String>>,
    status: Option<FeatureStatus>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FeatureStatus {
    experimental: bool,
    standard_track: bool,
    deprecated: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BrowserItem {
    browser: Browser,
    version: String,
    notes: Option<Vec<String>>,
}

pub async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    if credentials.token() == SETTINGS.auth.admin_update_bearer_token {
        Ok(req)
    } else {
        Err((Error::from(ApiError::InvalidBearer), req))
    }
}
#[derive(Debug)]
pub struct BcdNotification<'a> {
    path: &'a str,
    text: String,
    data: &'a DocumentChangeEvent,
}

#[derive(Debug)]
pub struct ContentNotification<'a> {
    pub url: String,
    pub text: String,
    pub data: &'a DocumentChangeEvent,
}

static GITHUB_PR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https://github.com/(.+)/pull/(\d+)$").expect("invalid github regex")
});

pub async fn process_notification_update(
    _req: HttpRequest,
    pool: web::Data<Pool>,
    client: web::Data<Client>,
    body: web::Json<UpdateNotificationsRequest>,
) -> Result<HttpResponse, ApiError> {
    let changes_json = get_update_json(client, &body).await?;
    let mut bcd_notifications: Vec<BcdNotification> = vec![];
    let mut content_notifications: Vec<ContentNotification> = vec![];

    changes_json.iter().for_each(|event| {
        match event {
            DocumentChangeEvent::RemovedStable(change)
            | DocumentChangeEvent::AddedStable(change) => {
                let browsers_grouped_by_type = group_by_browsers(&change.browsers, false);
                if browsers_grouped_by_type.is_empty() {
                    return;
                }
                let mut notifications = generate_bcd_notifications_for_event(
                    event,
                    &change.path,
                    browsers_grouped_by_type,
                );
                bcd_notifications.append(&mut notifications);
            }
            DocumentChangeEvent::AddedPreview(change) => {
                let browsers_grouped_by_type = group_by_browsers(&change.browsers, true);
                if browsers_grouped_by_type.is_empty() {
                    return;
                }
                //Create one BCD update per browser group
                let mut new_notifications = generate_bcd_notifications_for_event(
                    event,
                    &change.path,
                    browsers_grouped_by_type,
                );
                bcd_notifications.append(&mut new_notifications);
            }
            DocumentChangeEvent::AddedSubFeatures(change) => {
                let plural = if change.subfeatures.len() > 1 {
                    "'s"
                } else {
                    ""
                };
                bcd_notifications.push(BcdNotification {
                    path: &change.path,
                    text: format!("Compatibility subfeature{} added", plural),
                    data: event,
                })
            }
            DocumentChangeEvent::AddedNonNull(change) => {
                let browsers_by_name: Vec<String> = change
                    .support_changes
                    .iter()
                    .filter(|val| val.browser != Browser::Unknown)
                    .map(|val| val.browser.display_name().to_owned())
                    .collect();
                if browsers_by_name.is_empty() {
                    return;
                }
                let text = get_pluralized_string(&browsers_by_name);
                let non_null_notification = BcdNotification {
                    path: change.path.as_str(),
                    text,
                    data: event,
                };
                bcd_notifications.push(non_null_notification);
            }
            DocumentChangeEvent::ContentUpdated(change) => {
                let regex = GITHUB_PR_RE.captures(change.pr_url.as_str());
                let text = match regex {
                    Some(capture_groups) if capture_groups.len() > 1 => format!(
                        "Page updated (see PR!{}!{}!!)",
                        &capture_groups[0], &capture_groups[1]
                    ),
                    _ => "Page updated".to_string(),
                };
                content_notifications.push(ContentNotification {
                    url: normalize_uri(change.page_url.as_str()),
                    text,
                    data: event,
                });
            }
            DocumentChangeEvent::Unknown => {
                //Probably log an error here
                error!("Unknown event type received {:?}", event)
            }
        }
    });
    let mut conn_pool = pool.get()?;
    for notification in bcd_notifications.iter() {
        let mut parts: Vec<&str> = notification.path.split('.').collect();
        let mut suffix: Vec<&str> = Vec::with_capacity(parts.len());

        while !parts.is_empty() {
            let subpath = parts.join(".");
            let doc = db::documents::get_document_by_path(&mut conn_pool, subpath);
            suffix.push(parts.pop().unwrap());

            if let Ok(document) = doc {
                suffix.reverse();
                let title = suffix.join(".");
                let notification_data_id = create_notification_data(
                    &mut conn_pool,
                    NotificationDataInsert {
                        text: notification.text.to_owned(),
                        url: document.uri,
                        data: serde_json::to_value(notification.data).ok(),
                        title,
                        type_: db::types::NotificationTypeEnum::Compat,
                        document_id: document.id,
                    },
                )?;
                create_notifications_for_users(&mut conn_pool, document.id, notification_data_id)?;
            } else {
                continue;
            }
        }
    }

    for notification in content_notifications.iter() {
        let doc = db::documents::get_document_by_url(&mut conn_pool, notification.url.as_str());

        if let Ok(document) = doc {
            let notification_data_id = create_notification_data(
                &mut conn_pool,
                NotificationDataInsert {
                    text: notification.text.to_owned(),
                    url: document.uri,
                    data: Some(json!(notification.data)),
                    title: document.title,
                    type_: db::types::NotificationTypeEnum::Content,
                    document_id: document.id,
                },
            )?;
            create_notifications_for_users(&mut conn_pool, document.id, notification_data_id)?;
        } else {
            continue;
        }
    }

    Ok(HttpResponse::Ok().finish())
}

fn generate_bcd_notifications_for_event<'a>(
    event: &'a DocumentChangeEvent,
    path: &'a str,
    mut browsers_grouped_by_type: HashMap<BrowserGroup, Vec<String>>,
) -> Vec<BcdNotification<'a>> {
    let mut bcd_notifications = vec![];
    //Create one BCD update per browser group
    for (_, browser_strings) in browsers_grouped_by_type.iter_mut() {
        let browser_string = get_pluralized_string(browser_strings);
        let update_string: String = format!("{} {}", event.get_event_copy(), browser_string);
        bcd_notifications.push(BcdNotification {
            path,
            text: update_string,
            data: event,
        });
    }
    bcd_notifications
}

fn get_pluralized_string(browser_strings: &[String]) -> String {
    match &browser_strings {
        [] => "None".to_string(),
        [browser_string] => browser_string.to_owned(),
        [list @ .., last] => format!("{} and {}", list.join(", "), last),
    }
}

fn group_by_browsers(
    val: &[BrowserItem],
    is_preview_feature: bool,
) -> HashMap<BrowserGroup, Vec<String>> {
    let mut map: HashMap<BrowserGroup, Vec<String>> = HashMap::new();
    val.iter()
        .filter(|b| b.browser != Browser::Unknown)
        .for_each(|item| {
            //Normalize browser name
            let browser_name = if is_preview_feature {
                item.browser.preview_name()
            } else {
                item.browser.display_name()
            };
            let update_string = format!("{} {}", browser_name, item.version);
            // Group by 'browser group' and update string.
            if let Some(exists) = map.get_mut(&item.browser.browser_group()) {
                exists.push(update_string);
            } else {
                map.insert(item.browser.browser_group(), vec![update_string]);
            }
        });
    map
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
            error!("{:1}", err);
            ApiError::DocumentNotFound
        })?;
    Ok(res)
}

pub fn admin_service() -> impl HttpServiceFactory {
    web::scope("/admin-api")
        .wrap(HttpAuthentication::bearer(validator))
        .service(web::resource("/update/").route(web::post().to(process_notification_update)))
        .service(web::resource("/v2/updates/").route(web::post().to(update_bcd)))
}
