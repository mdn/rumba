use actix_identity::Identity;
use actix_web::{
    web::{Data, Json},
    Either, HttpResponse, Responder,
};
use actix_web_lab::sse;
use async_openai::{types::ChatCompletionRequestMessage, Client};
use futures_util::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;
use crate::{ai::ask::prepare_ask_req, db::SupaPool};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChatRequestMessages {
    messages: Vec<ChatCompletionRequestMessage>,
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

        let refs = stream::once(async {
            Ok(sse::Event::Data(sse::Data::new_json(ask_req.refs).unwrap()))
        });
        return Either::Left(sse::Sse::from_stream(refs.chain(
            stream.map_ok(|res| sse::Event::Data(sse::Data::new_json(res).unwrap())),
        )));
    }
    Either::Right(Ok(HttpResponse::NotImplemented().finish()))
}
