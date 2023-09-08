use actix_identity::Identity;
use actix_web::{
    web::{Data, Json, Path},
    Either, HttpResponse, Responder,
};
use actix_web_lab::{__reexports::tokio::sync::mpsc, sse};
use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, CreateChatCompletionRequest,
        CreateChatCompletionStreamResponse, Role::Assistant,
    },
    Client,
};
use futures_util::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value::Null;
use serde_with::{base64::Base64, serde_as};
use uuid::Uuid;

use crate::{
    ai::{
        ask::{prepare_ask_req, RefDoc},
        constants::{AskConfig, AI_EXPLAIN_VERSION},
        explain::{hash_highlighted, prepare_explain_req, verify_explain_request, ExplainRequest},
    },
    db::{
        ai::{
            add_explain_answer, add_help_log, add_help_log_feedback, create_or_increment_total,
            explain_from_cache, get_count, help_from_log, help_log_list, set_explain_feedback,
            AskFeedback, ExplainFeedback, FeedbackTyp, AI_HELP_LIMIT,
        },
        experiments::get_experiments,
        model::{AIExplainCacheInsert, AIHelpLogs, AIHelpLogsFeedbackInsert, AIHelpLogsInsert},
        SupaPool,
    },
    experiments::Experiments,
};
use crate::{
    api::error::ApiError,
    db::{ai::create_or_increment_limit, users::get_user, Pool},
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChatRequestMessages {
    chat_id: Option<Uuid>,
    messages: Vec<ChatCompletionRequestMessage>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum MetaType {
    #[default]
    Metadata,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AskMeta {
    #[serde(rename = "type")]
    pub typ: MetaType,
    pub chat_id: Uuid,
    pub message_id: i32,
    pub sources: Vec<RefDoc>,
    pub quota: Option<AskLimit>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AskLogMessage {
    pub metadata: AskMeta,
    pub user: ChatCompletionRequestMessage,
    pub assistant: ChatCompletionRequestMessage,
}

#[derive(Serialize, Debug, Clone)]
pub struct AskLog {
    pub chat_id: Uuid,
    pub messages: Vec<AskLogMessage>,
}

impl TryFrom<Vec<AIHelpLogs>> for AskLog {
    type Error = ApiError;

    fn try_from(value: Vec<AIHelpLogs>) -> Result<Self, Self::Error> {
        let mut chat_id = None;
        let messages = value
            .into_iter()
            .map(|log| {
                let res: AskLogResponse = serde_json::from_value(log.response).unwrap_or_default();
                let mut req: CreateChatCompletionRequest =
                    serde_json::from_value(log.request).unwrap_or_default();
                if chat_id.is_none() {
                    chat_id = Some(res.meta.chat_id);
                }
                AskLogMessage {
                    metadata: res.meta,
                    user: req.messages.pop().unwrap_or_default(),
                    assistant: res.answer,
                }
            })
            .collect();
        Ok(AskLog {
            chat_id: chat_id.unwrap_or_default(),
            messages,
        })
    }
}

#[derive(Serialize, Default)]
pub struct GeneratedChunkDelta {
    pub content: String,
}

#[derive(Serialize, Default)]
pub struct GeneratedChunkChoice {
    pub delta: GeneratedChunkDelta,
    pub finish_reason: Option<String>,
}
#[derive(Serialize)]
pub struct GeneratedChunk {
    pub choices: Vec<GeneratedChunkChoice>,
    pub id: i64,
}

impl Default for GeneratedChunk {
    fn default() -> Self {
        Self {
            choices: Default::default(),
            id: 1,
        }
    }
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

impl From<&str> for GeneratedChunk {
    fn from(content: &str) -> Self {
        GeneratedChunk {
            choices: vec![GeneratedChunkChoice {
                delta: GeneratedChunkDelta {
                    content: content.into(),
                },
                ..Default::default()
            }],
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct AskLogResponse {
    meta: AskMeta,
    answer: ChatCompletionRequestMessage,
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
) -> Result<Either<impl Responder, impl Responder>, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let experiments = get_experiments(&mut conn, &user)?;
    let current = if user.is_subscriber() {
        create_or_increment_total(&mut conn, &user)?;
        None
    } else {
        let current = create_or_increment_limit(&mut conn, &user)?;
        if current.is_none() {
            return Err(ApiError::PaymentRequired);
        }
        current
    };
    if let (Some(client), Some(pool)) = (&**openai_client, &**supabase_pool) {
        let ChatRequestMessages { mut chat_id, messages } = messages.into_inner();
        if messages.len() == 1 {
            chat_id = None;
        }
        let message_id = i32::try_from(messages.len()).ok().unwrap_or_default();
        match prepare_ask_req(client, pool, messages, experiments).await? {
            Some(ask_req) => {
                let chat_id = chat_id.unwrap_or_else(Uuid::new_v4);
                let ask_meta = AskMeta {
                    typ: MetaType::Metadata,
                    chat_id,
                    message_id,
                    sources: ask_req.refs,
                    quota: current.map(AskLimit::from_count),
                };
                let tx = if let Some(Experiments {
                    active: true,
                    config,
                }) = experiments
                {
                    let req = ask_req.req.clone();
                    let ask_meta_log = ask_meta.clone();
                    let ask_config = AskConfig::from(config);

                    let (tx, mut rx) =
                        mpsc::unbounded_channel::<CreateChatCompletionStreamResponse>();
                    actix_web::rt::spawn(async move {
                        let mut answer = vec![];
                        while let Some(mut chunk) = rx.recv().await {
                            if let Some(part) = chunk.choices.pop().and_then(|c| c.delta.content) {
                                answer.push(part);
                            }
                        }
                        let insert = AIHelpLogsInsert {
                            user_id: user.id,
                            variant: ask_config.name.to_owned(),
                            chat_id,
                            message_id,
                            created_at: None,
                            request: serde_json::to_value(req).unwrap_or(Null),
                            response: serde_json::to_value(AskLogResponse {
                                meta: ask_meta_log,
                                answer: ChatCompletionRequestMessage {
                                    role: Assistant,
                                    content: Some(answer.join("")),
                                    ..Default::default()
                                },
                            })
                            .unwrap_or(Null),
                        };
                        if let Err(err) = add_help_log(&mut conn, &insert) {
                            error!("AI Help log: {err}");
                        }
                    });
                    Some(tx)
                } else {
                    None
                };
                let stream = client.chat().create_stream(ask_req.req).await.unwrap();
                let refs = stream::once(async move {
                    Ok(sse::Event::Data(
                        sse::Data::new_json(ask_meta).map_err(OpenAIError::JSONDeserialize)?,
                    ))
                });
                return Ok(Either::Left(sse::Sse::from_stream(refs.chain(
                    stream.map_ok(move |res| {
                        if let Some(ref tx) = tx {
                            if let Err(e) = tx.send(res.clone()) {
                                error!("{e}");
                            }
                        }
                        sse::Event::Data(sse::Data::new_json(res).unwrap())
                    }),
                ))));
            }
            None => {
                let parts = vec![
                    sse::Data::new_json(AskMeta {
                        typ: MetaType::Metadata,
                        chat_id: chat_id.unwrap_or_else(Uuid::new_v4),
                        message_id,
                        sources: vec![],
                        quota: current.map(AskLimit::from_count),
                    })
                    .map_err(OpenAIError::JSONDeserialize)?,
                    sse::Data::new_json(GeneratedChunk::from(
                        "Sorry, I don't know how to help with that.",
                    ))
                    .map_err(OpenAIError::JSONDeserialize)?,
                    sse::Data::new_json(GeneratedChunk {
                        choices: vec![GeneratedChunkChoice {
                            finish_reason: Some("stop".to_owned()),
                            ..Default::default()
                        }],
                        ..Default::default()
                    })
                    .map_err(OpenAIError::JSONDeserialize)?,
                ];
                let stream = futures::stream::iter(parts.into_iter());
                let res =
                    sse::Sse::from_stream(stream.map(|r| Ok::<_, ApiError>(sse::Event::Data(r))));

                return Ok(Either::Right(res));
            }
        }
    }
    Err(ApiError::NotImplemented)
}

pub async fn ask_log(
    user_id: Identity,
    diesel_pool: Data<Pool>,
    chat_id: Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let experiments = get_experiments(&mut conn, &user)?;
    if experiments.map(|e| e.active).unwrap_or_default() {
        let hit = help_from_log(&mut conn, &user, &chat_id.into_inner())?;
        if !hit.is_empty() {
            let res = AskLog::try_from(hit)?;
            return Ok(HttpResponse::Ok().json(res));
        }
    }
    Err(ApiError::NotImplemented)
}

pub async fn ask_log_list(
    user_id: Identity,
    diesel_pool: Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let experiments = get_experiments(&mut conn, &user)?;
    if experiments.map(|e| e.active).unwrap_or_default() {
        let hit = help_log_list(&mut conn, &user)?;
        return Ok(HttpResponse::Ok().json(hit));
    }
    Err(ApiError::NotImplemented)
}

pub async fn ask_feedback(
    user_id: Identity,
    diesel_pool: Data<Pool>,
    req: Json<AskFeedback>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let experiments = get_experiments(&mut conn, &user)?;
    if !experiments.map(|ex| ex.active).unwrap_or_default() {
        return Ok(HttpResponse::BadRequest().finish());
    }
    let ask_feedback = req.into_inner();
    let feedback = AIHelpLogsFeedbackInsert {
        feedback: ask_feedback.feedback.map(Some),
        thumbs: ask_feedback
            .thumbs
            .map(|t| Some(t == FeedbackTyp::ThumbsUp)),
    };
    add_help_log_feedback(&mut conn, &user, &ask_feedback.chat_id, ask_feedback.message_id, &feedback)?;
    Ok(HttpResponse::Created().finish())
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
                sse::Data::new_json(GeneratedChunk::from(explanation.as_str()))
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
