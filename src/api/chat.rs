use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use actix_identity::Identity;
use actix_web::{
    web::{Data, Json},
    Either, HttpResponse, Responder,
};
use actix_web_lab::sse;
use async_openai::{
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs,
        CreateChatCompletionRequestArgs, Role,
    },
    Client,
};
use futures_util::TryStreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;
use crate::{ai::ask::prepare_ask_req, db::SupaPool};

#[derive(Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
}

#[derive(Deserialize)]
pub struct ExplainRequest {
    code: String,
    selection: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub reply: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptStyle {
    Chat,
    Readme,
    Html,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model {
    Text,
    Code,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Code {
    pub html: Option<String>,
    pub css: Option<String>,
    pub js: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChatRequestMessage {
    role: Role,
    content: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChatRequestMessages {
    messages: Vec<ChatCompletionRequestMessage>,
}

impl PartialEq for ChatRequestMessage {
    fn eq(&self, other: &Self) -> bool {
        let role = matches!(
            (&self.role, &other.role),
            (Role::System, Role::System)
                | (Role::User, Role::User)
                | (Role::Assistant, Role::Assistant)
        );
        role && self.content == other.content
    }
}
impl Eq for ChatRequestMessage {}

impl std::hash::Hash for ChatRequestMessage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.content.hash(state);
        self.role.to_string().hash(state);
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Hash)]
pub struct ExampleRequest {
    pub prompt: String,
    pub code: Option<Code>,
    pub context: Option<Vec<ChatRequestMessage>>,
}

#[derive(Serialize, Default, Clone)]
pub struct ExampleResponse {
    pub code: Code,
    pub context: Option<Vec<ChatCompletionRequestMessage>>,
}

#[derive(Deserialize)]
pub struct EditRequest {
    pub instruction: String,
    pub input: Option<String>,
}

static MODEL: &str = "gpt-3.5-turbo";

static EXPLAIN_SYSTEM: &str = "You are a system that explains code examples.\
These example come from the MDN web docs. \
You will be given a code example and a selected part of this examples. \
You will explain the selected part in context of the provided code example.";

static EXAMPLE_SYSTEM: &str = "You are a system that provides working front-end code.\
The code must not depend on 3rd party libraries. \
The code must not use style attributes on html tags. \
You reply with code separated in markdown code blocks for HTML, CSS and JS.
You must use codeblocks with language specifiers, like ```js \
and must not include CSS or JavaScript in the html block. \
The user will ask for web components or any part or a web site.";

static EXAMPLE_REFINE: &str = "You must not reply with partial updates, when you modify code reply the fully updated code block(s).";

pub static CACHE: Lazy<Arc<RwLock<HashMap<ExampleRequest, ExampleResponse>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub async fn explain_chat(
    _: Identity,
    openai_client: Data<Option<Client>>,
    chat_request: Json<ExplainRequest>,
) -> Result<HttpResponse, ApiError> {
    if let Some(client) = &**openai_client {
        let ExplainRequest { code, selection } = chat_request.into_inner();
        let system_message = ChatCompletionRequestMessageArgs::default()
            .role(Role::System)
            .content(EXPLAIN_SYSTEM)
            .build()?;
        let prompt = ChatCompletionRequestMessageArgs::default()
                    .role(Role::User)
                    .content(format!(
                        "Given the following code: ```{code}```. Can you explain the following part: {selection}",
                    ))
                    .build()?;
        let request = CreateChatCompletionRequestArgs::default()
            .model(MODEL)
            .messages(vec![system_message, prompt])
            .temperature(0.0)
            .build()?;

        let mut response = client.chat().create(request).await?;
        let reply = response
            .choices
            .pop()
            .map(|r| r.message.content)
            .unwrap_or_default();
        return Ok(HttpResponse::Ok().json(ChatResponse { reply }));
    };
    Ok(HttpResponse::NotImplemented().finish())
}

fn code_to_prompt(Code { html, css, js }: Code) -> String {
    let mut prompt = vec![];
    prompt.push("```html\n");
    if let Some(html) = html.as_ref() {
        prompt.push(html);
    };
    prompt.push("\n```");
    prompt.push("```css\n");
    if let Some(css) = css.as_ref() {
        prompt.push(css);
    };
    prompt.push("\n```");
    prompt.push("```js\n");
    if let Some(js) = js.as_ref() {
        prompt.push(js);
    };
    prompt.push("\n```");

    prompt.join("\n")
}

pub async fn generate_example(
    _: Identity,
    openai_client: Data<Option<Client>>,
    chat_request: Json<ExampleRequest>,
) -> Result<HttpResponse, ApiError> {
    let response = { (*CACHE.read().unwrap()).get(&*chat_request).cloned() };
    if let Some(response) = response {
        println!("cached...");
        return Ok(HttpResponse::Ok().json(response));
    }
    if let Some(client) = &**openai_client {
        let chat_request = chat_request.into_inner();
        let ExampleRequest {
            context,
            prompt,
            code,
        } = chat_request.clone();

        println!("---\n{context:#?}\n---\n{prompt:#?}\n---\n{code:#?}\n---");
        let system_message = ChatCompletionRequestMessageArgs::default()
            .role(Role::System)
            .content(EXAMPLE_SYSTEM)
            .build()?;
        let refine_message = ChatCompletionRequestMessageArgs::default()
            .role(Role::System)
            .content(EXAMPLE_REFINE)
            .build()?;
        let mut messages = match (context, code) {
            (None, None) => {
                let prompt = ChatCompletionRequestMessageArgs::default()
                    .role(Role::User)
                    .content(format!("Give me {}", prompt))
                    .build()?;
                vec![system_message, prompt]
            }
            (None, Some(code)) => {
                let prompt = ChatCompletionRequestMessageArgs::default()
                    .role(Role::User)
                    .content(format!(
                        "Given the following code {}. Can you {prompt}",
                        code_to_prompt(code)
                    ))
                    .build()?;
                vec![system_message, refine_message, prompt]
            }
            (Some(messages), None) => {
                let prompt = ChatCompletionRequestMessageArgs::default()
                    .role(Role::User)
                    .content(format!("Can you {prompt}?",))
                    .build()?;
                messages
                    .into_iter()
                    .map(
                        |ChatRequestMessage { role, content }| ChatCompletionRequestMessage {
                            role,
                            content,
                            name: None,
                        },
                    )
                    .chain(vec![refine_message, prompt])
                    .collect()
            }
            (Some(messages), Some(code)) => {
                let prompt = ChatCompletionRequestMessageArgs::default()
                    .role(Role::User)
                    .content(format!(
                        "I've modified the code to be {}. Can you {prompt}",
                        code_to_prompt(code)
                    ))
                    .build()?;
                messages
                    .into_iter()
                    .map(
                        |ChatRequestMessage { role, content }| ChatCompletionRequestMessage {
                            role,
                            content,
                            name: None,
                        },
                    )
                    .chain(vec![refine_message, prompt])
                    .collect()
            }
        };
        let request = CreateChatCompletionRequestArgs::default()
            .model(MODEL)
            .messages(messages.clone())
            .temperature(0.0)
            .build()?;

        let res = ExampleResponse::default();
        let mut response = client.chat().create(request).await?;
        let reply = if let Some(m) = response.choices.pop().map(|r| r.message) {
            m
        } else {
            return Ok(HttpResponse::Ok().json(res));
        };

        messages.push(ChatCompletionRequestMessage {
            role: reply.role,
            content: reply.content.clone(),
            name: None,
        });
        let content = reply.content;

        println!("{content}");
        let reply_split = content.split("```");

        let mut response = ExampleResponse {
            context: Some(messages),
            ..Default::default()
        };
        for substring in reply_split {
            if let Some(x) = substring.strip_prefix("css") {
                response.code.css = Some(x.trim().to_string());
            } else if let Some(x) = substring.strip_prefix("html") {
                response.code.html = Some(x.trim().to_string());
            } else if let Some(x) = substring.strip_prefix("js") {
                response.code.js = Some(x.trim().to_string());
            }
        }

        CACHE
            .write()
            .unwrap()
            .insert(chat_request, response.clone());

        return Ok(HttpResponse::Ok().json(response));
    };
    Ok(HttpResponse::NotImplemented().finish())
}

pub async fn ask(
    _: Identity,
    openai_client: Data<Option<Client>>,
    supabase_pool: Data<Option<SupaPool>>,
    messages: Json<ChatRequestMessages>,
) -> Either<impl Responder, Result<HttpResponse, ApiError>> {
    if let (Some(client), Some(pool)) = (&**openai_client, &**supabase_pool) {
        let ask_req = match prepare_ask_req(client, pool, messages.into_inner().messages).await {
            Ok(r) => r,
            Err(e) => return Either::Right(Err(e.into())),
        };
        // 1. Prepare messages
        let stream = client.chat().create_stream(ask_req.req).await.unwrap();

        return Either::Left(sse::Sse::from_stream(
            stream.map_ok(|res| sse::Event::Data(sse::Data::new_json(res).unwrap())),
        ));
    }
    Either::Right(Ok(HttpResponse::NotImplemented().finish()))
}
