use actix_web::{
    web::{Data, Json},
    Either, HttpResponse, Responder,
};
use actix_web_lab::{__reexports::tokio::sync::mpsc, sse};
use async_openai::{
    config::OpenAIConfig, error::OpenAIError, types::CreateChatCompletionStreamResponse, Client,
};
use futures_util::{stream, StreamExt, TryStreamExt};
use serde::Serialize;
use serde_with::{base64::Base64, serde_as};

use crate::{
    ai::{
        constants::AI_EXPLAIN_VERSION,
        explain::{hash_highlighted, prepare_explain_req, verify_explain_request, ExplainRequest},
    },
    api::common::GeneratedChunk,
    db::{
        ai_explain::{
            add_explain_answer, explain_from_cache, set_explain_feedback, ExplainFeedback,
        },
        model::AIExplainCacheInsert,
    },
};
use crate::{api::error::ApiError, db::Pool};

#[serde_as]
#[derive(Serialize)]
pub struct ExplainInitialData {
    cached: bool,
    #[serde_as(as = "Base64")]
    hash: Vec<u8>,
}
#[derive(Serialize)]
pub struct ExplainInitial {
    initial: ExplainInitialData,
}

pub async fn explain_feedback(
    diesel_pool: Data<Pool>,
    req: Json<ExplainFeedback>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = diesel_pool.get()?;
    set_explain_feedback(&mut conn, req.into_inner())?;
    Ok(HttpResponse::Created().finish())
}

pub async fn explain(
    openai_client: Data<Option<Client<OpenAIConfig>>>,
    diesel_pool: Data<Pool>,
    req: Json<ExplainRequest>,
) -> Result<Either<impl Responder, impl Responder>, ApiError> {
    let explain_request = req.into_inner();

    if verify_explain_request(&explain_request).is_err() {
        return Err(ApiError::Unauthorized);
    }
    let signature = explain_request.signature.clone();
    let to_be_hashed = if let Some(ref highlighted) = explain_request.highlighted {
        highlighted
    } else {
        &explain_request.sample
    };
    let highlighted_hash = hash_highlighted(to_be_hashed.as_str());
    let hash = highlighted_hash.clone();
    let language = explain_request.language.clone();

    let mut conn = diesel_pool.get()?;
    if let Some(hit) = explain_from_cache(&mut conn, &signature, &highlighted_hash)? {
        if let Some(explanation) = hit.explanation {
            let parts = vec![
                sse::Data::new_json(ExplainInitial {
                    initial: ExplainInitialData { cached: true, hash },
                })
                .map_err(OpenAIError::JSONDeserialize)?,
                sse::Data::new_json(GeneratedChunk::from(explanation.as_str()))
                    .map_err(OpenAIError::JSONDeserialize)?,
            ];
            let stream = futures::stream::iter(parts.into_iter());
            return Ok(Either::Left(sse::Sse::from_stream(
                stream.map(|r| Ok::<_, ApiError>(sse::Event::Data(r))),
            )));
        }
    }
    if let Some(client) = &**openai_client {
        let explain_req = prepare_explain_req(explain_request, client).await?;
        let stream = client.chat().create_stream(explain_req).await.unwrap();

        let (tx, mut rx) = mpsc::unbounded_channel::<CreateChatCompletionStreamResponse>();

        actix_web::rt::spawn(async move {
            let mut answer = vec![];
            while let Some(mut chunk) = rx.recv().await {
                if let Some(part) = chunk.choices.pop().and_then(|c| c.delta.content) {
                    answer.push(part);
                }
            }
            let insert = AIExplainCacheInsert {
                language,
                signature,
                highlighted_hash,
                explanation: Some(answer.join("")),
                version: AI_EXPLAIN_VERSION,
            };
            if let Err(err) = add_explain_answer(&mut conn, &insert) {
                error!("AI Explain cache: {err}");
            }
        });
        let initial = stream::once(async move {
            Ok::<_, OpenAIError>(sse::Event::Data(
                sse::Data::new_json(ExplainInitial {
                    initial: ExplainInitialData {
                        cached: false,
                        hash,
                    },
                })
                .map_err(OpenAIError::JSONDeserialize)?,
            ))
        });

        return Ok(Either::Right(sse::Sse::from_stream(initial.chain(
            stream.map_ok(move |res| {
                if let Err(e) = tx.send(res.clone()) {
                    error!("{e}");
                }
                sse::Event::Data(sse::Data::new_json(res).unwrap())
            }),
        ))));
    }
    Err(ApiError::Artificial)
}
