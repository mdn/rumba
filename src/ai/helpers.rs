use async_openai::types::{ChatCompletionRequestMessage, Role};
use tiktoken_rs::async_openai::num_tokens_from_messages;

use crate::ai::{
    constants::{ASK_MAX_COMPLETION_TOKENS, ASK_TOKEN_LIMIT, MODEL},
    error::AIError,
};

pub fn sanitize_messages(
    messages: Vec<ChatCompletionRequestMessage>,
) -> Vec<ChatCompletionRequestMessage> {
    messages
        .into_iter()
        .filter(|message| message.role == Role::User || message.role == Role::Assistant)
        .collect()
}

pub fn into_user_messages(
    messages: Vec<ChatCompletionRequestMessage>,
) -> Vec<ChatCompletionRequestMessage> {
    messages
        .into_iter()
        .filter(|message| message.role == Role::User)
        .collect()
}

pub fn cap_messages(
    mut init_messages: Vec<ChatCompletionRequestMessage>,
    context_messages: Vec<ChatCompletionRequestMessage>,
) -> Result<Vec<ChatCompletionRequestMessage>, AIError> {
    let init_tokens = num_tokens_from_messages(MODEL, &init_messages)?;
    if init_tokens + ASK_MAX_COMPLETION_TOKENS > ASK_TOKEN_LIMIT {
        return Err(AIError::TokenLimit);
    }
    let mut context_tokens = num_tokens_from_messages(MODEL, &context_messages)?;

    let mut index = 0;
    while context_tokens + init_tokens + ASK_MAX_COMPLETION_TOKENS > ASK_TOKEN_LIMIT {
        index += 1;
        if index >= context_messages.len() {
            return Err(AIError::TokenLimit);
        }
        context_tokens = num_tokens_from_messages(MODEL, &context_messages[index..])?;
    }
    init_messages.extend(context_messages.into_iter().skip(index));
    Ok(init_messages)
}
