use async_openai::{config::OpenAIConfig, types::CreateEmbeddingRequestArgs, Client};

use crate::{
    ai::{constants::EMBEDDING_MODEL, error::AIError},
    db::SupaPool,
};

const DEFAULT_EMB_DISTANCE: f64 = 0.78;
const DEFAULT_EMB_SEC_MIN_LENGTH: i64 = 50;
const DEFAULT_EMB_DOC_LIMIT: i64 = 5;

const DEFAULT_QUERY: &str = "select
mdn_doc.url,
mdn_doc.slug,
mdn_doc.title,
mdn_doc_section.content,
mdn_doc_section.embedding <=> $1 as similarity
from mdn_doc_section left join mdn_doc on mdn_doc.id = mdn_doc_section.doc_id
where length(mdn_doc_section.content) >= $4
and (mdn_doc_section.embedding <=> $1) < $2
order by mdn_doc_section.embedding <=> $1
limit $3;";

const FULL_EMB_DISTANCE: f64 = 0.78;
const FULL_EMB_SEC_MIN_LENGTH: i64 = 50;
const FULL_EMB_DOC_LIMIT: i64 = 5;

const FULL_DOCS_QUERY: &str = "select
mdn_doc.url,
mdn_doc.slug,
mdn_doc.title,
mdn_doc.content,
mdn_doc.embedding <=> $1 as similarity
from mdn_doc
where length(mdn_doc.content) >= $4
and (mdn_doc.embedding <=> $1) < $2
order by mdn_doc.embedding <=> $1
limit $3;";

const MACRO_EMB_DISTANCE: f64 = 0.78;
const MACRO_EMB_SEC_MIN_LENGTH: i64 = 50;
const MACRO_EMB_DOC_LIMIT: i64 = 5;

const MACRO_DOCS_QUERY: &str = "select
mdn_doc_macro.mdn_url as url,
mdn_doc_macro.title,
mdn_doc_macro.markdown as content,
mdn_doc_macro.embedding <=> $1 as similarity
from mdn_doc_macro
where length(mdn_doc_macro.markdown) >= $4
and (mdn_doc_macro.embedding <=> $1) < $2
and mdn_doc_macro.mdn_url not like '/en-US/docs/MDN%'
order by mdn_doc_macro.embedding <=> $1
limit $3;";

#[derive(sqlx::FromRow, Debug)]
pub struct RelatedDoc {
    pub url: String,
    pub title: String,
    pub content: String,
    pub similarity: f64,
}

pub async fn get_related_macro_docs(
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
    let docs: Vec<RelatedDoc> = sqlx::query_as(MACRO_DOCS_QUERY)
        .bind(embedding)
        .bind(MACRO_EMB_DISTANCE)
        .bind(MACRO_EMB_DOC_LIMIT)
        .bind(MACRO_EMB_SEC_MIN_LENGTH)
        .fetch_all(pool)
        .await?;
    Ok(docs)
}

pub async fn get_related_full_docs(
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
    let docs: Vec<RelatedDoc> = sqlx::query_as(FULL_DOCS_QUERY)
        .bind(embedding)
        .bind(FULL_EMB_DISTANCE)
        .bind(FULL_EMB_DOC_LIMIT)
        .bind(FULL_EMB_SEC_MIN_LENGTH)
        .fetch_all(pool)
        .await?;
    Ok(docs)
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
    let docs: Vec<RelatedDoc> = sqlx::query_as(DEFAULT_QUERY)
        .bind(embedding)
        .bind(DEFAULT_EMB_DISTANCE)
        .bind(DEFAULT_EMB_DOC_LIMIT)
        .bind(DEFAULT_EMB_SEC_MIN_LENGTH)
        .fetch_all(pool)
        .await?;
    Ok(docs)
}
