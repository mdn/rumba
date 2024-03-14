// Inspired by async-openai.

use async_openai::types::{ChatCompletionRequestMessage, CreateChatCompletionRequest};
use futures::Stream;
use reqwest::Client;
use reqwest_streams::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, pin::Pin};

pub struct GeminiClient<C: Config> {
    config: C,
}

pub type GenerateContentResponseStream =
    Pin<Box<dyn Stream<Item = Result<GenerateContentResponse, error::StreamBodyError>> + Send>>;

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

#[derive(Debug, Serialize, Deserialize)]
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
            role: Some(
                match value.role {
                    async_openai::types::Role::System => "system",
                    async_openai::types::Role::User => "user",
                    async_openai::types::Role::Assistant => "assistant",
                    async_openai::types::Role::Function => "function",
                }
                .to_string(),
            ),
            parts: value.content.map_or(vec![], |text| vec![Part::Text(text)]),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GenerateContentResponse {
    Chunk(GenerateContentResponseChunk),
    Error(GenerateContentResponseError),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponseChunk {
    pub candidates: Vec<Candidate>,
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Option<CandidateContent>,
    pub citation_metadata: Option<CitationMetadata>,
    pub safety_ratings: Option<Vec<SafetyRating>>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CandidateContent {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    end_index: u32,
    license: String,
    start_index: u32,
    uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationMetadata {
    pub citation_sources: Vec<Citation>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    candidates_token_count: Option<i32>,
    prompt_token_count: i32,
    total_token_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponseError {
    pub error: GenerateContentResponseErrorDetails,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponseErrorDetails {
    pub code: i32,
    pub message: String,
    pub status: String,
}
