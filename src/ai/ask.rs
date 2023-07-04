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
use serde::Serialize;

use crate::{
    ai::{
        constants::{ASK_SYSTEM_MESSAGE, ASK_USER_MESSAGE, MODEL},
        embeddings::get_related_docs,
        error::AIError,
        helpers::{cap_messages, into_user_messages, sanitize_messages},
    },
    db::SupaPool,
};

#[derive(Eq, Hash, PartialEq, Serialize)]
pub struct RefDoc {
    pub url: String,
    pub slug: String,
    pub title: String,
}

pub struct AskRequest {
    pub req: CreateChatCompletionRequest,
    pub refs: Vec<RefDoc>,
}

pub async fn prepare_ask_req(
    client: &Client<OpenAIConfig>,
    pool: &SupaPool,
    messages: Vec<ChatCompletionRequestMessage>,
) -> Result<AskRequest, AIError> {
    let open_ai_messages = sanitize_messages(messages);

    // TODO: sign messages os we don't check again
    let context_messages: Vec<_> = into_user_messages(open_ai_messages);
    let moderations = FuturesUnordered::from_iter(
        context_messages
            .iter()
            .map(|msg| {
                CreateModerationRequestArgs::default()
                    .input(msg.content.clone())
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
        .ok_or(AIError::NoUserPrompt)?;

    let related_docs =
        get_related_docs(client, pool, last_user_message.content.replace('\n', " ")).await?;

    let mut context = vec![];
    let mut refs = vec![];
    let mut token_len = 0;
    for doc in related_docs.into_iter() {
        debug!("url: {}", doc.url);
        let bpe = tiktoken_rs::r50k_base().unwrap();
        let tokens = bpe.encode_with_special_tokens(&doc.content).len();
        token_len += tokens;
        if token_len >= 1500 {
            break;
        }
        context.push(format!("Excerpt from MDN article \"{}\":\n{}", doc.title, doc.content));
        if !refs.iter().any(|r: &RefDoc| r.slug == doc.slug) {
            refs.push(RefDoc {
                url: doc.url,
                slug: doc.slug,
                title: doc.title,
            });
        }
    }
    if context.is_empty() {
        return Err(AIError::NoSources)
    }
    let context = context.join("\n---\n");
    let system_message = ChatCompletionRequestMessageArgs::default()
        .role(Role::System)
        .content(ASK_SYSTEM_MESSAGE)
        .build()
        .unwrap();
    let context_message = ChatCompletionRequestMessageArgs::default()
        .role(Role::User)
        .content(format!("Here is the MDN content:\n{context}"))
        .build()
        .unwrap();
    let user_message = ChatCompletionRequestMessageArgs::default()
        .role(Role::User)
        .content(ASK_USER_MESSAGE)
        .build()
        .unwrap();
    let init_messages = vec![system_message, context_message, user_message];
    let messages = cap_messages(init_messages, context_messages)?;

    let req = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .messages(messages)
        .temperature(0.0)
        .build()?;

    Ok(AskRequest { req, refs })
}
