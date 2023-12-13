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
        constants::AI_HELP_GPT4_FULL_DOC_NEW_PROMPT,
        embeddings::{get_related_docs, get_related_macro_docs},
        error::AIError,
        helpers::{cap_messages, into_user_messages, sanitize_messages},
    },
    db::SupaPool,
};

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

pub async fn prepare_ai_help_req(
    client: &Client<OpenAIConfig>,
    pool: &SupaPool,
    messages: Vec<ChatCompletionRequestMessage>,
) -> Result<Option<AIHelpRequest>, AIError> {
    let config = AI_HELP_GPT4_FULL_DOC_NEW_PROMPT;
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

    let related_docs = if config.full_doc {
        get_related_macro_docs(client, pool, last_user_message.replace('\n', " ")).await?
    } else {
        get_related_docs(client, pool, last_user_message.replace('\n', " ")).await?
    };

    let mut context = vec![];
    let mut refs = vec![];
    let mut token_len = 0;
    for doc in related_docs.into_iter() {
        debug!("url: {}", doc.url);
        let bpe = tiktoken_rs::r50k_base().unwrap();
        let tokens = bpe.encode_with_special_tokens(&doc.content).len();
        token_len += tokens;
        debug!("tokens: {}, token_len: {}", tokens, token_len);
        if token_len >= config.context_limit {
            token_len -= tokens;
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
        .build()?;

    Ok(Some(AIHelpRequest { req, refs }))
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
        .model("gpt-3.5-turbo")
        .messages(messages)
        .temperature(0.0)
        .build()?;

    Ok(req)
}
