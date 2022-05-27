use crate::api::elastic;
use crate::api::error::ApiError;
use actix_web::{web, HttpRequest, HttpResponse};
use elasticsearch::http::transport::Transport;
use elasticsearch::{CountParts, Elasticsearch, SearchParts};
use serde::{Deserialize, Serialize};
use serde_json::json;

// TODO: log errors appropriately?
// TODO: copy elastic-implementation comments from kuma for future reference
// TODO: sort out struct naming, use of use, etc.
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
    locale: String,
    slug: String,
    popularity: f64,
    summary: String,
    highlight: elastic::ResponseHighlight,
}

#[derive(Clone, Deserialize)]
struct Params {
    q: String,
    #[serde(default)]
    sort: String,
    #[serde(default = "default_size")]
    size: i64,
    #[serde(default = "default_page")]
    page: i64,
    #[serde(skip)]
    locale: Vec<String>,
}

fn default_size() -> i64 {
    10
}

fn default_page() -> i64 {
    1
}

pub async fn search(request: HttpRequest) -> Result<HttpResponse, ApiError> {
    // TODO: share transport/client across requests?
    // TODO: get settings from toml, figure out authentication
    let transport = Transport::single_node("http://elastic:9200")?;
    let client = Elasticsearch::new(transport);

    let mut params = web::Query::<Params>::from_query(request.query_string())?.into_inner();
    params.locale = web::Query::<Vec<(String, String)>>::from_query(request.query_string())
        .unwrap_or_else(|_| web::Query(vec![("locale".to_string(), "en-US".to_string())]))
        .iter()
        .filter_map(|(key, value)| {
            if key == "locale" {
                Some(value.to_lowercase())
            } else {
                None
            }
        })
        .collect();

    let search_response = do_search(client.clone(), params.clone()).await?;

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
        suggestions: get_suggestion(search_response.suggest, client, params.locale)
            .await
            .unwrap_or_default(),
    };
    Ok(HttpResponse::Ok().json(response))
}

async fn do_search(
    client: Elasticsearch,
    params: Params,
) -> Result<elastic::SearchResponse, elasticsearch::Error> {
    let suggest = if params.q.len() > 100 || params.q.split(' ').any(|x| x.len() > 30) {
        elastic::Suggest {
            text: None,
            title_suggestions: None,
            body_suggestions: None,
        }
    } else {
        elastic::Suggest {
            text: Some(params.q.clone()),
            title_suggestions: Some(elastic::SuggestBody {
                term: elastic::SuggestTerm {
                    field: "title".to_string(),
                },
            }),
            body_suggestions: Some(elastic::SuggestBody {
                term: elastic::SuggestTerm {
                    field: "body".to_string(),
                },
            }),
        }
    };

    let mut subqueries: Vec<elastic::Query> = vec![
        elastic::QueryMatch::new_query(
            "title",
            elastic::QueryMatchField {
                query: params.q.clone(),
                boost: 5.0,
            },
        ),
        elastic::QueryMatch::new_query(
            "body",
            elastic::QueryMatchField {
                query: params.q.clone(),
                boost: 1.0,
            },
        ),
    ];
    if params.q.contains(' ') {
        subqueries.push(elastic::QueryMatchPhrase::new_query(
            "title",
            elastic::QueryMatchField {
                query: params.q.clone(),
                boost: 10.0,
            },
        ));
        subqueries.push(elastic::QueryMatchPhrase::new_query(
            "body",
            elastic::QueryMatchField {
                query: params.q.clone(),
                boost: 2.0,
            },
        ));
    }

    let subquery = elastic::QueryBool::new_query("should", subqueries);

    let highlight = elastic::Highlight {
        fields: elastic::HighlightFields {
            title: json!({}),
            body: json!({}),
        },
        pre_tags: vec!["<mark>".to_string()],
        post_tags: vec!["</mark>".to_string()],
        number_of_fragments: 3,
        fragment_size: 120,
        encoder: "html".to_string(),
    };

    let (sort, query) = match params.sort.as_str() {
        "relevance" => (
            vec![
                elastic::SortField::new("_score", "desc"),
                elastic::SortField::new("popularity", "desc"),
            ],
            subquery,
        ),
        "popularity" => (
            vec![
                elastic::SortField::new("popularity", "desc"),
                elastic::SortField::new("_score", "desc"),
            ],
            subquery,
        ),
        _ => (
            vec![],
            elastic::Query::QueryFunctionScore(elastic::QueryFunctionScore {
                function_score: elastic::QueryFunctionScoreBody {
                    query: Box::new(subquery),
                    functions: vec![elastic::QueryFunctionScoreFunction {
                        field_value_factor: elastic::QueryFunctionScoreFunctionBody {
                            field: "popularity".to_string(),
                            factor: 10,
                            missing: 0,
                        },
                    }],
                    boost_mode: "sum".to_string(),
                    score_mode: "max".to_string(),
                },
            }),
        ),
    };

    let search_body = elastic::Search {
        from: params.size * (params.page - 1),
        size: params.size * params.page,
        _source: elastic::Source {
            excludes: vec!["body".to_string()],
        },
        sort,
        query: elastic::Query::QueryBool(elastic::QueryBool {
            bool: elastic::QueryBoolBody {
                filter: Some(vec![elastic::Query::QueryTerms(elastic::QueryTerms {
                    terms: elastic::QueryTermsBody {
                        locale: params.locale,
                    },
                })]),
                must: Some(vec![query]),
                should: None,
            },
        }),
        highlight,
        suggest,
    };
    // println!("{}", serde_json::to_string(&search_body).unwrap());
    client
        .search(SearchParts::Index(&["mdn_docs"]))
        .body(search_body)
        .send()
        .await?
        .json::<elastic::SearchResponse>()
        .await
}

async fn get_suggestion(
    suggest: elastic::ResponseSuggest,
    client: Elasticsearch,
    locales: Vec<String>,
) -> Option<Vec<Suggestion>> {
    let mut options: Vec<elastic::ResponseSuggestionOption> = suggest
        .body_suggestions
        .into_iter()
        .chain(suggest.title_suggestions)
        .flat_map(|suggestion| suggestion.options)
        .collect();
    // TODO: no idea how to unwrap properly here
    options.sort_unstable_by(|a, b| (b.score, b.freq).partial_cmp(&(a.score, a.freq)).unwrap());
    for option in options {
        let (text, count) =
            match do_count(client.clone(), option.text.clone(), locales.clone()).await {
                Ok(x) => x,
                Err(_) => {
                    continue;
                }
            };
        if count > 0 {
            return Some(vec![Suggestion {
                text,
                total: elastic::ResponseTotal {
                    value: count,
                    relation: "eq".to_string(),
                },
            }]);
        };
    }
    None
}

async fn do_count(
    client: Elasticsearch,
    query: String,
    locales: Vec<String>,
) -> Result<(String, u64), elasticsearch::Error> {
    Ok((
        query.clone(),
        client
            .count(CountParts::Index(&["mdn_docs"]))
            .body(elastic::Count {
                query: elastic::QueryBool::new_query(
                    "filter",
                    vec![
                        elastic::Query::QueryMultiMatch(elastic::QueryMultiMatch {
                            multi_match: elastic::QueryMultiMatchBody {
                                query,
                                fields: vec!["title".to_string(), "body".to_string()],
                            },
                        }),
                        elastic::Query::QueryTerms(elastic::QueryTerms {
                            terms: elastic::QueryTermsBody { locale: locales },
                        }),
                    ],
                ),
            })
            .send()
            .await?
            .json::<elastic::CountResponse>()
            .await?
            .count,
    ))
}
