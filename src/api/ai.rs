use actix_identity::Identity;
use actix_web::{
    web::{Data, Json},
    Either, HttpResponse, Responder,
};
use actix_web_lab::sse;
use async_openai::{error::OpenAIError, types::ChatCompletionRequestMessage, Client};
use futures_util::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    ai::ask::{prepare_ask_req, RefDoc},
    db::SupaPool,
};
use crate::{
    api::error::ApiError,
    db::{ai::create_or_increment_limit, users::get_user, Pool},
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChatRequestMessages {
    messages: Vec<ChatCompletionRequestMessage>,
}

#[derive(Serialize)]
pub struct AskMeta {
    pub refs: Vec<RefDoc>,
    pub count: Option<i64>,
}

pub async fn ask(
    user_id: Identity,
    openai_client: Data<Option<Client>>,
    supabase_pool: Data<Option<SupaPool>>,
    diesel_pool: Data<Pool>,
    messages: Json<ChatRequestMessages>,
) -> Result<Either<impl Responder, HttpResponse>, ApiError> {
    let mut conn = diesel_pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    let current = if user.is_subscriber() {
        None
    } else {
        let current = create_or_increment_limit(&mut conn, &user)?;
        if current.is_none() {
            return Ok(Either::Right(HttpResponse::Ok().json(json!(null))));
        }
        current
    };
    if let (Some(client), Some(pool)) = (&**openai_client, &**supabase_pool) {
        let ask_req = prepare_ask_req(client, pool, messages.into_inner().messages).await?;
        // 1. Prepare messages
        let stream = client.chat().create_stream(ask_req.req).await.unwrap();

        let refs = stream::once(async move {
            Ok(sse::Event::Data(
                sse::Data::new_json(AskMeta {
                    refs: ask_req.refs,
                    count: current,
                })
                .map_err(OpenAIError::JSONDeserialize)?,
            ))
        });
        return Ok(Either::Left(sse::Sse::from_stream(refs.chain(
            stream.map_ok(|res| sse::Event::Data(sse::Data::new_json(res).unwrap())),
        ))));
    }
    Ok(Either::Right(HttpResponse::NotImplemented().finish()))
}
