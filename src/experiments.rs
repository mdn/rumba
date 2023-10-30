use diesel::{
    backend::Backend, deserialize::FromSql, query_builder::bind_collector::RawBytesBindCollector,
    serialize::ToSql, sql_types::Jsonb,
};
use serde::{Deserialize, Serialize};

use crate::db::model::{ExperimentsQuery, UserQuery};

#[derive(
    FromSqlRow, AsExpression, Debug, Copy, Clone, Serialize, Default, Deserialize, PartialEq,
)]
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

fn restrict(experiment: Option<bool>, allowed: bool) -> Option<bool> {
    if !allowed {
        None
    } else {
        Some(experiment.unwrap_or_default())
    }
}

impl ExperimentsConfig {
    pub fn restrict(&self, user: &UserQuery) -> Self {
        ExperimentsConfig {
            gpt4: restrict(self.gpt4, user.is_admin || user.is_mdn_team),
            full_doc: restrict(self.full_doc, user.is_admin || user.is_mdn_team),
            new_prompt: restrict(self.new_prompt, user.is_admin || user.is_mdn_team),
        }
    }
}

impl From<usize> for ExperimentsConfig {
    fn from(value: usize) -> Self {
        ExperimentsConfig {
            gpt4: Some(value & 0b0001 != 0),
            full_doc: Some(value & 0b0010 != 0),
            new_prompt: Some(value & 0b0100 != 0),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Experiments {
    pub active: bool,
    pub config: ExperimentsConfig,
}

impl Experiments {
    pub fn restrict(&self, user: &UserQuery) -> Option<Self> {
        if user.eligible_for_experiments() {
            Some(Experiments {
                active: self.active,
                config: self.config.restrict(user),
            })
        } else {
            None
        }
    }
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

#[cfg(test)]
mod test {
    use crate::db::model::UserQuery;

    use super::*;

    #[test]
    fn test_experiments_restriction() {
        let mut user = UserQuery::dummy();
        let exp = Experiments::default().restrict(&user);
        assert_eq!(exp, None::<Experiments>);

        user.is_admin = true;
        let exp = Experiments::default().restrict(&user);
        assert!(!exp.unwrap().active);
        assert_eq!(exp.unwrap().config.gpt4, Some(false));
    }
}
