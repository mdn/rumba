use actix_identity::Identity;
use actix_web::{
    web::{Data, Json, Path},
    Either, HttpResponse, Responder,
};
use actix_web_lab::{__reexports::tokio::sync::mpsc, sse};
use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{ChatCompletionRequestMessage, CreateChatCompletionStreamResponse, Role::Assistant},
    Client,
};
use chrono::{DateTime, TimeZone, Utc};
use futures_util::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value::Null;
use uuid::Uuid;

use crate::{
    ai::{
        constants::AIHelpConfig,
        help::{prepare_ai_help_req, AIHelpHistoryAndMessage, AIHelpRequest, RefDoc},
    },
    api::common::{GeneratedChunk, GeneratedChunkChoice},
    db::{
        self,
        ai_help::{
            add_help_debug_feedback, add_help_debug_log, add_help_feedback, add_help_history,
            create_or_increment_total, delete_help_history, get_count, help_history,
            list_help_history, AIHelpFeedback, FeedbackTyp, AI_HELP_LIMIT,
        },
        experiments::get_experiments,
        model::{AIHelpDebugLogsInsert, AIHelpFeedbackInsert, AIHelpHistoryMessage},
        SupaPool,
    },
    experiments::{Experiments, ExperimentsConfig},
};
use crate::{
    api::error::ApiError,
    db::{ai_help::create_or_increment_limit, users::get_user, Pool},
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChatRequestMessages {
    chat_id: Option<Uuid>,
    parent_id: Option<Uuid>,
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
    pub message_id: Uuid,
    pub parent_id: Option<Uuid>,
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

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct AIHelpHistoryListEntry {
    pub chat_id: Uuid,
    pub last: DateTime<Utc>,
    pub label: String,
}

impl From<db::ai_help::AIHelpHistoryListEntry> for AIHelpHistoryListEntry {
    fn from(value: db::ai_help::AIHelpHistoryListEntry) -> Self {
        AIHelpHistoryListEntry {
            chat_id: value.chat_id,
            last: Utc.from_utc_datetime(&value.last),
            label: value.label,
        }
    }
}

impl TryFrom<Vec<AIHelpHistoryMessage>> for AIHelpLog {
    type Error = ApiError;

    fn try_from(value: Vec<AIHelpHistoryMessage>) -> Result<Self, Self::Error> {
        let mut chat_id = None;
        let messages = value
            .into_iter()
            .map(|log| {
                let assistant: ChatCompletionRequestMessage =
                    serde_json::from_value(log.response).unwrap_or_default();
                let user: ChatCompletionRequestMessage =
                    serde_json::from_value(log.request).unwrap_or_default();
                let sources: Vec<RefDoc> = serde_json::from_value(log.sources).unwrap_or_default();
                if chat_id.is_none() {
                    chat_id = Some(log.chat_id);
                }
                AIHelpLogMessage {
                    metadata: AIHelpMeta {
                        typ: MetaType::Metadata,
                        chat_id: log.chat_id,
                        message_id: log.message_id,
                        parent_id: log.parent_id,
                        sources,
                        quota: None,
                    },
                    user,
                    assistant,
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

fn record(
    pool: Data<Pool>,
    ai_help_req: &AIHelpRequest,
    config: ExperimentsConfig,
    user_id: i64,
    chat_id: Uuid,
    message_id: Uuid,
    parent_id: Option<Uuid>,
) -> Result<mpsc::UnboundedSender<CreateChatCompletionStreamResponse>, ApiError> {
    let req = ai_help_req.req.clone();
    let sources = ai_help_req.refs.clone();
    let ai_help_config = AIHelpConfig::from(config);
    let mut conn = pool.get()?;

    let (tx, mut rx) = mpsc::unbounded_channel::<CreateChatCompletionStreamResponse>();
    actix_web::rt::spawn(async move {
        let mut answer = vec![];
        while let Some(mut chunk) = rx.recv().await {
            if let Some(part) = chunk.choices.pop().and_then(|c| c.delta.content) {
                answer.push(part);
            }
        }
        let response = ChatCompletionRequestMessage {
            role: Assistant,
            content: Some(answer.join("")),
            ..Default::default()
        };
        let mut created_at = None;
        if config.history.unwrap_or_default() {
            let insert = AIHelpHistoryAndMessage {
                user_id,
                chat_id,
                message_id,
                parent_id,
                created_at,
                sources: &sources,
                request: req.messages.last(),
                response: &response,
            };
            match add_help_history(&mut conn, &insert) {
                Err(err) => {
                    error!("AI Help log: {err}");
                }
                Ok(updated_at) => {
                    created_at = Some(updated_at);
                }
            }
        }
        let insert = AIHelpDebugLogsInsert {
            user_id,
            variant: ai_help_config.name.to_owned(),
            chat_id,
            message_id,
            parent_id,
            created_at,
            sources: serde_json::to_value(&sources).unwrap_or(Null),
            request: serde_json::to_value(&req.messages).unwrap_or(Null),
            response: serde_json::to_value(&response).unwrap_or(Null),
        };
        if let Err(err) = add_help_debug_log(&mut conn, &insert) {
            error!("AI Help log: {err}");
        }
    });
    Ok(tx)
}

pub fn sorry_response(
    chat_id: Option<Uuid>,
    message_id: Uuid,
    parent_id: Option<Uuid>,
    quota: Option<AIHelpLimit>,
) -> Result<Vec<sse::Data>, ApiError> {
    let parts = vec![
        sse::Data::new_json(AIHelpMeta {
            typ: MetaType::Metadata,
            chat_id: chat_id.unwrap_or_else(Uuid::new_v4),
            message_id,
            parent_id,
            sources: vec![],
            quota,
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
    Ok(parts)
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
            parent_id,
            messages,
        } = messages.into_inner();
        if messages.len() == 1 {
            chat_id = None;
        }
        let message_id = Uuid::new_v4();
        match prepare_ai_help_req(client, pool, messages, experiments).await? {
            Some(ai_help_req) => {
                let chat_id = chat_id.unwrap_or_else(Uuid::new_v4);
                let ai_help_meta = AIHelpMeta {
                    typ: MetaType::Metadata,
                    chat_id,
                    message_id,
                    parent_id,
                    sources: ai_help_req.refs.clone(),
                    quota: current.map(AIHelpLimit::from_count),
                };
                let tx = if let Some(Experiments {
                    active: true,
                    config,
                }) = experiments
                {
                    Some(record(
                        diesel_pool,
                        &ai_help_req,
                        config,
                        user.id,
                        chat_id,
                        message_id,
                        parent_id,
                    )?)
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
                let parts = sorry_response(
                    chat_id,
                    message_id,
                    parent_id,
                    current.map(AIHelpLimit::from_count),
                )?;
                let stream = futures::stream::iter(parts.into_iter());
                let res =
                    sse::Sse::from_stream(stream.map(|r| Ok::<_, ApiError>(sse::Event::Data(r))));

                return Ok(Either::Right(res));
            }
        }
    }
    Err(ApiError::NotImplemented)
}

pub async fn ai_help_history(
    user_id: Identity,
    diesel_pool: Data<Pool>,
    chat_id: Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let experiments = get_experiments(&mut conn, &user)?;

    if experiments
        .map(|e| e.active && e.config.history.unwrap_or_default())
        .unwrap_or_default()
    {
        let hit = help_history(&mut conn, &user, &chat_id.into_inner())?;
        if !hit.is_empty() {
            let res = AIHelpLog::try_from(hit)?;
            return Ok(HttpResponse::Ok().json(res));
        }
    }
    Err(ApiError::NotImplemented)
}

pub async fn ai_help_list_history(
    user_id: Identity,
    diesel_pool: Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let experiments = get_experiments(&mut conn, &user)?;
    if experiments
        .map(|e| e.active && e.config.history.unwrap_or_default())
        .unwrap_or_default()
    {
        let hit = list_help_history(&mut conn, &user)?;
        return Ok(HttpResponse::Ok().json(
            hit.into_iter()
                .map(AIHelpHistoryListEntry::from)
                .collect::<Vec<_>>(),
        ));
    }
    Err(ApiError::NotImplemented)
}

pub async fn ai_help_delete_history(
    user_id: Identity,
    diesel_pool: Data<Pool>,
    chat_id: Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    if delete_help_history(&mut conn, &user, chat_id.into_inner())? {
        Ok(HttpResponse::Created().finish())
    } else {
        Err(ApiError::NotImplemented)
    }
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
    let feedback = AIHelpFeedbackInsert {
        message_id: ai_help_feedback.message_id,
        feedback: ai_help_feedback.feedback,
        thumbs: ai_help_feedback.thumbs.map(|t| t == FeedbackTyp::ThumbsUp),
    };
    add_help_feedback(&mut conn, &user, &feedback)?;
    add_help_debug_feedback(&mut conn, &user, &feedback.into())?;

    Ok(HttpResponse::Created().finish())
}
