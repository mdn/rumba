use crate::db::types::{FxaEventStatus, Subscription};
use crate::db::{schema::*, types::FxaEvent};
use crate::experiments::ExperimentsConfig;
use crate::helpers::to_utc;
use chrono::NaiveDateTime;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::types::Locale;

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = users)]
pub struct User {
    pub updated_at: NaiveDateTime,
    pub email: String,
    pub fxa_uid: String,
    pub fxa_refresh_token: String,
    pub avatar_url: Option<Option<String>>,
    pub subscription_type: Subscription,
    pub enforce_plus: Option<Subscription>,
    pub is_admin: Option<bool>,
    pub is_mdn_team: Option<bool>,
    pub is_fox_food: Option<bool>,
}

impl User {
    pub fn get_subscription_type(&self) -> Subscription {
        self.enforce_plus.unwrap_or(self.subscription_type)
    }
}

#[derive(Queryable, Debug, Serialize)]
#[diesel(table_name = users)]
pub struct UserQuery {
    pub id: i64,
    #[serde(serialize_with = "to_utc")]
    pub created_at: NaiveDateTime,
    #[serde(serialize_with = "to_utc")]
    pub updated_at: NaiveDateTime,
    pub email: String,
    pub fxa_uid: String,
    pub fxa_refresh_token: String,
    pub avatar_url: Option<String>,
    subscription_type: Option<Subscription>,
    pub enforce_plus: Option<Subscription>,
    pub is_admin: bool,
    pub is_mdn_team: bool,
    pub is_fox_food: bool,
}

impl UserQuery {
    pub fn get_subscription_type(&self) -> Option<Subscription> {
        self.enforce_plus.or(self.subscription_type)
    }

    pub fn is_subscriber(&self) -> bool {
        self.get_subscription_type()
            .unwrap_or_default()
            .is_subscriber()
    }

    pub fn eligible_for_experiments(&self) -> bool {
        self.is_admin || self.is_mdn_team
    }

    #[cfg(test)]
    pub fn dummy() -> Self {
        UserQuery {
            id: 0,
            created_at: NaiveDateTime::MIN,
            updated_at: NaiveDateTime::MIN,
            email: "foo@bar.com".to_string(),
            fxa_uid: Uuid::nil().to_string(),
            fxa_refresh_token: Default::default(),
            avatar_url: None,
            subscription_type: None,
            enforce_plus: None,
            is_admin: false,
            is_mdn_team: false,
            is_fox_food: false,
        }
    }
}

#[derive(Queryable, Clone)]
pub struct CollectionAndDocumentQuery {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub document_id: i64,
    pub notes: Option<String>,
    pub custom_name: Option<String>,
    pub user_id: i64,
    pub uri: String,
    pub metadata: Option<Value>,
    pub title: String,
}

#[derive(Serialize, Deserialize)]
pub struct CollectionParent {
    pub uri: String,
    pub title: String,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = documents)]
pub struct DocumentInsert {
    pub absolute_uri: String,
    pub uri: String,
    pub metadata: Option<Value>,
    pub updated_at: NaiveDateTime,
    pub title: String,
    pub paths: Vec<String>,
}

#[derive(Queryable, Debug)]
#[diesel(table_name = settings)]
pub struct Settings {
    pub id: i64,
    pub user_id: i64,
    pub locale_override: Option<Locale>,
    pub mdnplus_newsletter: bool,
    pub no_ads: bool,
}

#[derive(Insertable, AsChangeset, Default)]
#[diesel(table_name = settings)]
pub struct SettingsInsert {
    pub user_id: i64,
    pub locale_override: Option<Option<Locale>>,
    pub mdnplus_newsletter: Option<bool>,
    pub no_ads: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub mdn_url: String,
    pub parents: Option<Vec<CollectionParent>>,
    pub title: String,
    pub paths: Vec<String>,
}

#[allow(dead_code)]
#[derive(Queryable)]
pub struct IdQuery {
    id: i64,
}

#[derive(Queryable)]
pub struct DocumentQuery {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub absolute_uri: String,
    pub uri: String,
    pub metadata: Option<Value>,
    pub title: String,
    pub paths: Vec<Option<String>>,
}

#[derive(Insertable)]
#[diesel(table_name = webhook_events)]
pub struct WebHookEventInsert {
    pub fxa_uid: String,
    pub change_time: Option<NaiveDateTime>,
    pub issue_time: NaiveDateTime,
    pub typ: FxaEvent,
    pub status: FxaEventStatus,
    pub payload: Value,
}

#[derive(Queryable, AsChangeset, Debug)]
#[diesel(table_name = webhook_events)]
pub struct WebHookEventQuery {
    pub id: i64,
    pub fxa_uid: String,
    pub change_time: Option<NaiveDateTime>,
    pub issue_time: NaiveDateTime,
    pub typ: FxaEvent,
    pub status: FxaEventStatus,
    pub payload: Value,
}

#[derive(Insertable)]
#[diesel(table_name = raw_webhook_events_tokens)]
pub struct RawWebHookEventsTokenInsert {
    pub token: String,
    pub error: String,
}

#[derive(Insertable, Serialize)]
#[diesel(table_name = activity_pings)]
pub struct ActivityPingInsert {
    pub user_id: i64,
    pub activity: Value,
}

#[derive(Insertable, Serialize, Debug, Default)]
#[diesel(table_name = playground)]
pub struct PlaygroundInsert {
    pub user_id: Option<i64>,
    pub gist: String,
    pub active: bool,
    pub flagged: bool,
}

#[derive(Queryable, Serialize, Debug, Default)]
#[diesel(table_name = playground)]
pub struct PlaygroundQuery {
    pub id: i64,
    pub user_id: Option<i64>,
    pub gist: String,
    pub active: bool,
    pub flagged: bool,
    pub deleted_user_id: Option<i64>,
}

#[derive(Insertable, Serialize, Debug, Default)]
#[diesel(table_name = ai_help_limits)]
pub struct AIHelpLimitInsert {
    pub user_id: i64,
    pub latest_start: NaiveDateTime,
    pub session_questions: i64,
    pub total_questions: i64,
}

#[derive(Insertable, Serialize, Debug, Default)]
#[diesel(table_name = ai_explain_cache)]
pub struct AIExplainCacheInsert {
    pub language: Option<String>,
    pub highlighted_hash: Vec<u8>,
    pub signature: Vec<u8>,
    pub explanation: Option<String>,
    pub version: i64,
}

#[derive(Queryable, Serialize, Debug, Default)]
#[diesel(table_name = ai_explain_cache)]
pub struct AIExplainCacheQuery {
    pub id: i64,
    pub signature: Vec<u8>,
    pub highlighted_hash: Vec<u8>,
    pub language: Option<String>,
    pub explanation: Option<String>,
    pub created_at: NaiveDateTime,
    pub last_used: NaiveDateTime,
    pub view_count: i64,
    pub version: i64,
    pub thumbs_up: i64,
    pub thumbs_down: i64,
}

#[derive(Insertable, AsChangeset, Debug)]
#[diesel(table_name = experiments)]
pub struct ExperimentsInsert {
    pub user_id: i64,
    pub active: Option<bool>,
    pub config: ExperimentsConfig,
}

#[derive(Queryable, Serialize, Debug, Default)]
#[diesel(table_name = experiments)]
pub struct ExperimentsQuery {
    pub id: i64,
    pub user_id: i64,
    pub active: bool,
    pub config: ExperimentsConfig,
}

#[derive(Insertable, Serialize, Debug, Default)]
#[diesel(table_name = ai_help_history)]
pub struct AIHelpHistoryInsert {
    pub user_id: i64,
    pub chat_id: Uuid,
    pub message_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub created_at: Option<NaiveDateTime>,
    pub sources: Value,
    pub request: Value,
    pub response: Value,
}

#[derive(Queryable, Serialize, Debug, Default)]
#[diesel(table_name = ai_help_history)]
pub struct AIHelpHistory {
    pub id: i64,
    pub user_id: i64,
    pub chat_id: Uuid,
    pub message_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub sources: Value,
    pub request: Value,
    pub response: Value,
}

#[derive(Insertable, Serialize, Debug, Default)]
#[diesel(table_name = ai_help_debug_logs)]
pub struct AIHelpDebugLogsInsert {
    pub user_id: i64,
    pub variant: String,
    pub chat_id: Uuid,
    pub message_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub created_at: Option<NaiveDateTime>,
    pub sources: Value,
    pub request: Value,
    pub response: Value,
}

#[derive(Queryable, Serialize, Debug, Default)]
#[diesel(table_name = ai_help_debug_logs)]
pub struct AIHelpDebugLogs {
    pub id: i64,
    pub user_id: i64,
    pub variant: String,
    pub chat_id: Uuid,
    pub message_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub sources: Value,
    pub request: Value,
    pub response: Value,
    pub feedback: Option<String>,
    pub thumbs: Option<bool>,
}

#[derive(Insertable, AsChangeset, Serialize, Debug, Default)]
#[diesel(table_name = ai_help_debug_logs)]
pub struct AIHelpDebugLogsFeedbackInsert {
    pub feedback: Option<Option<String>>,
    pub thumbs: Option<Option<bool>>,
}

#[derive(Insertable, AsChangeset, Serialize, Debug, Default)]
#[diesel(table_name = ai_help_feedback)]
pub struct AIHelpFeedbackInsert {
    pub message_id: Uuid,
    pub feedback: Option<Option<String>>,
    pub thumbs: Option<Option<bool>>,
}
