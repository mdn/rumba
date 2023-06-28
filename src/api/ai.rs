use actix_identity::Identity;
use actix_web::{
    web::{Data, Json},
    Either, HttpResponse, Responder,
};
use actix_web_lab::{__reexports::tokio::sync::mpsc, sse};
use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{ChatCompletionRequestMessage, CreateChatCompletionStreamResponse},
    Client,
};
use futures_util::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_with::{base64::Base64, serde_as};

use crate::{
    ai::{
        ask::{prepare_ask_req, RefDoc},
        constants::AI_EXPLAIN_VERSION,
        explain::{hash_highlighted, prepare_explain_req, verify_explain_request, ExplainRequest},
    },
    db::{
        ai::{
            add_explain_answer, create_or_increment_total, explain_from_cache, get_count,
            set_explain_feedback, ExplainFeedback, AI_HELP_LIMIT,
        },
        model::AIExplainCacheInsert,
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

#[derive(Serialize)]
pub struct CachedChunkDelta {
    pub content: String,
}

#[derive(Serialize)]
pub struct CachedChunkChoice {
    pub delta: CachedChunkDelta,
}
#[derive(Serialize)]
pub struct CachedChunk {
    pub choices: Vec<CachedChunkChoice>,
}

#[serde_as]
#[derive(Serialize)]
pub struct ExplainInitialData {
    cached: bool,
    #[serde_as(as = "Base64")]
    hash: Vec<u8>,
}
#[derive(Serialize)]
pub struct ExplainInitial {
    initial: ExplainInitialData,
}

impl From<&str> for CachedChunk {
    fn from(content: &str) -> Self {
        CachedChunk {
            choices: vec![CachedChunkChoice {
                delta: CachedChunkDelta {
                    content: content.into(),
                },
            }],
        }
    }
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

pub async fn explain_feedback(
    diesel_pool: Data<Pool>,
    req: Json<ExplainFeedback>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    set_explain_feedback(&mut conn, req.into_inner())?;
    Ok(HttpResponse::Created().finish())
}

pub async fn explain(
    openai_client: Data<Option<Client<OpenAIConfig>>>,
    diesel_pool: Data<Pool>,
    req: Json<ExplainRequest>,
) -> Result<Either<impl Responder, impl Responder>, ApiError> {
    let explain_request = req.into_inner();

    if verify_explain_request(&explain_request).is_err() {
        return Err(ApiError::Unauthorized);
    }
    let signature = explain_request.signature.clone();
    let to_be_hashed = if let Some(ref highlighted) = explain_request.highlighted {
        highlighted
    } else {
        &explain_request.sample
    };
    let highlighted_hash = hash_highlighted(to_be_hashed.as_str());
    let hash = highlighted_hash.clone();
    let language = explain_request.language.clone();

    let mut conn = diesel_pool.get()?;
    if let Some(hit) = explain_from_cache(&mut conn, &signature, &highlighted_hash)? {
        if let Some(explanation) = hit.explanation {
            let parts = vec![
                sse::Data::new_json(ExplainInitial {
                    initial: ExplainInitialData { cached: true, hash },
                })
                .map_err(OpenAIError::JSONDeserialize)?,
                sse::Data::new_json(CachedChunk::from(explanation.as_str()))
                    .map_err(OpenAIError::JSONDeserialize)?,
            ];
            let stream = futures::stream::iter(parts.into_iter());
            return Ok(Either::Left(sse::Sse::from_stream(
                stream.map(|r| Ok::<_, ApiError>(sse::Event::Data(r))),
            )));
        }
    }
    if let Some(client) = &**openai_client {
        let explain_req = prepare_explain_req(explain_request, client).await?;
        let stream = client.chat().create_stream(explain_req).await.unwrap();

        let (tx, mut rx) = mpsc::unbounded_channel::<CreateChatCompletionStreamResponse>();

        actix_web::rt::spawn(async move {
            let mut answer = vec![];
            while let Some(mut chunk) = rx.recv().await {
                if let Some(part) = chunk.choices.pop().and_then(|c| c.delta.content) {
                    answer.push(part);
                }
            }
            let insert = AIExplainCacheInsert {
                language,
                signature,
                highlighted_hash,
                explanation: Some(answer.join("")),
                version: AI_EXPLAIN_VERSION,
            };
            if let Err(err) = add_explain_answer(&mut conn, &insert) {
                error!("AI Explain cache: {err}");
            }
        });
        let initial = stream::once(async move {
            Ok::<_, OpenAIError>(sse::Event::Data(
                sse::Data::new_json(ExplainInitial {
                    initial: ExplainInitialData {
                        cached: false,
                        hash,
                    },
                })
                .map_err(OpenAIError::JSONDeserialize)?,
            ))
        });

        return Ok(Either::Right(sse::Sse::from_stream(initial.chain(
            stream.map_ok(move |res| {
                if let Err(e) = tx.send(res.clone()) {
                    error!("{e}");
                }
                sse::Event::Data(sse::Data::new_json(res).unwrap())
            }),
        ))));
    }
    Err(ApiError::Artificial)
}
