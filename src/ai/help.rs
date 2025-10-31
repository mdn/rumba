use std::time::Duration;

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs,
        CreateChatCompletionRequest, CreateChatCompletionRequestArgs, CreateModerationRequestArgs,
        Role,
    },
    Client,
};
use futures_util::{stream::FuturesUnordered, TryStreamExt};
use serde::{Deserialize, Serialize};

use crate::{
    ai::{
        constants::{AI_HELP_ADVANCED, AI_HELP_BASIC},
        embeddings::{get_related_docs, get_related_macro_docs},
        error::AIError,
        helpers::{cap_messages, into_user_messages, sanitize_messages},
    },
    db::SupaPool,
    settings::SETTINGS,
};

use super::constants::BASIC_MODEL;

#[derive(Eq, Hash, PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct RefDoc {
    pub url: String,
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AIHelpRequest {
    pub req: CreateChatCompletionRequest,
    pub refs: Vec<RefDoc>,
}

#[derive(Default)]
pub struct AIHelpRequestMeta {
    pub query_len: Option<usize>,
    pub context_len: Option<usize>,
    pub embedding_duration: Option<Duration>,
    pub search_duration: Option<Duration>,
    pub embedding_model: Option<&'static str>,
    pub model: Option<&'static str>,
    pub sources: Option<Vec<RefDoc>>,
}

pub async fn prepare_ai_help_req(
    client: &Client<OpenAIConfig>,
    pool: &SupaPool,
    is_subscriber: bool,
    messages: Vec<ChatCompletionRequestMessage>,
    request_meta: &mut AIHelpRequestMeta,
) -> Result<AIHelpRequest, AIError> {
    let config = if is_subscriber {
        AI_HELP_ADVANCED
    } else {
        AI_HELP_BASIC
    };

    // // check for secret error trigger in the last message
    // // just for QA purposes
    qa_check_for_error_trigger(&messages)?;

    let open_ai_messages = sanitize_messages(messages);

    // TODO: sign messages os we don't check again
    let context_messages: Vec<_> = into_user_messages(open_ai_messages);
    let moderations = FuturesUnordered::from_iter(
        context_messages
            .iter()
            .filter_map(|msg| msg.content.clone())
            .map(|content| {
                CreateModerationRequestArgs::default()
                    .input(content)
                    .build()
                    .unwrap()
            })
            .map(|req| async { client.moderations().create(req).await }),
    )
    .try_collect::<Vec<_>>()
    .await?;

    if let Some(_flagged) = moderations
        .into_iter()
        .flat_map(|moderation| moderation.results)
        .find(|r| r.flagged)
    {
        return Err(AIError::FlaggedError);
    }

    let last_user_message = context_messages
        .iter()
        .last()
        .and_then(|msg| msg.content.as_ref())
        .ok_or(AIError::NoUserPrompt)?;
    request_meta.query_len = Some(last_user_message.len());

    let related_docs = if config.full_doc {
        get_related_macro_docs(
            client,
            pool,
            last_user_message.replace('\n', " "),
            request_meta,
        )
        .await?
    } else {
        get_related_docs(
            client,
            pool,
            last_user_message.replace('\n', " "),
            request_meta,
        )
        .await?
    };

    let mut context = vec![];
    let mut refs = vec![];
    let mut context_len = 0;
    let mut context_token_len = 0;
    for doc in related_docs.into_iter() {
        debug!("url: {}", doc.url);
        context_len += doc.content.len();
        let bpe = tiktoken_rs::r50k_base().unwrap();
        let tokens = bpe.encode_with_special_tokens(&doc.content).len();
        context_token_len += tokens;
        debug!("tokens: {}, token_len: {}", tokens, context_token_len);
        if context_token_len >= config.context_limit {
            context_token_len -= tokens;
            continue;
        }
        if !refs.iter().any(|r: &RefDoc| r.url == doc.url) {
            refs.push(RefDoc {
                url: doc.url.clone(),
                title: doc.title.clone(),
            });
        }
        context.push(doc);
    }
    request_meta.sources = Some(refs.clone());
    request_meta.context_len = Some(context_len);

    let system_message = ChatCompletionRequestMessageArgs::default()
        .role(Role::System)
        .content(config.system_prompt)
        .build()
        .unwrap();
    let context_message = if context.is_empty() {
        None
    } else {
        Some(
            ChatCompletionRequestMessageArgs::default()
                .role(Role::User)
                .content((config.make_context)(context))
                .build()
                .unwrap(),
        )
    };
    let user_message = config.user_prompt.map(|x| {
        ChatCompletionRequestMessageArgs::default()
            .role(Role::User)
            .content(x)
            .build()
            .unwrap()
    });
    let init_messages = vec![Some(system_message), context_message, user_message]
        .into_iter()
        .flatten()
        .collect();
    let messages = cap_messages(&config, init_messages, context_messages)?;

    let req = CreateChatCompletionRequestArgs::default()
        .model(config.model)
        .messages(messages)
        .temperature(0.0)
        .stop(config.stop_phrase.unwrap_or_default())
        .build()?;
    request_meta.model = Some(config.model);

    Ok(AIHelpRequest { req, refs })
}

pub fn prepare_ai_help_summary_req(
    messages: Vec<ChatCompletionRequestMessage>,
) -> Result<CreateChatCompletionRequest, AIError> {
    let system_message = ChatCompletionRequestMessageArgs::default()
        .role(Role::System)
        .content(include_str!("prompts/summary/system.md"))
        .build()
        .unwrap();
    let user_message = ChatCompletionRequestMessageArgs::default()
        .role(Role::User)
        .content(include_str!("prompts/summary/user.md"))
        .build()
        .unwrap();
    let messages = [&[system_message], &messages[..], &[user_message]].concat();

    let req = CreateChatCompletionRequestArgs::default()
        .model(BASIC_MODEL)
        .messages(messages)
        .temperature(0.0)
        .build()?;

    Ok(req)
}

// This function is for QA purposes only, it enables triggering
// an error based on the input message. The message can be optionally
// set in the settings `ai.trigger_error_for_search_term`. Nothing
// will be triggered if the setting is missing, which should be the
// situation in production-like environments.
fn qa_check_for_error_trigger(
    messages: &[ChatCompletionRequestMessage],
) -> Result<(), async_openai::error::OpenAIError> {
    if let Some(magic_words) = SETTINGS
        .ai
        .as_ref()
        .and_then(|ai| ai.trigger_error_for_search_term.as_ref())
    {
        if let Some(msg_text) = messages
            .iter()
            .filter(|m| m.role == Role::User)
            .next_back()
            .and_then(|m| m.content.as_ref())
        {
            if msg_text == magic_words {
                return Err(async_openai::error::OpenAIError::ApiError(
                    async_openai::error::ApiError {
                        message: "Artificial QA error in search phase".to_string(),
                        r#type: None,
                        param: None,
                        code: None,
                    },
                ));
            }
        }
    }
    Ok(())
}
