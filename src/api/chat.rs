use actix_identity::Identity;
use actix_web::{web::{Data, Json}, HttpResponse};
use async_openai::{types::CreateCompletionRequestArgs, Client};
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
