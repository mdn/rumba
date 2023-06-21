use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessageArgs, CreateChatCompletionRequest,
        CreateChatCompletionRequestArgs, CreateModerationRequestArgs, Role,
    },
    Client,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::{
    ai::{
        constants::{EXPLAIN_SYSTEM_MESSAGE, MODEL},
        error::AIError,
    },
    api::error::ApiError,
    settings::SETTINGS,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
pub struct ExplainRequest {
    pub language: String,
    pub sample: String,
    pub signature: String,
    pub highlighted: Option<String>,
}

pub fn verify_explain_request(req: &ExplainRequest) -> Result<(), anyhow::Error> {
    if let Some(part) = &req.highlighted {
        if !req.sample.contains(part) {
            return Err(ApiError::Artificial.into());
        }
    }
    let mut mac = HmacSha256::new_from_slice(
        &SETTINGS
            .ai
            .as_ref()
            .map(|ai| ai.explain_sign_key)
            .ok_or(ApiError::Artificial)?,
    )?;

    mac.update(req.sample.as_bytes());

    mac.verify_slice(&STANDARD.decode(&req.signature)?)?;
    Ok(())
}

pub async fn prepare_explain_req(
    q: ExplainRequest,
    client: &Client<OpenAIConfig>,
) -> Result<CreateChatCompletionRequest, AIError> {
    let ExplainRequest {
        language,
        sample,
        highlighted,
        ..
    } = q;
    let user_prompt = if let Some(highlighted) = highlighted {
        format!("Explain the following part: ```{language}\n{highlighted}\n```")
    } else {
        "Explain the example in detail.".to_string()
    };
    let context_prompt = format!(
        "Given the following code example is the MDN code example:```{language}\n{sample}\n```"
    );
    let req = CreateModerationRequestArgs::default()
        .input(format!("{user_prompt}\n{context_prompt}"))
        .build()
        .unwrap();
    let moderation = client.moderations().create(req).await?;

    if let Some(_) = moderation.results.iter().find(|r| r.flagged) {
        return Err(AIError::FlaggedError);
    }
    let system_message = ChatCompletionRequestMessageArgs::default()
        .role(Role::System)
        .content(EXPLAIN_SYSTEM_MESSAGE)
        .build()
        .unwrap();
    let context_message = ChatCompletionRequestMessageArgs::default()
        .role(Role::User)
        .content(context_prompt)
        .build()
        .unwrap();
    let user_message = ChatCompletionRequestMessageArgs::default()
        .role(Role::User)
        .content(user_prompt)
        .build()
        .unwrap();
    let req = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .messages(vec![system_message, context_message, user_message])
        .temperature(0.0)
        .build()?;
    Ok(req)
}
