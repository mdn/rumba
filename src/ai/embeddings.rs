use std::time::Instant;

use async_openai::{config::OpenAIConfig, types::CreateEmbeddingRequestArgs, Client};
use itertools::Itertools;

use crate::{
    ai::{constants::EMBEDDING_MODEL, error::AIError, help::AIHelpRequestMeta},
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

const MACRO_DOCS_QUERY: &str = "SELECT
  doc.mdn_url AS url,
  doc.title,
  parent.title_short AS title_parent,
  doc.markdown AS content,
  doc.embedding_next <=> $1 AS similarity
FROM mdn_doc_macro doc
LEFT JOIN mdn_doc_macro parent ON parent.mdn_url = SUBSTRING(doc.mdn_url, 1, LENGTH(doc.mdn_url) - STRPOS(REVERSE(doc.mdn_url), '/'))
WHERE LENGTH(doc.markdown) >= $4
  AND (doc.embedding_next <=> $1) < $2
  AND doc.mdn_url NOT LIKE '/en-US/docs/MDN%'
ORDER BY doc.embedding_next <=> $1
LIMIT $3;";

#[derive(sqlx::FromRow, Debug)]
pub struct RelatedDoc {
    pub url: String,
    pub title: String,
    pub title_parent: Option<String>,
    pub content: String,
    pub similarity: f64,
}

pub async fn get_related_macro_docs(
    client: &Client<OpenAIConfig>,
    pool: &SupaPool,
    prompt: String,
    request_meta: &mut AIHelpRequestMeta,
) -> Result<Vec<RelatedDoc>, AIError> {
    request_meta.embedding_model = Some(EMBEDDING_MODEL);

    let embedding_req = CreateEmbeddingRequestArgs::default()
        .model(EMBEDDING_MODEL)
        .input(prompt)
        .build()?;
    let start = Instant::now();
    let embedding_res = client.embeddings().create(embedding_req).await?;
    request_meta.embedding_duration = Some(start.elapsed());

    let embedding =
        pgvector::Vector::from(embedding_res.data.into_iter().next().unwrap().embedding);

    let start = Instant::now();
    let mut docs: Vec<RelatedDoc> = sqlx::query_as(MACRO_DOCS_QUERY)
        .bind(embedding)
        .bind(MACRO_EMB_DISTANCE)
        .bind(MACRO_EMB_DOC_LIMIT)
        .bind(MACRO_EMB_SEC_MIN_LENGTH)
        .fetch_all(pool)
        .await?;
    request_meta.search_duration = Some(start.elapsed());

    let duplicate_titles: Vec<String> = docs
        .iter()
        .map(|x| x.title.to_string())
        .duplicates()
        .collect();

    docs.iter_mut().for_each(|doc| {
        if let (true, Some(title_parent)) =
            (duplicate_titles.contains(&doc.title), &doc.title_parent)
        {
            doc.title = format!("{} ({})", doc.title, title_parent);
        }
    });

    Ok(docs)
}

pub async fn get_related_full_docs(
    client: &Client<OpenAIConfig>,
    pool: &SupaPool,
    prompt: String,
    request_meta: &mut AIHelpRequestMeta,
) -> Result<Vec<RelatedDoc>, AIError> {
    request_meta.embedding_model = Some(EMBEDDING_MODEL);

    let embedding_req = CreateEmbeddingRequestArgs::default()
        .model(EMBEDDING_MODEL)
        .input(prompt)
        .build()?;
    let start = Instant::now();
    let embedding_res = client.embeddings().create(embedding_req).await?;
    request_meta.embedding_duration = Some(start.elapsed());

    let embedding =
        pgvector::Vector::from(embedding_res.data.into_iter().next().unwrap().embedding);
    let start = Instant::now();
    let docs: Vec<RelatedDoc> = sqlx::query_as(FULL_DOCS_QUERY)
        .bind(embedding)
        .bind(FULL_EMB_DISTANCE)
        .bind(FULL_EMB_DOC_LIMIT)
        .bind(FULL_EMB_SEC_MIN_LENGTH)
        .fetch_all(pool)
        .await?;
    request_meta.search_duration = Some(start.elapsed());

    Ok(docs)
}

pub async fn get_related_docs(
    client: &Client<OpenAIConfig>,
    pool: &SupaPool,
    prompt: String,
    request_meta: &mut AIHelpRequestMeta,
) -> Result<Vec<RelatedDoc>, AIError> {
    request_meta.embedding_model = Some(EMBEDDING_MODEL);

    let embedding_req = CreateEmbeddingRequestArgs::default()
        .model(EMBEDDING_MODEL)
        .input(prompt)
        .build()?;
    let start = Instant::now();
    let embedding_res = client.embeddings().create(embedding_req).await?;
    request_meta.embedding_duration = Some(start.elapsed());

    let embedding =
        pgvector::Vector::from(embedding_res.data.into_iter().next().unwrap().embedding);
    let start = Instant::now();
    let docs: Vec<RelatedDoc> = sqlx::query_as(DEFAULT_QUERY)
        .bind(embedding)
        .bind(DEFAULT_EMB_DISTANCE)
        .bind(DEFAULT_EMB_DOC_LIMIT)
        .bind(DEFAULT_EMB_SEC_MIN_LENGTH)
        .fetch_all(pool)
        .await?;
    request_meta.search_duration = Some(start.elapsed());

    Ok(docs)
}
