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
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use futures_util::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value::Null;
use uuid::Uuid;

use crate::{
    ai::help::{prepare_ai_help_req, prepare_ai_help_summary_req, RefDoc},
    api::common::{GeneratedChunk, GeneratedChunkChoice},
    db::{
        self,
        ai_help::{
            add_help_history, add_help_history_message, create_or_increment_total,
            delete_full_help_history, delete_help_history, get_count, help_history,
            help_history_get_message, list_help_history, update_help_history_label, AI_HELP_LIMIT,
        },
        model::{AIHelpHistoryMessage, AIHelpHistoryMessageInsert, Settings},
        settings::get_settings,
        SupaPool,
    },
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
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AIHelpLogMessage {
    pub metadata: AIHelpMeta,
    pub user: ChatCompletionRequestMessage,
    pub assistant: Option<ChatCompletionRequestMessage>,
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

impl From<AIHelpHistoryMessage> for AIHelpLogMessage {
    fn from(value: AIHelpHistoryMessage) -> Self {
        let assistant: Option<ChatCompletionRequestMessage> =
            serde_json::from_value(value.response).unwrap_or_default();
        let user: ChatCompletionRequestMessage =
            serde_json::from_value(value.request).unwrap_or_default();
        let sources: Vec<RefDoc> = serde_json::from_value(value.sources).unwrap_or_default();
        AIHelpLogMessage {
            metadata: AIHelpMeta {
                typ: MetaType::Metadata,
                chat_id: value.chat_id,
                message_id: value.message_id,
                parent_id: value.parent_id,
                sources,
                quota: None,
                created_at: Utc.from_utc_datetime(&value.created_at),
            },
            user,
            assistant,
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
                let log_message: AIHelpLogMessage = log.into();
                if chat_id.is_none() {
                    chat_id = Some(log_message.metadata.chat_id);
                }
                log_message
            })
            .collect();
        Ok(AIHelpLog {
            chat_id: chat_id.unwrap_or_default(),
            messages,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HelpIds {
    chat_id: Uuid,
    message_id: Uuid,
    parent_id: Option<Uuid>,
}

#[derive(Serialize, Default)]
pub struct AIHelpHistorySummaryResponse {
    title: Option<String>,
}

fn history_enabled(settings: &Option<Settings>) -> bool {
    if let Some(settings) = settings {
        return settings.ai_help_history;
    }
    false
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

fn record_question(
    pool: &Data<Pool>,
    message: &ChatCompletionRequestMessage,
    history_enabled: bool,
    user_id: i64,
    help_ids: HelpIds,
) -> Result<Option<NaiveDateTime>, ApiError> {
    if !history_enabled {
        return Ok(None);
    }
    let mut conn = pool.get()?;
    let HelpIds {
        chat_id,
        message_id,
        parent_id,
    } = help_ids;
    if let Err(err) = add_help_history(&mut conn, user_id, chat_id) {
        error!("AI Help log: {err}");
    }
    let insert = AIHelpHistoryMessageInsert {
        user_id,
        chat_id,
        message_id,
        parent_id,
        created_at: None,
        sources: None,
        request: Some(serde_json::to_value(message).unwrap_or(Null)),
        response: None,
    };
    match add_help_history_message(&mut conn, insert) {
        Err(err) => {
            error!("AI Help log: {err}");
            Err(err.into())
        }
        Ok(updated_at) => Ok(Some(updated_at)),
    }
}

fn record_sources(
    pool: &Data<Pool>,
    sources: &Vec<RefDoc>,
    history_enabled: bool,
    user_id: i64,
    help_ids: HelpIds,
) -> Result<Option<NaiveDateTime>, ApiError> {
    if !history_enabled {
        return Ok(None);
    }
    let mut conn = pool.get()?;
    let HelpIds {
        chat_id,
        message_id,
        parent_id,
    } = help_ids;
    let insert = AIHelpHistoryMessageInsert {
        user_id,
        chat_id,
        message_id,
        parent_id,
        created_at: None,
        sources: Some(serde_json::to_value(sources).unwrap_or(Null)),
        request: None,
        response: None,
    };
    match add_help_history_message(&mut conn, insert) {
        Err(err) => {
            error!("AI Help log: {err}");
            Err(err.into())
        }
        Ok(updated_at) => Ok(Some(updated_at)),
    }
}

fn log_errors_and_record_response(
    pool: &Data<Pool>,
    history_enabled: bool,
    user_id: i64,
    help_ids: HelpIds,
) -> Result<Option<mpsc::UnboundedSender<CreateChatCompletionStreamResponse>>, ApiError> {
    let mut conn = pool.get()?;
    let (tx, mut rx) = mpsc::unbounded_channel::<CreateChatCompletionStreamResponse>();
    actix_web::rt::spawn(async move {
        let mut answer = vec![];
        let mut has_finish_reason = false;

        while let Some(mut chunk) = rx.recv().await {
            if let Some(c) = chunk.choices.pop() {
                if let Some(part) = c.delta.content {
                    answer.push(part);
                }
                if let Some(finish_reason) = c.finish_reason {
                    debug!("Finish reason: {finish_reason}");
                    has_finish_reason = true;
                }
            }
        }

        if !has_finish_reason {
            error!("AI Help log: OpenAI stream ended without a finish_reason");
        }

        if history_enabled {
            let HelpIds {
                chat_id,
                message_id,
                parent_id,
            } = help_ids;
            let response = ChatCompletionRequestMessage {
                role: Assistant,
                content: Some(answer.join("")),
                ..Default::default()
            };
            let insert = AIHelpHistoryMessageInsert {
                user_id,
                chat_id,
                message_id,
                parent_id,
                created_at: None,
                sources: None,
                request: None,
                response: Some(serde_json::to_value(response).unwrap_or(Null)),
            };
            if let Err(err) = add_help_history_message(&mut conn, insert) {
                error!("AI Help log: {err}");
            }
        }
    });
    Ok(Some(tx))
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
            created_at: Utc::now(),
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
    let settings = get_settings(&mut conn, &user)?;
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
            chat_id: chat_id_opt,
            parent_id,
            messages,
        } = messages.into_inner();
        let chat_id = chat_id_opt.unwrap_or_else(Uuid::new_v4);
        let message_id = Uuid::new_v4();
        let help_ids = HelpIds {
            chat_id,
            message_id,
            parent_id,
        };

        if let Some(question) = messages.last() {
            record_question(
                &diesel_pool,
                question,
                history_enabled(&settings),
                user.id,
                help_ids,
            )?;
        }

        match prepare_ai_help_req(client, pool, user.is_subscriber(), messages).await? {
            Some(ai_help_req) => {
                let sources = ai_help_req.refs;
                let created_at = match record_sources(
                    &diesel_pool,
                    &sources,
                    history_enabled(&settings),
                    user.id,
                    help_ids,
                )? {
                    Some(x) => Utc.from_utc_datetime(&x),
                    None => Utc::now(),
                };

                let ai_help_meta = AIHelpMeta {
                    typ: MetaType::Metadata,
                    chat_id,
                    message_id,
                    parent_id,
                    sources,
                    quota: current.map(AIHelpLimit::from_count),
                    created_at,
                };
                let tx = log_errors_and_record_response(
                    &diesel_pool,
                    history_enabled(&settings),
                    user.id,
                    help_ids,
                )?;
                let stream = client.chat().create_stream(ai_help_req.req).await.unwrap();
                let refs = stream::once(async move {
                    Ok(sse::Event::Data(
                        sse::Data::new_json(ai_help_meta).map_err(OpenAIError::JSONDeserialize)?,
                    ))
                });

                Ok(Either::Left(sse::Sse::from_stream(refs.chain(
                    stream.map_ok(move |res| {
                        if let Some(ref tx) = tx {
                            if let Err(e) = tx.send(res.clone()) {
                                error!("{e}");
                            }
                        }
                        sse::Event::Data(sse::Data::new_json(res).unwrap())
                    }),
                ))))
            }
            None => {
                let parts = sorry_response(
                    Some(chat_id),
                    message_id,
                    parent_id,
                    current.map(AIHelpLimit::from_count),
                )?;
                let stream = futures::stream::iter(parts.into_iter());
                let res =
                    sse::Sse::from_stream(stream.map(|r| Ok::<_, ApiError>(sse::Event::Data(r))));

                Ok(Either::Right(res))
            }
        }
    } else {
        Err(ApiError::NotImplemented)
    }
}

pub async fn ai_help_title_summary(
    user_id: Identity,
    diesel_pool: Data<Pool>,
    message_id: Path<Uuid>,
    openai_client: Data<Option<Client<OpenAIConfig>>>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let settings = get_settings(&mut conn, &user)?;

    if history_enabled(&settings) {
        if let Some(client) = &**openai_client {
            let hit = help_history_get_message(&mut conn, &user, &message_id.into_inner())?;
            if let Some(hit) = hit {
                let log_message = AIHelpLogMessage::from(hit);
                let req = prepare_ai_help_summary_req(
                    vec![Some(log_message.user), log_message.assistant]
                        .into_iter()
                        .flatten()
                        .collect(),
                )?;
                let mut res = client.chat().create(req).await?;
                let title = res.choices.pop().and_then(|c| c.message.content);
                if let Some(ref title) = title {
                    update_help_history_label(
                        &mut conn,
                        &user,
                        log_message.metadata.chat_id,
                        title,
                    )?;
                }
                return Ok(HttpResponse::Ok().json(AIHelpHistorySummaryResponse { title }));
            }
            return Ok(HttpResponse::NotFound().finish());
        }
        Err(ApiError::Artificial)
    } else {
        Err(ApiError::NotImplemented)
    }
}

pub async fn ai_help_history(
    user_id: Identity,
    diesel_pool: Data<Pool>,
    chat_id: Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let settings = get_settings(&mut conn, &user)?;

    if history_enabled(&settings) {
        let hit = help_history(&mut conn, &user, &chat_id.into_inner())?;
        if !hit.is_empty() {
            let res = AIHelpLog::try_from(hit)?;
            Ok(HttpResponse::Ok().json(res))
        } else {
            Ok(HttpResponse::NotFound().finish())
        }
    } else {
        Err(ApiError::NotImplemented)
    }
}

pub async fn ai_help_list_history(
    user_id: Identity,
    diesel_pool: Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let settings = get_settings(&mut conn, &user)?;
    if history_enabled(&settings) {
        let hit = list_help_history(&mut conn, &user)?;
        Ok(HttpResponse::Ok().json(
            hit.into_iter()
                .map(AIHelpHistoryListEntry::from)
                .collect::<Vec<_>>(),
        ))
    } else {
        Err(ApiError::NotImplemented)
    }
}

pub async fn ai_help_delete_history(
    user_id: Identity,
    diesel_pool: Data<Pool>,
    chat_id: Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let settings = get_settings(&mut conn, &user)?;

    if history_enabled(&settings) {
        if delete_help_history(&mut conn, &user, chat_id.into_inner())? {
            Ok(HttpResponse::NoContent().finish())
        } else {
            Ok(HttpResponse::InternalServerError().finish())
        }
    } else {
        Err(ApiError::NotImplemented)
    }
}

pub async fn ai_help_delete_full_history(
    user_id: Identity,
    diesel_pool: Data<Pool>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    delete_full_help_history(&mut conn, &user)?;
    Ok(HttpResponse::Created().finish())
}
