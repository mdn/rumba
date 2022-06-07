#![allow(non_camel_case_types)]
use crate::db::schema;
use schema::sql_types::NotificationType;
use schema::sql_types::{FxaEventStatusType, FxaEventType, Locale as DbLocale, SubscriptionType};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, diesel_derive_enum::DbEnum, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[DieselExistingType = "DbLocale"]
pub enum Locale {
    #[serde(rename = "de")]
    De,
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "es")]
    Es,
    #[serde(rename = "fr")]
    Fr,
    #[serde(rename = "ja")]
    Ja,
    #[serde(rename = "ko")]
    Ko,
    #[serde(rename = "pl")]
    Pl,
    #[serde(rename = "pt-BR")]
    PtBr,
    #[serde(rename = "ru")]
    Ru,
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "zh-TW")]
    ZhTw,
}

#[derive(
    Copy,
    Clone,
    diesel_derive_enum::DbEnum,
    Debug,
    Deserialize,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
#[DieselExistingType = "FxaEventStatusType"]
pub enum FxaEventStatus {
    #[serde(rename = "processed")]
    Processed,
    #[serde(rename = "ignored")]
    Ignored,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(
    Copy,
    Clone,
    diesel_derive_enum::DbEnum,
    Debug,
    Deserialize,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
#[DieselExistingType = "FxaEventType"]
pub enum FxaEvent {
    #[serde(rename = "delete_user")]
    DeleteUser,
    #[serde(rename = "password_change")]
    PasswordChange,
    #[serde(rename = "profile_change")]
    ProfileChange,
    #[serde(rename = "subscription_state_change")]
    SubscriptionStateChange,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(
    Copy,
    Clone,
    diesel_derive_enum::DbEnum,
    Debug,
    Deserialize,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
#[DieselExistingType = "SubscriptionType"]
pub enum Subscription {
    #[serde(rename(serialize = "core"))]
    Core,
    #[serde(rename = "mdn_plus_5m")]
    MdnPlus_5m,
    #[serde(rename = "mdn_plus_10m")]
    MdnPlus_10m,
    #[serde(rename = "mdn_plus_5y")]
    MdnPlus_5y,
    #[serde(rename = "mdn_plus_10y")]
    MdnPlus_10y,
}

impl Subscription {
    pub fn is_subscriber(&self) -> bool {
        match self {
            Subscription::Core => false,
            Subscription::MdnPlus_5m
            | Subscription::MdnPlus_10m
            | Subscription::MdnPlus_5y
            | Subscription::MdnPlus_10y => true,
        }
    }
}

impl Default for Subscription {
    fn default() -> Self {
        Self::Core
    }
}

impl From<String> for Subscription {
    fn from(s: String) -> Self {
        match s.as_str() {
            "mdn_plus_5m" => Subscription::MdnPlus_5m,
            "mdn_plus_5y" => Subscription::MdnPlus_5y,
            "mdn_plus_10y" => Subscription::MdnPlus_10y,
            "mdn_plus_10m" => Subscription::MdnPlus_10m,
            _ => Subscription::Core,
        }
    }
}

impl From<Subscription> for String {
    fn from(val: Subscription) -> Self {
        match val {
            Subscription::MdnPlus_5m => "mdn_plus_5m",
            Subscription::MdnPlus_5y => "mdn_plus_5y",
            Subscription::MdnPlus_10y => "mdn_plus_10y",
            Subscription::MdnPlus_10m => "mdn_plus_10m",
            Subscription::Core => "core",
        }
        .into()
    }
}

#[derive(
    Copy,
    Clone,
    diesel_derive_enum::DbEnum,
    Debug,
    Deserialize,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
#[DieselExistingType = "NotificationType"]
pub enum NotificationTypeEnum {
    #[serde(rename(serialize = "content"))]
    Content,
    #[serde(rename(serialize = "compat"))]
    Compat,
}
