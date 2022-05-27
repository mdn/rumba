use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize)]
pub struct Search {
    pub from: i64,
    pub size: i64,
    pub query: Query,
    pub _source: Source,
    pub highlight: Highlight,
    pub suggest: Suggest,
    pub sort: Vec<SortField>,
}

#[derive(Serialize)]
pub struct Count {
    pub query: Query,
}

// TODO: figure out how to actually use enums properly
#[derive(Serialize)]
#[serde(untagged)]
pub enum Query {
    QueryBool(QueryBool),
    QueryTerms(QueryTerms),
    QueryMatch(QueryMatch),
    QueryMatchPhrase(QueryMatchPhrase),
    QueryMultiMatch(QueryMultiMatch),
    QueryFunctionScore(QueryFunctionScore),
}

#[derive(Serialize)]
pub struct QueryBool {
    pub bool: QueryBoolBody,
}

impl QueryBool {
    pub fn new_query(occurrence: &str, queries: Vec<Query>) -> Query {
        let mut body = QueryBoolBody {
            filter: None,
            must: None,
            should: None,
        };
        match occurrence {
            "filter" => body.filter = Some(queries),
            "must" => body.must = Some(queries),
            "should" => body.should = Some(queries),
            _ => unreachable!(),
        }
        Query::QueryBool(QueryBool { bool: body })
    }
}

#[derive(Serialize)]
pub struct QueryBoolBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Vec<Query>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must: Option<Vec<Query>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub should: Option<Vec<Query>>,
}

#[derive(Serialize)]
pub struct QueryTerms {
    pub terms: QueryTermsBody,
}

#[derive(Serialize)]
pub struct QueryTermsBody {
    pub locale: Vec<String>,
}

#[derive(Serialize)]
pub struct QueryMatch {
    pub r#match: QueryMatchBody, // need to escape match keyword
}

impl QueryMatch {
    pub fn new_query(field: &str, field_body: QueryMatchField) -> Query {
        Query::QueryMatch(QueryMatch {
            r#match: create_query_match_body(field, field_body), // need to escape match keyword
        })
    }
}

#[derive(Serialize)]
pub struct QueryMatchPhrase {
    pub match_phrase: QueryMatchBody,
}

impl QueryMatchPhrase {
    pub fn new_query(field: &str, field_body: QueryMatchField) -> Query {
        Query::QueryMatchPhrase(QueryMatchPhrase {
            match_phrase: create_query_match_body(field, field_body),
        })
    }
}

fn create_query_match_body(field: &str, field_body: QueryMatchField) -> QueryMatchBody {
    let mut body = QueryMatchBody {
        title: None,
        body: None,
    };
    match field {
        "title" => body.title = Some(field_body),
        "body" => body.body = Some(field_body),
        _ => unreachable!(),
    }
    body
}

#[derive(Serialize)]
pub struct QueryMatchBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<QueryMatchField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<QueryMatchField>,
}

#[derive(Serialize)]
pub struct QueryMultiMatch {
    pub multi_match: QueryMultiMatchBody,
}

#[derive(Serialize)]
pub struct QueryMultiMatchBody {
    pub query: String,
    pub fields: Vec<String>,
}

#[derive(Serialize)]
pub struct QueryMatchField {
    pub query: String,
    pub boost: f64,
}

#[derive(Serialize)]
pub struct QueryFunctionScore {
    pub function_score: QueryFunctionScoreBody,
}

#[derive(Serialize)]
pub struct QueryFunctionScoreBody {
    pub query: Box<Query>,
    pub functions: Vec<QueryFunctionScoreFunction>,
    pub boost_mode: String,
    pub score_mode: String,
}

#[derive(Serialize)]
pub struct QueryFunctionScoreFunction {
    pub field_value_factor: QueryFunctionScoreFunctionBody,
}

#[derive(Serialize)]
pub struct QueryFunctionScoreFunctionBody {
    pub field: String,
    pub factor: u64,
    pub missing: u64,
}

#[derive(Serialize)]
pub struct Source {
    pub excludes: Vec<String>,
}

#[derive(Serialize)]
pub struct Highlight {
    pub fields: HighlightFields,
    pub pre_tags: Vec<String>,
    pub post_tags: Vec<String>,
    pub number_of_fragments: u64,
    pub fragment_size: u64,
    pub encoder: String,
}

#[derive(Serialize)]
pub struct HighlightFields {
    pub title: Value,
    pub body: Value,
}

#[derive(Serialize)]
pub struct Suggest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_suggestions: Option<SuggestBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_suggestions: Option<SuggestBody>,
}

#[derive(Serialize)]
pub struct SuggestBody {
    pub term: SuggestTerm,
}

#[derive(Serialize)]
pub struct SuggestTerm {
    pub field: String,
}

#[derive(Serialize)]
pub struct SortField {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _score: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub popularity: Option<String>,
}

impl SortField {
    pub fn new(field: &str, order: &str) -> SortField {
        let mut sort_field = SortField {
            _score: None,
            popularity: None,
        };
        match field {
            "_score" => sort_field._score = Some(order.to_string()),
            "popularity" => sort_field.popularity = Some(order.to_string()),
            _ => unreachable!(),
        }
        sort_field
    }
}

#[derive(Deserialize)]
pub struct SearchResponse {
    pub hits: ResponseHits,
    pub suggest: ResponseSuggest,
    pub took: u64,
}

#[derive(Deserialize)]
pub struct ResponseHits {
    pub hits: Vec<ResponseHit>,
    pub total: ResponseTotal,
}

#[derive(Deserialize)]
pub struct ResponseHit {
    pub _id: String,
    pub _score: f64,
    pub _source: ResponseSource,
    pub highlight: ResponseHighlight,
}

#[derive(Deserialize)]
pub struct ResponseSource {
    pub title: String,
    pub locale: String,
    pub slug: String,
    pub popularity: f64,
    pub summary: String,
}

#[derive(Deserialize, Serialize)]
pub struct ResponseHighlight {
    #[serde(default = "Vec::default")]
    pub body: Vec<String>,
    #[serde(default = "Vec::default")]
    pub title: Vec<String>,
}

#[derive(Deserialize)]
pub struct ResponseSuggest {
    pub body_suggestions: Vec<ResponseSuggestion>,
    pub title_suggestions: Vec<ResponseSuggestion>,
}

#[derive(Deserialize)]
pub struct ResponseSuggestion {
    pub options: Vec<ResponseSuggestionOption>,
}

#[derive(Deserialize, Clone)]
pub struct ResponseSuggestionOption {
    pub text: String,
    pub score: f64,
    pub freq: u64,
}

#[derive(Deserialize, Serialize)]
pub struct ResponseTotal {
    pub value: u64,
    pub relation: String,
}

#[derive(Deserialize)]
pub struct CountResponse {
    pub count: u64,
}
