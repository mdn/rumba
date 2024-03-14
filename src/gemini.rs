// Inspired by async-openai.

use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionResponseStreamMessage,
    ChatCompletionStreamResponseDelta, CreateChatCompletionRequest,
    CreateChatCompletionStreamResponse,
};

use futures::Stream;
use itertools::Itertools;
use reqwest::Client;
use reqwest_streams::{error::StreamBodyError, *};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, pin::Pin};

pub struct GeminiClient<C: Config> {
    config: C,
}

pub type GenerateContentResponseStream =
    Pin<Box<dyn Stream<Item = Result<GenerateContentResponse, StreamBodyError>> + Send>>;

impl<C: Config> GeminiClient<C> {
    pub fn with_config(config: C) -> Self {
        Self { config }
    }

    pub async fn create_stream(
        &self,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponseStream, ()> {
        let client = Client::new();
        let api_key = self.config.api_key();
        let model = self.config.model();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent",
            model
        );

        let input = json!(request);
        let res = client
            .post(url)
            .query(&[("key", &api_key)])
            .json(&input)
            .send()
            .await;

        let stream = res
            .map(|res| res.json_array_stream::<GenerateContentResponse>(1024 * 1024))
            .map_err(|_| ());

        stream
    }
}

pub trait Config: Clone {
    fn api_key(&self) -> &str;
    fn model(&self) -> &str;
}

#[derive(Clone)]
pub struct GeminiConfig {
    api_key: String,
    model: String,
}

impl GeminiConfig {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.api_key = api_key.into();
        self
    }

    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = model.into();
        self
    }
}

impl Config for GeminiConfig {
    fn api_key(&self) -> &str {
        &self.api_key
    }
    fn model(&self) -> &str {
        &self.model
    }
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| "".to_string()),
            model: std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-pro".to_string()),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GenerateContentRequest {
    pub contents: Vec<RequestContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Part {
    Text(String),
    InlineData {
        mime_type: String,
        data: String,
    },
    FileData {
        mime_type: String,
        file_uri: String,
    },
    FunctionCall {
        name: String,
        args: HashMap<String, String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<u8>,
}

impl From<CreateChatCompletionRequest> for GenerateContentRequest {
    fn from(value: CreateChatCompletionRequest) -> Self {
        GenerateContentRequest {
            contents: value
                .messages
                .clone()
                .into_iter()
                .map(RequestContent::from)
                .collect(),
            generation_config: Some(&value).map(|v| GenerationConfig {
                temperature: v.temperature,
                max_output_tokens: v.max_tokens,
                top_p: v.top_p,
                candidate_count: v.n,
                ..Default::default()
            }),
        }
    }
}

impl From<ChatCompletionRequestMessage> for RequestContent {
    fn from(value: ChatCompletionRequestMessage) -> Self {
        RequestContent {
            role: Some(from_openai_role(value.role)),
            parts: value.content.map_or(vec![], |text| vec![Part::Text(text)]),
        }
    }
}

fn from_openai_role(role: async_openai::types::Role) -> String {
    match role {
        async_openai::types::Role::System => "system",
        async_openai::types::Role::User => "user",
        async_openai::types::Role::Assistant => "assistant",
        async_openai::types::Role::Function => "function",
    }
    .to_string()
}
fn to_openai_role(role: String) -> Option<async_openai::types::Role> {
    match role.as_str() {
        "system" => Some(async_openai::types::Role::System),
        "user" => Some(async_openai::types::Role::User),
        "assistant" => Some(async_openai::types::Role::Assistant),
        "function" => Some(async_openai::types::Role::Function),
        _ => None,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum GenerateContentResponse {
    Chunk(GenerateContentResponseChunk),
    Error(GenerateContentResponseError),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponseChunk {
    pub candidates: Vec<Candidate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Option<CandidateContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation_metadata: Option<CitationMetadata>,
    pub safety_ratings: Option<Vec<SafetyRating>>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CandidateContent {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    end_index: u32,
    license: String,
    start_index: u32,
    uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CitationMetadata {
    pub citation_sources: Vec<Citation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    candidates_token_count: Option<i32>,
    prompt_token_count: i32,
    total_token_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponseError {
    pub error: GenerateContentResponseErrorDetails,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponseErrorDetails {
    pub code: i32,
    pub message: String,
    pub status: String,
}

impl From<GenerateContentResponse> for CreateChatCompletionStreamResponse {
    fn from(value: GenerateContentResponse) -> Self {
        CreateChatCompletionStreamResponse {
            id: uuid::Uuid::new_v4().to_string(),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u32,
            model: "gemini".to_string(),
            choices: match value.clone() {
                GenerateContentResponse::Chunk(chunk) => chunk
                    .candidates
                    .into_iter()
                    .filter_map(|candidate| match candidate.content {
                        Some(content) => {
                            let msg = ChatCompletionResponseStreamMessage {
                                index: 0, // todo
                                delta: ChatCompletionStreamResponseDelta {
                                    role: to_openai_role(content.role),
                                    content: Some(
                                        content
                                            .parts
                                            .into_iter()
                                            .filter_map(|p| match p {
                                                Part::Text(text) => Some(text),
                                                _ => None,
                                            })
                                            .join(""),
                                    ),
                                    function_call: None,
                                },
                                finish_reason: None,
                            };
                            Some(msg)
                        }
                        None => None,
                    })
                    .collect(),
                GenerateContentResponse::Error(_) => vec![],
            },
        }
    }
}
