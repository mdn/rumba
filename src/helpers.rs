use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json::{from_value, Value};

use crate::db::types::Subscription;

pub fn deserialize_enforce_plus<'de, D>(deserializer: D) -> Result<Option<Subscription>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: String = Deserialize::deserialize(deserializer)?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Subscription::from(value)))
    }
}

pub fn deserialize_checkbox<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value: String = Deserialize::deserialize(deserializer)?;
    Ok(value == "on")
}

pub fn deserialize_string_or_vec<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: DeserializeOwned,
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value: Value = Deserialize::deserialize(deserializer)?;
    match from_value::<Vec<T>>(value.clone()) {
        Ok(val) => Ok(val),
        Err(_) => {
            let single_val: T = from_value(value).map_err(Error::custom)?;
            Ok(vec![single_val])
        }
    }
}

pub mod serde_utc_seconds_f {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds: serde_json::Number = Deserialize::deserialize(deserializer)?;
        let (secs, nsecs) = if let Some(secs) = seconds.as_i64() {
            (secs, 0u32)
        } else {
            let secs_f64 = seconds.as_f64().ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "failed to parse `{}` as UTC datetime (in seconds)",
                    seconds
                ))
            })?;
            let secs = secs_f64.trunc();
            let ms = (secs_f64.fract() * 1000f64).trunc() as u32;
            (secs as i64, ms * 1_000_000)
        };
        Utc.timestamp_opt(secs, nsecs).single().ok_or_else(|| {
            serde::de::Error::custom(format!(
                "failed to parse `{}` as UTC datetime (in seconds)",
                seconds
            ))
        })
    }
}

pub mod serde_utc_milliseconds {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds: serde_json::Number = Deserialize::deserialize(deserializer)?;
        let (secs, nsecs) = if let Some(secs) = seconds.as_i64() {
            (secs / 1000, ((secs % 1000) * 1_000_000) as u32)
        } else {
            return Err(serde::de::Error::custom(format!(
                "failed to parse `{}` as UTC datetime (in milliseconds)",
                seconds
            )));
        };
        Utc.timestamp_opt(secs, nsecs).single().ok_or_else(|| {
            serde::de::Error::custom(format!(
                "failed to parse `{}` as UTC datetime (in milliseconds)",
                seconds
            ))
        })
    }

    pub fn serialize<S>(v: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (v.timestamp_millis()).serialize(serializer)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Error;
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::Serialize;
    use serde_json::json;

    #[derive(Deserialize)]
    struct StingOrVec {
        #[serde(deserialize_with = "deserialize_string_or_vec")]
        field: Vec<String>,
    }

    #[derive(Deserialize, Serialize)]
    struct Millis {
        #[serde(with = "serde_utc_milliseconds")]
        dt: DateTime<Utc>,
    }

    #[derive(Deserialize)]
    struct SecondsF {
        #[serde(with = "serde_utc_seconds_f")]
        dt: DateTime<Utc>,
    }

    #[test]
    fn test_string_or_vec() -> Result<(), Error> {
        let json = json!({ "field": "foo" });
        let sov: StingOrVec = serde_json::from_value(json)?;
        assert_eq!(&sov.field, &["foo".to_string()]);

        let json = json!({ "field": ["foo", "bar"] });
        let sov: StingOrVec = serde_json::from_value(json)?;
        assert_eq!(&sov.field, &["foo".to_string(), "bar".to_string()]);

        Ok(())
    }

    #[test]
    fn test_utc_milliseconds() -> Result<(), Error> {
        let json = json!({ "dt": 1655312049699001i64 });
        let dt_serde: Millis = serde_json::from_value(json.clone())?;
        let dt =
            DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(1655312049699, 1_000_000), Utc);
        assert_eq!(dt, dt_serde.dt);

        let millis = Millis { dt };
        let millis_json = serde_json::to_value(&millis)?;
        assert_eq!(json, millis_json);

        Ok(())
    }

    #[test]
    fn test_utc_seconds_f() -> Result<(), Error> {
        let json = json!({ "dt": 1655312049699.1f64 });
        let dt_serde: SecondsF = serde_json::from_value(json)?;
        let dt = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp(1655312049699, 100_000_000),
            Utc,
        );
        assert_eq!(dt, dt_serde.dt);

        Ok(())
    }
}
