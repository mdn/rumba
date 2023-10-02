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
use uuid::Uuid;

use crate::{
    ai::{
        constants::AIHelpConfig,
        help::{prepare_ai_help_req, RefDoc},
    },
    api::common::{GeneratedChunk, GeneratedChunkChoice},
    db::{
        ai_help::{
            add_help_log, add_help_log_feedback, create_or_increment_total, get_count,
            help_from_log, help_log_list, AIHelpFeedback, FeedbackTyp, AI_HELP_LIMIT,
        },
        experiments::get_experiments,
        model::{AIHelpLogs, AIHelpLogsFeedbackInsert, AIHelpLogsInsert},
        SupaPool,
    },
    experiments::Experiments,
};
use crate::{
    api::error::ApiError,
    db::{ai_help::create_or_increment_limit, users::get_user, Pool},
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
pub struct AIHelpLimit {
    pub count: i64,
    pub remaining: i64,
    pub limit: i64,
}

impl AIHelpLimit {
    pub fn from_count(count: i64) -> Self {
        Self {
            count,
            remaining: AI_HELP_LIMIT - count,
            limit: AI_HELP_LIMIT,
        }
    }
}

#[derive(Serialize)]
pub struct AIHelpQuota {
    pub quota: Option<AIHelpLimit>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AIHelpMeta {
    #[serde(rename = "type")]
    pub typ: MetaType,
    pub chat_id: Uuid,
    pub message_id: i32,
    pub sources: Vec<RefDoc>,
    pub quota: Option<AIHelpLimit>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AIHelpLogMessage {
    pub metadata: AIHelpMeta,
    pub user: ChatCompletionRequestMessage,
    pub assistant: ChatCompletionRequestMessage,
}

#[derive(Serialize, Debug, Clone)]
pub struct AIHelpLog {
    pub chat_id: Uuid,
    pub messages: Vec<AIHelpLogMessage>,
}

impl TryFrom<Vec<AIHelpLogs>> for AIHelpLog {
    type Error = ApiError;

    fn try_from(value: Vec<AIHelpLogs>) -> Result<Self, Self::Error> {
        let mut chat_id = None;
        let messages = value
            .into_iter()
            .map(|log| {
                let res: AIHelpLogResponse =
                    serde_json::from_value(log.response).unwrap_or_default();
                let mut req: CreateChatCompletionRequest =
                    serde_json::from_value(log.request).unwrap_or_default();
                if chat_id.is_none() {
                    chat_id = Some(res.meta.chat_id);
                }
                AIHelpLogMessage {
                    metadata: res.meta,
                    user: req.messages.pop().unwrap_or_default(),
                    assistant: res.answer,
                }
            })
            .collect();
        Ok(AIHelpLog {
            chat_id: chat_id.unwrap_or_default(),
            messages,
        })
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct AIHelpLogResponse {
    meta: AIHelpMeta,
    answer: ChatCompletionRequestMessage,
}

pub async fn quota(user_id: Identity, diesel_pool: Data<Pool>) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    if user.is_subscriber() {
        Ok(HttpResponse::Ok().json(AIHelpQuota { quota: None }))
    } else {
        let count = get_count(&mut conn, &user)?;
        Ok(HttpResponse::Ok().json(AIHelpQuota {
            quota: Some(AIHelpLimit::from_count(count)),
        }))
    }
}

pub async fn ai_help(
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
        let ChatRequestMessages {
            mut chat_id,
            messages,
        } = messages.into_inner();
        if messages.len() == 1 {
            chat_id = None;
        }
        let message_id = i32::try_from(messages.len()).ok().unwrap_or_default();
        match prepare_ai_help_req(client, pool, messages, experiments).await? {
            Some(ai_help_req) => {
                let chat_id = chat_id.unwrap_or_else(Uuid::new_v4);
                let ai_help_meta = AIHelpMeta {
                    typ: MetaType::Metadata,
                    chat_id,
                    message_id,
                    sources: ai_help_req.refs,
                    quota: current.map(AIHelpLimit::from_count),
                };
                let tx = if let Some(Experiments {
                    active: true,
                    config,
                }) = experiments
                {
                    let req = ai_help_req.req.clone();
                    let ai_help_meta_log = ai_help_meta.clone();
                    let ai_help_config = AIHelpConfig::from(config);

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
                            variant: ai_help_config.name.to_owned(),
                            chat_id,
                            message_id,
                            created_at: None,
                            request: serde_json::to_value(req).unwrap_or(Null),
                            response: serde_json::to_value(AIHelpLogResponse {
                                meta: ai_help_meta_log,
                                answer: ChatCompletionRequestMessage {
                                    role: Assistant,
                                    content: Some(answer.join("")),
                                    ..Default::default()
                                },
                            })
                            .unwrap_or(Null),
                            debug: true,
                        };
                        if let Err(err) = add_help_log(&mut conn, &insert) {
                            error!("AI Help log: {err}");
                        }
                    });
                    Some(tx)
                } else {
                    None
                };
                let stream = client.chat().create_stream(ai_help_req.req).await.unwrap();
                let refs = stream::once(async move {
                    Ok(sse::Event::Data(
                        sse::Data::new_json(ai_help_meta).map_err(OpenAIError::JSONDeserialize)?,
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
                    sse::Data::new_json(AIHelpMeta {
                        typ: MetaType::Metadata,
                        chat_id: chat_id.unwrap_or_else(Uuid::new_v4),
                        message_id,
                        sources: vec![],
                        quota: current.map(AIHelpLimit::from_count),
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

pub async fn ai_help_log(
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
            let res = AIHelpLog::try_from(hit)?;
            return Ok(HttpResponse::Ok().json(res));
        }
    }
    Err(ApiError::NotImplemented)
}

pub async fn ai_help_log_list(
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

pub async fn ai_help_feedback(
    user_id: Identity,
    diesel_pool: Data<Pool>,
    req: Json<AIHelpFeedback>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let experiments = get_experiments(&mut conn, &user)?;
    if !experiments.map(|ex| ex.active).unwrap_or_default() {
        return Ok(HttpResponse::BadRequest().finish());
    }
    let ai_help_feedback = req.into_inner();
    let feedback = AIHelpLogsFeedbackInsert {
        feedback: ai_help_feedback.feedback.map(Some),
        thumbs: ai_help_feedback
            .thumbs
            .map(|t| Some(t == FeedbackTyp::ThumbsUp)),
    };
    add_help_log_feedback(
        &mut conn,
        &user,
        &ai_help_feedback.chat_id,
        ai_help_feedback.message_id,
        &feedback,
    )?;
    Ok(HttpResponse::Created().finish())
}
