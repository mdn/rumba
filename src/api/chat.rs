use actix_identity::Identity;
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use async_openai::{
    types::{CreateCompletionRequestArgs, CreateEditRequestArgs},
    Client,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::error::ApiError,
    db::{users::get_user, Pool},
};

#[derive(Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub reply: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptStyle {
    Readme,
    Html,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model {
    Text,
    Code,
}

#[derive(Deserialize)]
pub struct ExampleRequest {
    pub prompt: String,
    pub prompt_style: Option<PromptStyle>,
    pub model: Option<Model>,
}

#[derive(Serialize)]
pub struct ExampleResponse {
    pub html: String,
    pub css: Option<String>,
    pub js: Option<String>,
}

#[derive(Deserialize)]
pub struct EditRequest {
    pub instruction: String,
    pub input: Option<String>,
}

pub async fn chat(
    pool: Data<Pool>,
    user_id: Identity,
    openai_client: Data<Option<Client>>,
    chat_request: Json<ChatRequest>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    if !user.is_admin {
        return Ok(HttpResponse::Unauthorized().finish());
    }
    if let Some(client) = &**openai_client {
        let request = CreateCompletionRequestArgs::default()
            .model("text-davinci-003")
            .prompt(&chat_request.prompt)
            .max_tokens(2048_u16)
            .build()?;

        let mut response = client.completions().create(request).await?;
        let reply = response.choices.pop().map(|r| r.text).unwrap_or_default();
        return Ok(HttpResponse::Ok().json(ChatResponse { reply }));
    };
    Ok(HttpResponse::NotImplemented().finish())
}

pub async fn explain(
    openai_client: Data<Option<Client>>,
    chat_request: Json<ChatRequest>,
) -> Result<HttpResponse, ApiError> {
    if let Some(client) = &**openai_client {
        let request = CreateCompletionRequestArgs::default()
            .model("text-davinci-003")
            .prompt(&chat_request.prompt)
            .max_tokens(2048_u16)
            .temperature(0.0)
            .build()?;

        let mut response = client.completions().create(request).await?;
        let reply = response.choices.pop().map(|r| r.text).unwrap_or_default();
        return Ok(HttpResponse::Ok().json(ChatResponse { reply }));
    };
    Ok(HttpResponse::NotImplemented().finish())
}

pub async fn generate_example(
    openai_client: Data<Option<Client>>,
    chat_request: Json<ExampleRequest>,
) -> Result<HttpResponse, ApiError> {
    if let Some(client) = &**openai_client {
        let prompt = match chat_request.prompt_style {
            None | Some(PromptStyle::Readme)
                => format!(
                    "finally, here's an example of {} in seperate markdown code blocks, using plain html, plain css and (if necessary) plain javascript:\n\n```html",
                    chat_request.prompt
                ),
            Some(PromptStyle::Html)
                => format!(
                    "<!-- create an example of {}, using plain html, plain css and (if necessary) plain javascript, without reference to any external libraries -->\n<!doctype html>",
                    chat_request.prompt
                ),
        };
        let request = CreateCompletionRequestArgs::default()
            .model(match chat_request.model {
                None | Some(Model::Text) => "text-davinci-003",
                Some(Model::Code) => "code-davinci-002",
            })
            .prompt(prompt)
            .max_tokens(2048_u16)
            .temperature(0.0)
            .build()?;

        let mut response = client.completions().create(request).await?;
        let reply = response.choices.pop().map(|r| r.text).unwrap_or_default();

        return match chat_request.prompt_style {
            None | Some(PromptStyle::Readme) => {
                let mut reply_split = reply.split("```");

                let mut response = ExampleResponse {
                    html: reply_split.next().unwrap().to_string(),
                    css: None,
                    js: None,
                };
                for substring in reply_split {
                    if let Some(x) = substring.strip_prefix("css") {
                        response.css = Some(x.to_string());
                    } else if let Some(x) = substring.strip_prefix("javascript") {
                        response.js = Some(x.to_string());
                    }
                }

                Ok(HttpResponse::Ok().json(response))
            }
            Some(PromptStyle::Html) => Ok(HttpResponse::Ok().json(ExampleResponse {
                html: reply,
                css: None,
                js: None,
            })),
        };
    };
    Ok(HttpResponse::NotImplemented().finish())
}

pub async fn edit_example(
    openai_client: Data<Option<Client>>,
    chat_request: Json<EditRequest>,
) -> Result<HttpResponse, ApiError> {
    if let Some(client) = &**openai_client {
        let request = CreateEditRequestArgs::default()
            .model("code-davinci-edit-001")
            .temperature(0.0)
            .instruction(chat_request.instruction.clone())
            .input(chat_request.input.clone().unwrap_or_default())
            .build()?;

        let mut response = client.edits().create(request).await?;
        let reply = response.choices.pop().map(|r| r.text).unwrap_or_default();

        return Ok(HttpResponse::Ok().json(ChatResponse { reply }));
    };
    Ok(HttpResponse::NotImplemented().finish())
}
