use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessageArgs, CreateChatCompletionRequest,
        CreateChatCompletionRequestArgs, CreateModerationRequestArgs, Role,
    },
    Client,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use sha2::{Digest, Sha256};

use crate::{
    ai::{
        constants::{BASIC_MODEL, EXPLAIN_SYSTEM_MESSAGE},
        error::AIError,
    },
    api::error::ApiError,
    settings::SETTINGS,
};

pub type HmacSha256 = Hmac<Sha256>;

#[serde_as]
#[derive(Serialize, Deserialize, Clone)]
pub struct ExplainRequest {
    pub language: Option<String>,
    pub sample: String,
    #[serde_as(as = "Base64")]
    pub signature: Vec<u8>,
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

    mac.update(req.language.clone().unwrap_or_default().as_bytes());
    mac.update(req.sample.as_bytes());

    mac.verify_slice(&req.signature)?;
    Ok(())
}

pub fn hash_highlighted(to_be_hashed: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(to_be_hashed.as_bytes());
    hasher.finalize().to_vec()
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
    let language = language.unwrap_or_default();
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

    if moderation.results.iter().any(|r| r.flagged) {
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
        .model(BASIC_MODEL)
        .messages(vec![system_message, context_message, user_message])
        .temperature(0.0)
        .build()?;
    Ok(req)
}
