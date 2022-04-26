

use serde::{Deserialize, Serialize};
use crate::db;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, PartialOrd, Ord,Eq, Copy)]
pub enum Subscription {
    #[serde(rename(serialize = "core"))]
    Core,
    #[serde(rename = "mdn_plus_5m")]
    MdnPlus5m,
    #[serde(rename = "mdn_plus_10m")]
    MdnPlus10m,
    #[serde(rename = "mdn_plus_5y")]
    MdnPlus5y,
    #[serde(rename = "mdn_plus_10y")]
    MdnPlus10y,
    #[serde(other)]
    Unknown,
}

impl Into<db::types::Subscription> for Subscription {
    fn into(self) -> db::types::Subscription {
        match self {
            Subscription::MdnPlus5m => db::types::Subscription::MdnPlus_5m,
            Subscription::MdnPlus5y => db::types::Subscription::MdnPlus_5y,
            Subscription::MdnPlus10y => db::types::Subscription::MdnPlus_10y,
            Subscription::MdnPlus10m => db::types::Subscription::MdnPlus_10m,
            Subscription::Core => db::types::Subscription::Core,
            Subscription::Unknown => db::types::Subscription::Core
        }.into()
    }
}