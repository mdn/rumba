use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessageContent,
};
use tiktoken_rs::ChatCompletionRequestMessage as TiktokenMessage;

use crate::ai::{constants::AIHelpConfig, error::AIError};

pub fn sanitize_messages(
    messages: Vec<ChatCompletionRequestMessage>,
) -> Vec<ChatCompletionRequestMessage> {
    messages
        .into_iter()
        .filter(|message| {
            matches!(
                message,
                ChatCompletionRequestMessage::User(_) | ChatCompletionRequestMessage::Assistant(_)
            )
        })
        .collect()
}

pub fn into_user_messages(
    messages: Vec<ChatCompletionRequestMessage>,
) -> Vec<ChatCompletionRequestMessage> {
    messages
        .into_iter()
        .filter(|message| matches!(message, ChatCompletionRequestMessage::User(_)))
        .collect()
}

/// Returns the text of a message, if it has plain text content.
pub fn message_text(message: &ChatCompletionRequestMessage) -> Option<&str> {
    match message {
        ChatCompletionRequestMessage::User(m) => match &m.content {
            ChatCompletionRequestUserMessageContent::Text(text) => Some(text),
            ChatCompletionRequestUserMessageContent::Array(_) => None,
        },
        ChatCompletionRequestMessage::Assistant(m) => match &m.content {
            Some(ChatCompletionRequestAssistantMessageContent::Text(text)) => Some(text),
            _ => None,
        },
        ChatCompletionRequestMessage::System(m) => match &m.content {
            ChatCompletionRequestSystemMessageContent::Text(text) => Some(text),
            ChatCompletionRequestSystemMessageContent::Array(_) => None,
        },
        _ => None,
    }
}

fn role_name(message: &ChatCompletionRequestMessage) -> &'static str {
    match message {
        ChatCompletionRequestMessage::Developer(_) => "developer",
        ChatCompletionRequestMessage::System(_) => "system",
        ChatCompletionRequestMessage::User(_) => "user",
        ChatCompletionRequestMessage::Assistant(_) => "assistant",
        ChatCompletionRequestMessage::Tool(_) => "tool",
        ChatCompletionRequestMessage::Function(_) => "function",
    }
}

fn num_tokens_from_messages(
    model: &str,
    messages: &[ChatCompletionRequestMessage],
) -> anyhow::Result<usize> {
    let messages: Vec<TiktokenMessage> = messages
        .iter()
        .map(|message| TiktokenMessage {
            role: role_name(message).to_string(),
            content: message_text(message).map(str::to_string),
            ..Default::default()
        })
        .collect();
    tiktoken_rs::num_tokens_from_messages(model, &messages)
}

pub fn cap_messages(
    config: &AIHelpConfig,
    mut init_messages: Vec<ChatCompletionRequestMessage>,
    context_messages: Vec<ChatCompletionRequestMessage>,
) -> Result<Vec<ChatCompletionRequestMessage>, AIError> {
    let init_tokens = num_tokens_from_messages(config.token_model, &init_messages)?;
    if init_tokens + config.max_completion_tokens > config.token_limit {
        return Err(AIError::TokenLimit);
    }
    let mut context_tokens = num_tokens_from_messages(config.token_model, &context_messages)?;

    let mut skip = 0;
    while context_tokens + init_tokens + config.max_completion_tokens > config.token_limit {
        skip += 1;
        if skip >= context_messages.len() {
            return Err(AIError::TokenLimit);
        }
        context_tokens = num_tokens_from_messages(config.token_model, &context_messages[skip..])?;
    }
    init_messages.extend(context_messages.into_iter().skip(skip));
    Ok(init_messages)
}

pub fn get_first_n_chars(input: &str, n: usize) -> String {
    input.chars().take(n).collect()
}
