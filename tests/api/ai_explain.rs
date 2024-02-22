use crate::helpers::app::{drop_stubr, test_app_with_login};
use crate::helpers::db::{get_pool, reset};
use actix_web::test;
use anyhow::Error;
use diesel::{QueryDsl, RunQueryDsl};
use hmac::Mac;
use rumba::ai::constants::AI_EXPLAIN_VERSION;
use rumba::ai::explain::{hash_highlighted, ExplainRequest, HmacSha256};
use rumba::db::ai_explain::{add_explain_answer, ExplainFeedback};
use rumba::db::ai_help::FeedbackTyp;
use rumba::db::model::{AIExplainCacheInsert, AIExplainCacheQuery};
use rumba::db::schema::ai_explain_cache;
use rumba::settings::SETTINGS;

const JS_SAMPLE: &str = "const foo = 1;";

fn sign(language: &str, sample: &str) -> Result<Vec<u8>, Error> {
    let mut mac = HmacSha256::new_from_slice(
        &SETTINGS
            .ai
            .as_ref()
            .map(|ai| ai.explain_sign_key)
            .expect("missing sign_key"),
    )?;

    mac.update(language.as_bytes());
    mac.update(sample.as_bytes());

    Ok(mac.finalize().into_bytes().to_vec())
}

fn add_explain_cache() -> Result<(), Error> {
    let insert = AIExplainCacheInsert {
        language: Some("js".to_owned()),
        signature: sign("js", JS_SAMPLE)?,
        highlighted_hash: hash_highlighted(JS_SAMPLE),
        explanation: Some("Explain this!".to_owned()),
        version: AI_EXPLAIN_VERSION,
    };
    let pool = get_pool();
    let mut conn = pool.get()?;
    add_explain_answer(&mut conn, &insert)?;
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_explain() -> Result<(), Error> {
    let pool = reset()?;
    add_explain_cache()?;
    let app = test_app_with_login(&pool).await.unwrap();
    let service = test::init_service(app).await;
    let request = test::TestRequest::post()
        .uri("/api/v1/plus/ai/explain")
        .set_json(ExplainRequest {
            language: Some("js".to_owned()),
            sample: JS_SAMPLE.to_owned(),
            signature: sign("js", JS_SAMPLE)?,
            highlighted: Some(JS_SAMPLE.to_owned()),
        })
        .to_request();
    let explain = test::call_service(&service, request).await;

    assert!(explain.status().is_success());

    let expected = "data: {\"initial\":{\"cached\":true,\"hash\":\"nW77myAksS9XEAZpmXYHPFbW3WZTQvZLLO1cAwPTKwQ=\"}}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"Explain this!\"},\"finish_reason\":null}],\"id\":1}\n\n";
    assert_eq!(
        expected,
        String::from_utf8_lossy(test::read_body(explain).await.as_ref())
    );

    let request = test::TestRequest::post()
        .uri("/api/v1/plus/ai/explain/feedback")
        .set_json(ExplainFeedback {
            typ: FeedbackTyp::ThumbsUp,
            signature: sign("js", JS_SAMPLE)?,
            hash: hash_highlighted(JS_SAMPLE),
        })
        .to_request();
    let feedback = test::call_service(&service, request).await;
    assert!(feedback.status().is_success());

    let mut conn = pool.get()?;
    let row: AIExplainCacheQuery = ai_explain_cache::table
        .select(ai_explain_cache::all_columns)
        .first(&mut conn)?;
    assert_eq!(row.thumbs_up, 1);
    assert_eq!(row.thumbs_down, 0);
    assert_eq!(row.view_count, 2);

    let request = test::TestRequest::post()
        .uri("/api/v1/plus/ai/explain/feedback")
        .set_json(ExplainFeedback {
            typ: FeedbackTyp::ThumbsDown,
            signature: sign("js", JS_SAMPLE)?,
            hash: hash_highlighted(JS_SAMPLE),
        })
        .to_request();
    let feedback = test::call_service(&service, request).await;
    assert!(feedback.status().is_success());

    let mut conn = pool.get()?;
    let row: AIExplainCacheQuery = ai_explain_cache::table
        .select(ai_explain_cache::all_columns)
        .first(&mut conn)?;
    assert_eq!(row.thumbs_up, 1);
    assert_eq!(row.thumbs_down, 1);

    let request = test::TestRequest::post()
        .uri("/api/v1/plus/ai/explain/feedback")
        .set_json(ExplainFeedback {
            typ: FeedbackTyp::ThumbsDown,
            signature: sign("js", JS_SAMPLE)?,
            hash: hash_highlighted("foo"),
        })
        .to_request();
    let feedback = test::call_service(&service, request).await;
    assert!(feedback.status().is_success());
    let row: AIExplainCacheQuery = ai_explain_cache::table
        .select(ai_explain_cache::all_columns)
        .first(&mut conn)?;
    assert_eq!(row.thumbs_up, 1);
    assert_eq!(row.thumbs_down, 1);
    drop_stubr(stubr).await;
    Ok(())
}
