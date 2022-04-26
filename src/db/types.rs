#![allow(non_camel_case_types)]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, DbEnum, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[DieselType = "Subscription_type"]
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
