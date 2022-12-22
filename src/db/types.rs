#![allow(non_camel_case_types)]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, diesel_derive_enum::DbEnum, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[DieselTypePath = "crate::db::schema::sql_types::Locale"]
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
#[DieselTypePath = "crate::db::schema::sql_types::FxaEventStatusType"]
#[serde(rename_all = "snake_case")]
pub enum FxaEventStatus {
    Processed,
    Ignored,
    Pending,
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
#[DieselTypePath = "crate::db::schema::sql_types::FxaEventType"]
#[serde(rename_all = "snake_case")]
pub enum FxaEvent {
    DeleteUser,
    PasswordChange,
    ProfileChange,
    SubscriptionStateChange,
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
#[DieselTypePath = "crate::db::schema::sql_types::SubscriptionType"]
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

    pub fn as_str(&self) -> &str {
        match self {
            Subscription::MdnPlus_5m => "mdn_plus_5m",
            Subscription::MdnPlus_5y => "mdn_plus_5y",
            Subscription::MdnPlus_10y => "mdn_plus_10y",
            Subscription::MdnPlus_10m => "mdn_plus_10m",
            Subscription::Core => "core",
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
        val.as_str().into()
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
#[DieselTypePath = "crate::db::schema::sql_types::NotificationType"]
pub enum NotificationTypeEnum {
    #[serde(rename(serialize = "content"))]
    Content,
    #[serde(rename(serialize = "compat"))]
    Compat,
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
// We only currently (22-12-2022) support added/removed stable.
#[DieselTypePath = "crate::db::schema::sql_types::BcdEventType"]
pub enum BcdUpdateEventType {
    #[serde(rename = "added_stable")]
    AddedStable,
    #[serde(rename = "added_preview")]
    AddedPreview,
    #[serde(rename = "added_subfeatures")]
    AddedSubFeatures,
    #[serde(rename = "added_nonnull")]
    AddedNonNull,
    #[serde(rename = "removed_stable")]
    RemovedStable,
    #[serde(other)]
    Unknown,
}
