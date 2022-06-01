use crate::api::elastic;
use crate::api::error::{ApiError, SearchError};
use actix_web::{web, HttpRequest, HttpResponse};
use elasticsearch::{CountParts, Elasticsearch, SearchParts};
use elasticsearch::http::response::Response as ElasticResponse;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;

// TODO: add caching headers from kuma
// TODO: add retry logic from kuma
// TODO: tests

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

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Sort {
    Best,
    Relevance,
    Popularity,
}

impl Default for Sort {
    fn default() -> Self {
        Sort::Best
    }
}

#[derive(Deserialize)]
struct Params {
    q: String,
    #[serde(default)]
    sort: Sort,
    #[serde(default = "default_size")]
    size: i64,
    #[serde(default = "default_page")]
    page: i64,
    #[serde(skip)]
    locale: Vec<elastic::Locale>,
}

fn default_size() -> i64 {
    10
}

fn default_page() -> i64 {
    1
}

pub async fn search(
    request: HttpRequest,
    client: web::Data<Elasticsearch>,
) -> Result<HttpResponse, ApiError> {
    let mut params = web::Query::<Params>::from_query(request.query_string())?.into_inner();
    params.locale = web::Query::<Vec<(String, String)>>::from_query(request.query_string())
        .unwrap_or_else(|_| web::Query(vec![("locale".to_string(), "en-US".to_string())]))
        .iter()
        .filter_map(|(key, value)| {
            if key == "locale" {
                ron::from_str(&value.to_lowercase()).ok()
            } else {
                None
            }
        })
        .collect();

    let search_response: elastic::SearchResponse =
        match parse_or_get_error_reason(do_search(&client, &params).await).await {
            Ok(x) => x,
            Err(e) => {
                error!("{}", e);
                return Err(e.into());
            }
        };

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
            page: 1,
        },
        suggestions: match search_response.suggest {
            Some(x) => get_suggestion(x, &client, &params.locale)
                .await
                .unwrap_or_default(),
            None => vec![],
        },
    };
    Ok(HttpResponse::Ok().json(response))
}

async fn do_search(
    client: &Elasticsearch,
    params: &Params,
) -> Result<ElasticResponse, elasticsearch::Error> {
    let suggest = if params.q.len() > 100 || params.q.split(' ').any(|x| x.len() > 30) {
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

    /*
    The business logic here that we search for things different ways,
    and each different way as a different boost which dictates its importance.
    The importance order is as follows:

     1. Title match-phrase
     2. Title match
     3. Body match-phrase
     4. Body match

    The order is determined by the `boost` number in the code below.
    Remember that sort order is a combination of "match" and popularity, but
    ideally the popularity should complement. Try to get a pretty good
    sort by pure relevance first, and let popularity just make it better.
    */
    let mut subqueries: Vec<elastic::Query> = vec![
        elastic::Query::Match(elastic::QueryMatch::Title(elastic::QueryMatchField {
            query: params.q.clone(),
            boost: 5.0,
        })),
        elastic::Query::Match(elastic::QueryMatch::Body(elastic::QueryMatchField {
            query: params.q.clone(),
            boost: 1.0,
        })),
    ];
    if params.q.contains(' ') {
        subqueries.push(elastic::Query::MatchPhrase(elastic::QueryMatch::Title(
            elastic::QueryMatchField {
                query: params.q.clone(),
                boost: 10.0,
            },
        )));
        subqueries.push(elastic::Query::MatchPhrase(elastic::QueryMatch::Body(
            elastic::QueryMatchField {
                query: params.q.clone(),
                boost: 2.0,
            },
        )));
    }

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
            vec![
                elastic::SortField::Score(elastic::Order::Desc),
                elastic::SortField::Popularity(elastic::Order::Desc),
            ],
            subquery,
        ),
        Sort::Popularity => (
            vec![
                elastic::SortField::Popularity(elastic::Order::Desc),
                elastic::SortField::Score(elastic::Order::Desc),
            ],
            subquery,
        ),
        Sort::Best => (
            vec![],
            elastic::Query::FunctionScore(elastic::QueryFunctionScore {
                query: Box::new(subquery),
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
        from: params.size * (params.page - 1),
        size: params.size * params.page,
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
    debug!("elastic request: {}", serde_json::to_string(&search_body)?);
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
    debug!("elastic request: {}", serde_json::to_string(&body)?);
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
