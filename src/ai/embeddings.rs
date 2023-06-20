use async_openai::{config::OpenAIConfig, types::CreateEmbeddingRequestArgs, Client};

use crate::{
    ai::{constants::EMBEDDING_MODEL, error::AIError},
    db::SupaPool,
};

const EMB_DISTANCE: f64 = 0.78;
const EMB_SEC_MIN_LENGTH: i64 = 50;
const EMB_DOC_LIMIT: i64 = 3;

#[derive(sqlx::FromRow)]
pub struct RelatedDoc {
    pub slug: String,
    pub title: String,
    pub heading: String,
    pub content: String,
    pub similarity: f64,
}

pub async fn get_related_docs(
    client: &Client<OpenAIConfig>,
    pool: &SupaPool,
    prompt: String,
) -> Result<Vec<RelatedDoc>, AIError> {
    let embedding_req = CreateEmbeddingRequestArgs::default()
        .model(EMBEDDING_MODEL)
        .input(prompt)
        .build()?;
    let embedding_res = client.embeddings().create(embedding_req).await?;

    let embedding =
        pgvector::Vector::from(embedding_res.data.into_iter().next().unwrap().embedding);
    let docs: Vec<RelatedDoc> = sqlx::query_as(
        "select
mdn_doc.slug,
mdn_doc.title,
mdn_doc_section.heading,
mdn_doc_section.content,
(mdn_doc_section.embedding <#> $1) * -1 as similarity
from mdn_doc_section left join mdn_doc on mdn_doc.id = mdn_doc_section.doc_id
where length(mdn_doc_section.content) >= $4
and (mdn_doc_section.embedding <#> $1) * -1 > $2
order by mdn_doc_section.embedding <#> $1
limit $3;",
    )
    .bind(embedding)
    .bind(EMB_DISTANCE)
    .bind(EMB_DOC_LIMIT)
    .bind(EMB_SEC_MIN_LENGTH)
    .fetch_all(pool)
    .await?;
    Ok(docs)
}
