use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[serde_with::skip_serializing_none]
#[derive(Serialize)]
pub struct Search<'a> {
    pub from: u64,
    pub size: u64,
    pub query: Query<'a>,
    pub _source: Source,
    pub highlight: Highlight,
    pub suggest: Option<Suggest>,
    pub sort: Option<Vec<SortField>>,
}

#[derive(Serialize)]
pub struct Count<'a> {
    pub query: Query<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Query<'a> {
    Bool(QueryBool<'a>),
    Term(QueryTerm),
    Terms(QueryTerms),
    Match(QueryMatch),
    MatchPhrase(QueryMatch),
    MultiMatch(QueryMultiMatch),
    FunctionScore(QueryFunctionScore<'a>),
    ConstantScore(QueryConstantScore<'a>),
}

#[serde_with::skip_serializing_none]
#[derive(Default, Serialize)]
pub struct QueryBool<'a> {
    pub filter: Option<Vec<Query<'a>>>,
    pub must: Option<Vec<Query<'a>>>,
    pub should: Option<Vec<Query<'a>>>,
}

#[derive(Serialize)]
pub enum QueryTerm {
    #[serde(rename = "inline_code.exact")]
    InlineCodeExact(String),
    #[serde(rename = "slug_leaf")]
    SlugLeaf(String),
}

#[derive(Serialize)]
pub struct QueryConstantScore<'a> {
    pub filter: Box<Query<'a>>,
    pub boost: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryTerms {
    Locale(Vec<Locale>),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Locale {
    #[serde(rename = "en-us")]
    English,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "ko")]
    Korean,
    #[serde(rename = "pl")]
    Polish,
    #[serde(rename = "pt-br")]
    PortugueseBrazilian,
    #[serde(rename = "ru")]
    Russian,
    #[serde(rename = "zh-cn")]
    ChineseSimplified,
    #[serde(rename = "zh-tw")]
    ChineseTraditional,
}

impl FromStr for Locale {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryMatch {
    Title(QueryMatchField),
    Body(QueryMatchField),
    #[serde(rename = "title.code")]
    TitleCode(QueryMatchField),
    #[serde(rename = "title.joined")]
    TitleJoined(QueryMatchField),
    #[serde(rename = "summary.code")]
    SummaryCode(QueryMatchField),
    #[serde(rename = "inline_code")]
    InlineCode(QueryMatchField),
}

#[derive(Serialize)]
pub struct QueryMultiMatch {
    pub query: String,
    pub fields: Vec<Field>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Field {
    Title,
    Body,
    Popularity,
}

#[serde_with::skip_serializing_none]
#[derive(Default, Serialize)]
pub struct QueryMatchField {
    pub query: String,
    pub boost: f64,
    pub fuzziness: Option<&'static str>,
    pub prefix_length: Option<u64>,
}

#[derive(Serialize)]
pub struct QueryFunctionScore<'a> {
    pub query: &'a Query<'a>,
    pub functions: Vec<QueryFunctionScoreFunction>,
    pub boost_mode: BoostMode,
    pub score_mode: ScoreMode,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BoostMode {
    Sum,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScoreMode {
    Max,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryFunctionScoreFunction {
    FieldValueFactor(QueryFunctionScoreFunctionFieldValueFactor),
}

#[derive(Serialize)]
pub struct QueryFunctionScoreFunctionFieldValueFactor {
    pub field: Field,
    pub factor: u64,
    pub missing: u64,
}

#[derive(Serialize)]
pub struct Source {
    pub excludes: Vec<Field>,
}

#[derive(Serialize)]
pub struct Highlight {
    pub fields: HighlightFields,
    pub pre_tags: Vec<String>,
    pub post_tags: Vec<String>,
    pub number_of_fragments: u64,
    pub fragment_size: u64,
    pub encoder: HighlightEncoder,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HighlightEncoder {
    HTML,
}

#[derive(Serialize)]
pub struct HighlightFields {
    pub title: Value,
    pub body: Value,
}

#[derive(Serialize)]
pub struct Suggest {
    pub text: String,
    pub title_suggestions: Suggester,
    pub body_suggestions: Suggester,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Suggester {
    Term(TermSuggester),
}

#[derive(Serialize)]
pub struct TermSuggester {
    pub field: Field,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortField {
    #[serde(rename = "_score")]
    Score(Order),
    Popularity(Order),
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Deserialize)]
pub struct SearchResponse {
    pub hits: ResponseHits,
    pub suggest: Option<ResponseSuggest>,
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
    // Absent when a hit matches only fields that aren't highlighted.
    #[serde(default)]
    pub highlight: ResponseHighlight,
}

#[derive(Deserialize)]
pub struct ResponseSource {
    #[serde(default = "String::default")]
    pub title: String,
    pub locale: Locale,
    #[serde(default = "String::default")]
    pub slug: String,
    #[serde(default = "f64::default")]
    pub popularity: f64,
    #[serde(default = "String::default")]
    pub summary: String,
}

#[derive(Default, Deserialize, Serialize)]
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
    pub relation: ResponseTotalRelation,
}

#[derive(Deserialize, Serialize)]
pub enum ResponseTotalRelation {
    #[serde(rename = "eq")]
    Equal,
    #[serde(rename = "gte")]
    GreaterThanOrEqual,
}

#[derive(Deserialize)]
pub struct CountResponse {
    pub count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_queries() {
        struct Case {
            name: &'static str,
            query: Query<'static>,
            expected: &'static str,
        }

        let cases = [
            Case {
                name: "constant_score term on inline_code.exact",
                query: Query::ConstantScore(QueryConstantScore {
                    filter: Box::new(Query::Term(QueryTerm::InlineCodeExact(
                        "max-age".to_string(),
                    ))),
                    boost: 10.0,
                }),
                expected: r#"{"constant_score":{"filter":{"term":{"inline_code.exact":"max-age"}},"boost":10.0}}"#,
            },
            Case {
                name: "match on title.code",
                query: Query::Match(QueryMatch::TitleCode(QueryMatchField {
                    query: "%".to_string(),
                    boost: 10.0,
                    ..QueryMatchField::default()
                })),
                expected: r#"{"match":{"title.code":{"query":"%","boost":10.0}}}"#,
            },
            Case {
                name: "fuzzy match on title",
                query: Query::Match(QueryMatch::Title(QueryMatchField {
                    query: "lenght".to_string(),
                    boost: 5.0,
                    fuzziness: Some("AUTO:4,6"),
                    prefix_length: Some(1),
                })),
                expected: r#"{"match":{"title":{"query":"lenght","boost":5.0,"fuzziness":"AUTO:4,6","prefix_length":1}}}"#,
            },
        ];

        for case in cases {
            let actual = serde_json::to_string(&case.query).unwrap();
            assert_eq!(actual, case.expected, "case: {}", case.name);
        }
    }
}
