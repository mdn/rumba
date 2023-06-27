use actix_identity::Identity;
use actix_web::{
    web::{Data, Json},
    Either, HttpResponse, Responder,
};
use actix_web_lab::sse;
use async_openai::{
    config::OpenAIConfig, error::OpenAIError, types::ChatCompletionRequestMessage, Client,
};
use futures_util::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    ai::ask::{prepare_ask_req, RefDoc},
    db::{
        ai::{create_or_increment_total, get_count, AI_HELP_LIMIT},
        SupaPool,
    },
};
use crate::{
    api::error::ApiError,
    db::{ai::create_or_increment_limit, users::get_user, Pool},
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChatRequestMessages {
    messages: Vec<ChatCompletionRequestMessage>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MetaType {
    Metadata,
}

#[derive(Serialize)]
pub struct AskLimit {
    pub count: i64,
    pub remaining: i64,
    pub limit: i64,
}

impl AskLimit {
    pub fn from_count(count: i64) -> Self {
        Self {
            count,
            remaining: AI_HELP_LIMIT - count,
            limit: AI_HELP_LIMIT,
        }
    }
}

#[derive(Serialize)]
pub struct AskQuota {
    pub quota: Option<AskLimit>,
}

#[derive(Serialize)]
pub struct AskMeta {
    #[serde(rename = "type")]
    pub typ: MetaType,
    pub sources: Vec<RefDoc>,
    pub quota: Option<AskLimit>,
}

pub async fn quota(user_id: Identity, diesel_pool: Data<Pool>) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    if user.is_subscriber() {
        Ok(HttpResponse::Ok().json(AskQuota { quota: None }))
    } else {
        let count = get_count(&mut conn, &user)?;
        Ok(HttpResponse::Ok().json(AskQuota {
            quota: Some(AskLimit::from_count(count)),
        }))
    }
}

pub async fn ask(
    user_id: Identity,
    openai_client: Data<Option<Client<OpenAIConfig>>>,
    supabase_pool: Data<Option<SupaPool>>,
    diesel_pool: Data<Pool>,
    messages: Json<ChatRequestMessages>,
) -> Result<Either<impl Responder, HttpResponse>, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let current = if user.is_subscriber() {
        create_or_increment_total(&mut conn, &user)?;
        None
    } else {
        let current = create_or_increment_limit(&mut conn, &user)?;
        if current.is_none() {
            return Ok(Either::Right(HttpResponse::Ok().json(json!(null))));
        }
        current
    };
    if let (Some(client), Some(pool)) = (&**openai_client, &**supabase_pool) {
        let ask_req = prepare_ask_req(client, pool, messages.into_inner().messages).await?;
        // 1. Prepare messages
        let stream = client.chat().create_stream(ask_req.req).await.unwrap();

        let refs = stream::once(async move {
            Ok(sse::Event::Data(
                sse::Data::new_json(AskMeta {
                    typ: MetaType::Metadata,
                    sources: ask_req.refs,
                    quota: current.map(AskLimit::from_count),
                })
                .map_err(OpenAIError::JSONDeserialize)?,
            ))
        });
        return Ok(Either::Left(sse::Sse::from_stream(refs.chain(
            stream.map_ok(|res| sse::Event::Data(sse::Data::new_json(res).unwrap())),
        ))));
    }
    Ok(Either::Right(HttpResponse::NotImplemented().finish()))
}
