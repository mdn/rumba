use diesel::{
    backend::Backend, deserialize::FromSql, query_builder::bind_collector::RawBytesBindCollector,
    serialize::ToSql, sql_types::Jsonb,
};
use serde::{Deserialize, Serialize};

use crate::db::model::ExperimentsQuery;

#[derive(FromSqlRow, AsExpression, Debug, Clone, Serialize, Default, Deserialize)]
#[diesel(sql_type = Jsonb)]
#[serde(default)]
pub struct ExperimentsConfig {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::unwrap_or_skip"
    )]
    pub gpt4: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::unwrap_or_skip"
    )]
    pub full_doc: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_with::rust::unwrap_or_skip"
    )]
    pub new_prompt: Option<bool>,
}

#[derive( Debug, Clone, Serialize, Default, Deserialize)]
#[serde(default)]
pub struct Experiments {
    pub active: bool,
    pub config: ExperimentsConfig,
}

impl From<ExperimentsQuery> for Experiments {
    fn from(value: ExperimentsQuery) -> Self {
        Experiments {
            active: value.active,
            config: value.config,
        }
    }
}

impl<DB: Backend> FromSql<Jsonb, DB> for ExperimentsConfig
where
    serde_json::Value: FromSql<Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        serde_json::from_value(serde_json::Value::from_sql(bytes)?).map_err(|e| e.into())
    }
}

impl<DB: Backend> ToSql<Jsonb, DB> for ExperimentsConfig
where
    serde_json::Value: ToSql<Jsonb, DB>,
    for<'c> DB: Backend<BindCollector<'c> = RawBytesBindCollector<DB>>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        let value = serde_json::to_value(self)?;
        value.to_sql(&mut out.reborrow())
    }
}
