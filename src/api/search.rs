use crate::api::elastic;
use crate::api::error::{ApiError, SearchError};
use crate::settings::SETTINGS;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::{web, HttpRequest, HttpResponse};
use elasticsearch::http::response::Response as ElasticResponse;
use elasticsearch::{CountParts, Elasticsearch, SearchParts};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use std::str::FromStr;

// TODO: add retry logic from kuma

#[derive(Serialize)]
struct SearchResponse {
    documents: Vec<Document>,
    metadata: Metadata,
    suggestions: Vec<Suggestion>,
}

#[derive(Serialize)]
struct Suggestion {
    text: String,
    total: elastic::ResponseTotal,
}

#[derive(Serialize)]
struct Metadata {
    took_ms: u64,
    size: u64,
    page: u64,
    total: elastic::ResponseTotal,
}

#[derive(Serialize)]
struct Document {
    mdn_url: String,
    score: f64,
    title: String,
    locale: elastic::Locale,
    slug: String,
    popularity: f64,
    summary: String,
    highlight: elastic::ResponseHighlight,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum Sort {
    #[default]
    Best,
    Relevance,
    Popularity,
}

#[derive(Deserialize)]
struct Params {
    q: String,
    #[serde(default)]
    sort: Sort,
    #[serde(default = "default_page")]
    page: u64,
    #[serde(skip)]
    locale: Vec<elastic::Locale>,
}

fn default_page() -> u64 {
    1
}

impl FromStr for Params {
    type Err = SearchError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        /*
        FIXME: this horrendous complexity only exists because no mature rust library supports
        repeated query keys to specify an array value. after we fully migrate away from kuma,
        and no longer need to remain api-compatible, we should change this behaviour.
        */
        let mut params: Params = serde_path_to_error::deserialize(
            serde_urlencoded::Deserializer::new(form_urlencoded::parse(s.as_bytes())),
        )
        .map_err(|e| SearchError::Query {
            key: e.path().to_string(),
            message: e.inner().to_string(),
        })?;

        if params.q.len() > SETTINGS.search.query_max_length {
            return Err(SearchError::Query {
                key: "q".to_string(),
                message: format!(
                    "Ensure this value is less than or equal to {}.",
                    SETTINGS.search.query_max_length
                ),
            });
        }

        match params.page {
            x if x < 1 => {
                return Err(SearchError::Query {
                    key: "page".to_string(),
                    message: "Ensure this value is greater than or equal to 1.".to_string(),
                })
            }
            x if x > 10 => {
                return Err(SearchError::Query {
                    key: "page".to_string(),
                    message: "Ensure this value is less than or equal to 10.".to_string(),
                })
            }
            _ => (),
        }

        for value in web::Query::<Vec<(String, String)>>::from_query(s)
            .unwrap_or_else(|_| web::Query(vec![]))
            .iter()
            .filter_map(
                |(key, value)| {
                    if key == "locale" {
                        Some(value)
                    } else {
                        None
                    }
                },
            )
        {
            params.locale.push(
                value
                    .to_lowercase()
                    .parse::<elastic::Locale>()
                    .map_err(|e| SearchError::Query {
                        key: "locale".to_string(),
                        message: e.to_string(),
                    })?,
            );
        }
        if params.locale.is_empty() {
            params.locale.push(elastic::Locale::English);
        }

        Ok(params)
    }
}

pub async fn search(
    request: HttpRequest,
    client: web::Data<Elasticsearch>,
) -> Result<HttpResponse, ApiError> {
    let params: Params = request.query_string().parse()?;

    let mut search_response: elastic::SearchResponse =
        parse_or_log(do_search(&client, &params, subqueries(&params.q), true).await).await?;
    if search_response.hits.total.value == 0 {
        let fallback: elastic::SearchResponse =
            parse_or_log(do_search(&client, &params, fallback_subqueries(&params.q), false).await)
                .await?;
        search_response.took += fallback.took;
        search_response.hits = fallback.hits;
    }

    let response = SearchResponse {
        documents: search_response
            .hits
            .hits
            .into_iter()
            .map(|hit| Document {
                mdn_url: hit._id,
                score: hit._score,
                title: hit._source.title,
                locale: hit._source.locale,
                slug: hit._source.slug,
                popularity: hit._source.popularity,
                summary: hit._source.summary,
                highlight: hit.highlight,
            })
            .collect(),
        metadata: Metadata {
            took_ms: search_response.took,
            total: search_response.hits.total,
            size: 10,
            page: params.page,
        },
        suggestions: match search_response.suggest {
            Some(x) => get_suggestion(x, &client, &params.locale)
                .await
                .unwrap_or_default(),
            None => vec![],
        },
    };
    Ok(HttpResponse::Ok()
        .insert_header(CacheControl(vec![CacheDirective::MaxAge(
            SETTINGS.search.cache_max_age,
        )]))
        .json(response))
}

async fn parse_or_log(
    result: Result<ElasticResponse, elasticsearch::Error>,
) -> Result<elastic::SearchResponse, ApiError> {
    parse_or_get_error_reason(result).await.map_err(|e| {
        error!("{}", e);
        e.into()
    })
}

/// Whether the query contains punctuation or a camelCase transition.
fn is_code_like(q: &str) -> bool {
    let chars: Vec<char> = q.chars().collect();
    chars
        .iter()
        .any(|c| !c.is_alphanumeric() && !c.is_whitespace())
        || chars
            .windows(2)
            .any(|w| w[0].is_lowercase() && w[1].is_uppercase())
}

/// Whether the query consists only of punctuation, like `%` or `?.`.
fn is_symbols_only(q: &str) -> bool {
    !q.is_empty()
        && q.chars()
            .all(|c| !c.is_alphanumeric() && !c.is_whitespace())
}

fn match_field(query: &str, boost: f64) -> elastic::QueryMatchField {
    elastic::QueryMatchField {
        query: query.to_string(),
        boost,
        ..elastic::QueryMatchField::default()
    }
}

fn fuzzy_field(query: &str, boost: f64, fuzziness: &'static str) -> elastic::QueryMatchField {
    elastic::QueryMatchField {
        fuzziness: Some(fuzziness),
        prefix_length: Some(1),
        ..match_field(query, boost)
    }
}

fn constant_term(term: elastic::QueryTerm, boost: f64) -> elastic::Query<'static> {
    elastic::Query::ConstantScore(elastic::QueryConstantScore {
        filter: Box::new(elastic::Query::Term(term)),
        boost,
    })
}

/*
The business logic here that we search for things different ways,
and each different way as a different boost which dictates its importance.
The importance order is as follows:

 1. Title match-phrase
 2. Title match, and for symbol-only queries (`%`) title and summary
    matches that keep punctuation
 3. Exact inline code value, as a flat bonus for code-like queries
 4. Body match-phrase
 5. Body match

The order is determined by the `boost` number in the code below.
Remember that sort order is a combination of "match" and popularity, but
ideally the popularity should complement. Try to get a pretty good
sort by pure relevance first, and let popularity just make it better.
*/
fn subqueries(q: &str) -> Vec<elastic::Query<'static>> {
    let mut subqueries = vec![
        elastic::Query::Match(elastic::QueryMatch::Title(match_field(q, 5.0))),
        elastic::Query::Match(elastic::QueryMatch::Body(match_field(q, 1.0))),
    ];
    if is_code_like(q) {
        // Constant, so that the many pages mentioning a value don't outrank its title match.
        subqueries.push(constant_term(
            elastic::QueryTerm::InlineCodeExact(q.trim().to_string()),
            10.0,
        ));
    }
    if is_symbols_only(q) {
        subqueries.push(elastic::Query::Match(elastic::QueryMatch::TitleCode(
            match_field(q, 10.0),
        )));
        subqueries.push(elastic::Query::Match(elastic::QueryMatch::SummaryCode(
            match_field(q, 5.0),
        )));
    }
    if q.contains(' ') {
        subqueries.push(elastic::Query::MatchPhrase(elastic::QueryMatch::Title(
            match_field(q, 10.0),
        )));
        subqueries.push(elastic::Query::MatchPhrase(elastic::QueryMatch::Body(
            match_field(q, 2.0),
        )));
    }
    subqueries
}

/// Used when `subqueries` find nothing: typos and stopwords (`a`, `then`).
fn fallback_subqueries(q: &str) -> Vec<elastic::Query<'static>> {
    vec![
        // Title terms tolerate 1 edit from 4 characters and 2 from 6; the larger body
        // vocabulary only 1 edit from 6 characters, to limit junk matches.
        elastic::Query::Match(elastic::QueryMatch::Title(fuzzy_field(q, 5.0, "AUTO:4,6"))),
        elastic::Query::Match(elastic::QueryMatch::Body(fuzzy_field(q, 1.0, "AUTO:6,9"))),
        // No stopwords, and term frequency favors pages using the value a lot.
        elastic::Query::Match(elastic::QueryMatch::InlineCode(match_field(q, 5.0))),
        constant_term(elastic::QueryTerm::SlugLeaf(q.trim().to_lowercase()), 20.0),
    ]
}

async fn do_search(
    client: &Elasticsearch,
    params: &Params,
    subqueries: Vec<elastic::Query<'_>>,
    with_suggest: bool,
) -> Result<ElasticResponse, elasticsearch::Error> {
    let suggest =
        if !with_suggest || params.q.len() > 100 || params.q.split(' ').any(|x| x.len() > 30) {
            /*
            If it's a really long query, or a specific word is just too long, you can get those tricky
            TransportError(500, 'search_phase_execution_exception', 'Term too complex:
            errors which are hard to prevent against.
            */
            None
        } else {
            /*
            XXX research if it it's better to use phrase suggesters and if that works
            https://www.elastic.co/guide/en/elasticsearch/reference/current/search-suggesters.html#phrase-suggester
            */
            Some(elastic::Suggest {
                text: params.q.clone(),
                title_suggestions: elastic::Suggester::Term(elastic::TermSuggester {
                    field: elastic::Field::Title,
                }),
                body_suggestions: elastic::Suggester::Term(elastic::TermSuggester {
                    field: elastic::Field::Body,
                }),
            })
        };

    let subquery = elastic::Query::Bool(elastic::QueryBool {
        should: Some(subqueries),
        ..elastic::QueryBool::default()
    });

    let highlight = elastic::Highlight {
        fields: elastic::HighlightFields {
            title: json!({}),
            body: json!({}),
        },
        pre_tags: vec!["<mark>".to_string()],
        post_tags: vec!["</mark>".to_string()],
        number_of_fragments: 3,
        fragment_size: 120,
        encoder: elastic::HighlightEncoder::HTML,
    };

    let (sort, query) = match params.sort {
        Sort::Relevance => (
            Some(vec![
                elastic::SortField::Score(elastic::Order::Desc),
                elastic::SortField::Popularity(elastic::Order::Desc),
            ]),
            subquery,
        ),
        Sort::Popularity => (
            Some(vec![
                elastic::SortField::Popularity(elastic::Order::Desc),
                elastic::SortField::Score(elastic::Order::Desc),
            ]),
            subquery,
        ),
        Sort::Best => (
            None,
            elastic::Query::FunctionScore(elastic::QueryFunctionScore {
                query: &subquery,
                functions: vec![elastic::QueryFunctionScoreFunction::FieldValueFactor(
                    elastic::QueryFunctionScoreFunctionFieldValueFactor {
                        field: elastic::Field::Popularity,
                        factor: 10,
                        missing: 0,
                    },
                )],
                boost_mode: elastic::BoostMode::Sum,
                score_mode: elastic::ScoreMode::Max,
            }),
        ),
    };

    let search_body = elastic::Search {
        from: 10 * (params.page - 1),
        size: 10,
        _source: elastic::Source {
            excludes: vec![elastic::Field::Body],
        },
        sort,
        query: elastic::Query::Bool(elastic::QueryBool {
            filter: Some(vec![elastic::Query::Terms(elastic::QueryTerms::Locale(
                params.locale.clone(),
            ))]),
            must: Some(vec![query]),
            ..elastic::QueryBool::default()
        }),
        highlight,
        suggest,
    };
    debug!(
        "elastic request: {}",
        serde_json::to_string(&search_body).unwrap_or_default()
    );
    client
        .search(SearchParts::Index(&["mdn_docs"]))
        .body(search_body)
        .send()
        .await
}

async fn get_suggestion(
    suggest: elastic::ResponseSuggest,
    client: &Elasticsearch,
    locales: &[elastic::Locale],
) -> Option<Vec<Suggestion>> {
    let mut options: Vec<elastic::ResponseSuggestionOption> = suggest
        .body_suggestions
        .into_iter()
        .chain(suggest.title_suggestions)
        .flat_map(|suggestion| suggestion.options)
        .collect();
    options.sort_unstable_by(|a, b| {
        (b.score, b.freq)
            .partial_cmp(&(a.score, a.freq))
            .unwrap_or(Ordering::Equal)
    });
    for option in options {
        // Sure, this is different way to spell, but what will it yield if you actually search it?
        let count = match parse_or_get_error_reason::<elastic::CountResponse>(
            do_count(client, &option.text, locales).await,
        )
        .await
        {
            Ok(x) => x.count,
            Err(e) => {
                error!("{}", e);
                continue;
            }
        };
        if count > 0 {
            /*
            Since they're sorted by score, it's usually never useful
            to suggestion more than exactly 1 good suggestion.
            */
            return Some(vec![Suggestion {
                text: option.text,
                total: elastic::ResponseTotal {
                    value: count,
                    relation: elastic::ResponseTotalRelation::Equal,
                },
            }]);
        };
    }
    None
}

async fn do_count(
    client: &Elasticsearch,
    query: &str,
    locales: &[elastic::Locale],
) -> Result<ElasticResponse, elasticsearch::Error> {
    let body = elastic::Count {
        query: elastic::Query::Bool(elastic::QueryBool {
            filter: Some(vec![
                elastic::Query::MultiMatch(elastic::QueryMultiMatch {
                    query: query.to_string(),
                    fields: vec![elastic::Field::Title, elastic::Field::Body],
                }),
                elastic::Query::Terms(elastic::QueryTerms::Locale(locales.to_vec())),
            ]),
            ..elastic::QueryBool::default()
        }),
    };
    debug!(
        "elastic request: {}",
        serde_json::to_string(&body).unwrap_or_default()
    );
    client
        .count(CountParts::Index(&["mdn_docs"]))
        .body(body)
        .send()
        .await
}

async fn parse_or_get_error_reason<T>(
    result: Result<ElasticResponse, elasticsearch::Error>,
) -> Result<T, SearchError>
where
    T: serde::de::DeserializeOwned,
{
    let response = result?;
    match response.error_for_status_code_ref() {
        Ok(_) => {
            let text = response.text().await?;
            debug!("elastic response: {}", text);
            serde_json::from_str(&text).map_err(|_| SearchError::ParseResponse)
        }
        Err(e) => {
            let exception = response
                .exception()
                .await?
                .ok_or(SearchError::ParseResponse)?;
            debug!("{:?}", exception);
            Err(SearchError::ElasticContext {
                reason: exception
                    .error()
                    .reason()
                    .ok_or(SearchError::ParseResponse)?
                    .to_string(),
                source: e,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// Compares numbers as floats, since lab configs write `5` where serde writes `5.0`.
    fn normalize(value: Value) -> Value {
        match value {
            Value::Number(n) => json!(n.as_f64().unwrap()),
            Value::Array(a) => Value::Array(a.into_iter().map(normalize).collect()),
            Value::Object(o) => {
                Value::Object(o.into_iter().map(|(k, v)| (k, normalize(v))).collect())
            }
            x => x,
        }
    }

    // Expected bodies are generated from the search lab config (`lab/configs/final.json`).
    #[test]
    fn test_subqueries() {
        struct Case {
            name: &'static str,
            q: &'static str,
            fallback: bool,
            expected: &'static str,
        }

        let cases = [
            Case {
                name: "plain word",
                q: "flexbox",
                fallback: false,
                expected: r#"[{"match":{"title":{"query":"flexbox","boost":5}}},{"match":{"body":{"query":"flexbox","boost":1}}}]"#,
            },
            Case {
                name: "multi-word",
                q: "css grid",
                fallback: false,
                expected: r#"[{"match":{"title":{"query":"css grid","boost":5}}},{"match":{"body":{"query":"css grid","boost":1}}},{"match_phrase":{"title":{"query":"css grid","boost":10}}},{"match_phrase":{"body":{"query":"css grid","boost":2}}}]"#,
            },
            Case {
                name: "symbol",
                q: "%",
                fallback: false,
                expected: r#"[{"match":{"title":{"query":"%","boost":5}}},{"match":{"body":{"query":"%","boost":1}}},{"constant_score":{"filter":{"term":{"inline_code.exact":"%"}},"boost":10}},{"match":{"title.code":{"query":"%","boost":10}}},{"match":{"summary.code":{"query":"%","boost":5}}}]"#,
            },
            Case {
                name: "code-like",
                q: "::before",
                fallback: false,
                expected: r#"[{"match":{"title":{"query":"::before","boost":5}}},{"match":{"body":{"query":"::before","boost":1}}},{"constant_score":{"filter":{"term":{"inline_code.exact":"::before"}},"boost":10}}]"#,
            },
            Case {
                name: "camelCase",
                q: "addEventListener",
                fallback: false,
                expected: r#"[{"match":{"title":{"query":"addEventListener","boost":5}}},{"match":{"body":{"query":"addEventListener","boost":1}}},{"constant_score":{"filter":{"term":{"inline_code.exact":"addEventListener"}},"boost":10}}]"#,
            },
            Case {
                name: "code-like with space",
                q: "position: sticky",
                fallback: false,
                expected: r#"[{"match":{"title":{"query":"position: sticky","boost":5}}},{"match":{"body":{"query":"position: sticky","boost":1}}},{"constant_score":{"filter":{"term":{"inline_code.exact":"position: sticky"}},"boost":10}},{"match_phrase":{"title":{"query":"position: sticky","boost":10}}},{"match_phrase":{"body":{"query":"position: sticky","boost":2}}}]"#,
            },
            Case {
                name: "fallback",
                q: " Then",
                fallback: true,
                expected: r#"[{"match":{"title":{"query":" Then","fuzziness":"AUTO:4,6","prefix_length":1,"boost":5}}},{"match":{"body":{"query":" Then","fuzziness":"AUTO:6,9","prefix_length":1,"boost":1}}},{"match":{"inline_code":{"query":" Then","boost":5}}},{"constant_score":{"filter":{"term":{"slug_leaf":"then"}},"boost":20}}]"#,
            },
        ];

        for case in cases {
            let queries = if case.fallback {
                fallback_subqueries(case.q)
            } else {
                subqueries(case.q)
            };
            let actual = normalize(serde_json::to_value(&queries).unwrap());
            let expected = normalize(serde_json::from_str(case.expected).unwrap());
            assert_eq!(actual, expected, "case: {}", case.name);
        }
    }

    #[test]
    fn test_query_gates() {
        struct Case {
            q: &'static str,
            code_like: bool,
            symbols_only: bool,
        }

        let cases = [
            Case {
                q: "flexbox",
                code_like: false,
                symbols_only: false,
            },
            Case {
                q: "css grid",
                code_like: false,
                symbols_only: false,
            },
            Case {
                q: "HTML",
                code_like: false,
                symbols_only: false,
            },
            Case {
                q: "addEventListener",
                code_like: true,
                symbols_only: false,
            },
            Case {
                q: "max-age",
                code_like: true,
                symbols_only: false,
            },
            Case {
                q: "?.",
                code_like: true,
                symbols_only: true,
            },
            Case {
                q: "% ",
                code_like: true,
                symbols_only: false,
            },
            Case {
                q: "",
                code_like: false,
                symbols_only: false,
            },
        ];

        for case in cases {
            assert_eq!(
                is_code_like(case.q),
                case.code_like,
                "code-like: {:?}",
                case.q
            );
            assert_eq!(
                is_symbols_only(case.q),
                case.symbols_only,
                "symbols only: {:?}",
                case.q
            );
        }
    }
}
