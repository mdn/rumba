use async_openai::{config::OpenAIConfig, types::CreateEmbeddingRequestArgs, Client};
use itertools::Itertools;

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

const MACRO_DOCS_QUERY: &str = "SELECT
  doc.mdn_url AS url,
  doc.title,
  parent.title_short AS title_parent,
  doc.markdown AS content,
  doc.embedding <=> $1 AS similarity
FROM mdn_doc_macro doc
LEFT JOIN mdn_doc_macro parent ON parent.mdn_url = SUBSTRING(doc.mdn_url, 1, LENGTH(doc.mdn_url) - STRPOS(REVERSE(doc.mdn_url), '/'))
WHERE LENGTH(doc.markdown) >= $4
  AND (doc.embedding <=> $1) < $2
  AND doc.mdn_url NOT LIKE '/en-US/docs/MDN%'
ORDER BY doc.embedding <=> $1
LIMIT $3;";

#[derive(sqlx::FromRow, Clone, Debug)]
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
) -> Result<Vec<RelatedDoc>, AIError> {
    let embedding_req = CreateEmbeddingRequestArgs::default()
        .model(EMBEDDING_MODEL)
        .input(prompt)
        .build()?;
    let embedding_res = client.embeddings().create(embedding_req).await?;

    let embedding =
        pgvector::Vector::from(embedding_res.data.into_iter().next().unwrap().embedding);

    let mut docs: Vec<RelatedDoc> = sqlx::query_as(MACRO_DOCS_QUERY)
        .bind(embedding)
        .bind(MACRO_EMB_DISTANCE)
        .bind(MACRO_EMB_DOC_LIMIT)
        .bind(MACRO_EMB_SEC_MIN_LENGTH)
        .fetch_all(pool)
        .await?;

    let duplicate_titles = get_duplicate_titles(&docs);

    docs.iter_mut().for_each(|doc| {
        if let (true, Some(title_parent)) =
            (duplicate_titles.contains(&doc.title), &doc.title_parent)
        {
            doc.title = format!("{} ({})", doc.title, title_parent);
        }
    });

    Ok(docs)
}

fn get_duplicate_titles<'a>(titles: &'a [RelatedDoc]) -> Vec<String> {
    titles
        .iter()
        .map(|x| &x.title)
        .sorted()
        .dedup_by_with_count(|a, b| a == b)
        .filter_map(|(count, title)| {
            if count > 1 {
                Some(title.to_string())
            } else {
                None
            }
        })
        .collect()
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
