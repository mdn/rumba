use std::{future, time::Instant};

use actix_identity::Identity;
use actix_web::{
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use actix_web_lab::{__reexports::tokio::sync::mpsc, sse};
use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, CreateChatCompletionStreamResponse,
        Role::{self, Assistant},
    },
    Client,
};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use futures_util::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value::{self, Null};
use uuid::Uuid;

use crate::{
    ai::help::{prepare_ai_help_req, prepare_ai_help_summary_req, AIHelpRequestMeta, RefDoc},
    db::{
        self,
        ai_help::{
            add_help_history_message, add_help_message_meta, create_or_increment_total,
            decrement_limit, delete_full_help_history, delete_help_history, get_count,
            help_history, help_history_get_message, list_help_history, update_help_history_label,
            AI_HELP_LIMIT,
        },
        model::{
            AIHelpHistoryMessage, AIHelpHistoryMessageInsert, AiHelpMessageMetaInsert, Settings,
        },
        settings::get_settings,
        SupaPool,
    },
    settings::SETTINGS,
};
use crate::{
    api::error::ApiError,
    db::{ai_help::create_or_increment_limit, users::get_user, Pool},
};

#[derive(Debug, Clone, Copy, Default)]
struct ResponseContext {
    len: usize,
    status: db::types::AiHelpMessageStatus,
}

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

pub async fn ai_help(
    user_id: Identity,
    openai_client: Data<Option<Client<OpenAIConfig>>>,
    supabase_pool: Data<Option<SupaPool>>,
    diesel_pool: Data<Pool>,
    messages: Json<ChatRequestMessages>,
) -> Result<impl Responder, ApiError> {
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

        let mut ai_help_req_meta = AIHelpRequestMeta::default();
        let prepare_res = prepare_ai_help_req(
            client,
            pool,
            user.is_subscriber(),
            messages,
            &mut ai_help_req_meta,
        )
        .await;
        // Reinstate the user quota if we fail to do the preparation step.
        // Flagged/moderation errors DO count towards the limit, otherwise
        // it is on us.
        match prepare_res {
            Err(crate::ai::error::AIError::OpenAIError(_))
            | Err(crate::ai::error::AIError::TiktokenError(_))
            | Err(crate::ai::error::AIError::TokenLimit)
            | Err(crate::ai::error::AIError::SqlXError(_))
            | Err(crate::ai::error::AIError::NoUserPrompt) => {
                let _ = decrement_limit(&mut conn, &user);
            }
            _ => (),
        }
        let user_id = user.id;

        match prepare_res {
            Ok(ai_help_req) => {
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

                let start = Instant::now();
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
                let qa_error_triggered =
                    qa_check_for_error_trigger(&ai_help_req.req.messages).is_err();
                let ai_help_res_stream =
                    client.chat().create_stream(ai_help_req.req).await.unwrap();
                let refs_sse_data = if qa_error_triggered {
                    Err(OpenAIError::InvalidArgument("Artificial Error".to_owned()))
                } else {
                    sse::Data::new_json(ai_help_meta).map_err(OpenAIError::JSONDeserialize)
                }
                .map(sse::Event::Data)
                .map_err(|e| {
                    let _ = decrement_limit(&mut conn, &user);
                    e
                });

                let refs = stream::once(async move { refs_sse_data });

                let res_stream = ai_help_res_stream
                    .map(Some) // Wrapping response chunks in some.
                    .chain(stream::once(async move { None })) // Adding a None at the end.
                    .scan(ResponseContext::default(), move |context, res| {
                        future::ready(match res {
                            Some(Ok(res)) => {
                                if let Some(ref tx) = tx {
                                    if let Err(e) = tx.send(res.clone()) {
                                        error!("{e}");
                                    }
                                }
                                if let Some(c) = res.choices.first() {
                                    if let Some(part) = &c.delta.content {
                                        context.len += part.len();
                                    }
                                    context.status = match c.finish_reason.as_deref() {
                                        Some("length") => {
                                            db::types::AiHelpMessageStatus::FinishedTooLong
                                        }
                                        Some("stop") => db::types::AiHelpMessageStatus::Success,
                                        Some("content_filter") => {
                                            db::types::AiHelpMessageStatus::FinishedContentFilter
                                        }
                                        Some(_) => db::types::AiHelpMessageStatus::Unknown,
                                        None => db::types::AiHelpMessageStatus::FinishedNoReason,
                                    }
                                }
                                Some(Ok(sse::Event::Data(sse::Data::new_json(res).unwrap())))
                            }
                            res => {
                                let response_duration = start.elapsed();
                                let status = if let Some(Err(e)) = &res {
                                    // reinstate the user quota and pass on the error
                                    let _ = decrement_limit(&mut conn, &user);
                                    e.into()
                                } else {
                                    context.status
                                };
                                let ai_help_message_meta = AiHelpMessageMetaInsert {
                                    user_id,
                                    chat_id,
                                    message_id,
                                    parent_id,
                                    created_at: Some(created_at.naive_utc()),
                                    search_duration: default_meta_big_int(
                                        ai_help_req_meta.search_duration.map(|d| d.as_millis()),
                                    ),
                                    response_duration: default_meta_big_int(Some(
                                        response_duration.as_millis(),
                                    )),
                                    query_len: default_meta_big_int(ai_help_req_meta.query_len),
                                    context_len: default_meta_big_int(ai_help_req_meta.context_len),
                                    response_len: default_meta_big_int(Some(context.len)),
                                    model: ai_help_req_meta.model.unwrap_or(""),
                                    status,
                                    sources: ai_help_req_meta.sources.as_ref().map(|sources| {
                                        serde_json::to_value(sources).unwrap_or(Value::Null)
                                    }),
                                };
                                add_help_message_meta(&mut conn, ai_help_message_meta);

                                if let Some(Err(e)) = res {
                                    Some(Err(e))
                                } else {
                                    None
                                }
                            }
                        })
                    });

                Ok(sse::Sse::from_stream(refs.chain(res_stream)))
            }
            Err(e) => {
                let ai_help_message_meta = AiHelpMessageMetaInsert {
                    user_id: user.id,
                    chat_id,
                    message_id,
                    parent_id,
                    search_duration: default_meta_big_int(
                        ai_help_req_meta.search_duration.map(|d| d.as_millis()),
                    ),
                    query_len: default_meta_big_int(ai_help_req_meta.query_len),
                    context_len: default_meta_big_int(ai_help_req_meta.context_len),
                    model: ai_help_req_meta.model.unwrap_or(""),
                    status: (&e).into(),
                    sources: ai_help_req_meta
                        .sources
                        .as_ref()
                        .map(|sources| serde_json::to_value(sources).unwrap_or(Value::Null)),
                    ..Default::default()
                };
                add_help_message_meta(&mut conn, ai_help_message_meta);

                // Reinstate the user quota if we fail to do the preparation step.
                // Flagged/moderation errors DO count towards the limit, otherwise
                // it is on us.
                match e {
                    crate::ai::error::AIError::OpenAIError(_)
                    | crate::ai::error::AIError::TiktokenError(_)
                    | crate::ai::error::AIError::TokenLimit
                    | crate::ai::error::AIError::SqlXError(_)
                    | crate::ai::error::AIError::NoUserPrompt => {
                        let _ = decrement_limit(&mut conn, &user);
                    }
                    _ => (),
                }

                Err(e.into())
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

// This function is for QA purposes only, it triggering
// an error based on the input message. The message can be optionally
// set in the settings `ai.trigger_error_for_chat_term`. Nothing
// will be triggered if the setting is missing, which should be the
// situation in production-like environments.
fn qa_check_for_error_trigger(
    messages: &[ChatCompletionRequestMessage],
) -> Result<(), crate::api::error::ApiError> {
    if let Some(magic_words) = SETTINGS
        .ai
        .as_ref()
        .and_then(|ai| ai.trigger_error_for_chat_term.as_ref())
    {
        if let Some(msg_text) = messages
            .iter()
            .filter(|m| m.role == Role::User)
            .last()
            .and_then(|m| m.content.as_ref())
        {
            if msg_text == magic_words {
                return Err(crate::api::error::ApiError::Artificial);
            }
        }
    }
    Ok(())
}

fn default_meta_big_int(value: Option<impl TryInto<i64>>) -> Option<i64> {
    value.and_then(|v| v.try_into().ok())
}
