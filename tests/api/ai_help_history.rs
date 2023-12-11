use crate::helpers::app::test_app_with_login;
use crate::helpers::db::{get_pool, reset};
use crate::helpers::http_client::{PostPayload, TestHttpClient};
use crate::helpers::{read_json, wait_for_stubr};
use actix_web::test;
use anyhow::Error;
use async_openai::types::ChatCompletionRequestMessage;
use async_openai::types::Role::{Assistant, User};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use rumba::ai::constants::AI_HELP_DEFAULT;
use rumba::ai::help::RefDoc;
use rumba::api::root::RootSetIsAdminQuery;
use rumba::db::ai_help::{
    add_help_debug_log, add_help_history, add_help_history_message, AIHelpFeedback, FeedbackTyp,
};
use rumba::db::model::{AIHelpDebugLogsInsert, AIHelpHistoryMessageInsert};
use rumba::db::schema::{ai_help_debug_feedback, ai_help_feedback};
use rumba::db::users::root_set_is_admin;
use serde_json::{json, Value::Null};
use uuid::Uuid;

const CHAT_ID: Uuid = Uuid::nil();
const MESSAGE_ID: Uuid = Uuid::from_u128(1);

fn add_history_log() -> Result<(), Error> {
    let request = ChatCompletionRequestMessage {
        role: User,
        content: Some("How to center a div with CSS?".into()),
        name: None,
        function_call: None,
    };
    let response = ChatCompletionRequestMessage {
        role: Assistant,
        content: Some("To center a div using CSS, ...".into()),
        name: None,
        function_call: None,
    };
    let sources = vec![
        RefDoc {
            url: "/en-US/docs/Learn/CSS/Howto/Center_an_item".into(),
            title: "How to center an item".into(),
        },
        RefDoc {
            url: "/en-US/docs/Web/CSS/margin".into(),
            title: "margin".into(),
        },
        RefDoc {
            url: "/en-US/docs/Web/CSS/CSS_grid_layout/Box_alignment_in_grid_layout".into(),
            title: "Box alignment in grid layout".into(),
        },
    ];
    let message_insert = AIHelpHistoryMessageInsert {
        user_id: 1,
        chat_id: CHAT_ID,
        message_id: MESSAGE_ID,
        parent_id: None,
        created_at: None,
        sources: Some(serde_json::to_value(&sources).unwrap_or(Null)),
        request: Some(serde_json::to_value(&request).unwrap_or(Null)),
        response: Some(serde_json::to_value(&response).unwrap_or(Null)),
    };
    let debug_insert = AIHelpDebugLogsInsert {
        user_id: 1,
        chat_id: CHAT_ID,
        message_id: MESSAGE_ID,
        parent_id: None,
        sources: serde_json::to_value(&sources)?,
        created_at: None,
        request: serde_json::to_value(Some(&request))?,
        response: serde_json::to_value(&response)?,
        variant: AI_HELP_DEFAULT.name.to_string(),
    };
    let pool = get_pool();
    let mut conn = pool.get()?;
    add_help_history(&mut conn, 1, CHAT_ID)?;
    add_help_history_message(&mut conn, message_insert)?;
    add_help_debug_log(&mut conn, &debug_insert)?;
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_history() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await.unwrap();
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    add_history_log()?;
    let mut conn = pool.get()?;
    root_set_is_admin(
        &mut conn,
        RootSetIsAdminQuery {
            fxa_uid: "TEST_SUB".into(),
            is_admin: true,
        },
    )?;
    let experiments = logged_in_client
        .post(
            "/api/v1/plus/settings/experiments/",
            None,
            Some(crate::helpers::http_client::PostPayload::Json(json!({
                "active": true,
            }))),
        )
        .await;
    assert_eq!(experiments.status(), 201);
    let json = read_json(experiments).await;
    assert_eq!(json["active"], true);
    let history = logged_in_client
        .get(
            "/api/v1/plus/ai/help/history/00000000-0000-0000-0000-000000000000",
            None,
        )
        .await;
    assert!(history.status().is_success());
    let expected = r#"{"chat_id":"00000000-0000-0000-0000-000000000000","messages":[{"metadata":{"type":"metadata","chat_id":"00000000-0000-0000-0000-000000000000","message_id":"00000000-0000-0000-0000-000000000000","parent_id":null,"sources":[{"url":"/en-US/docs/Learn/CSS/Howto/Center_an_item","title":"How to center an item"},{"url":"/en-US/docs/Web/CSS/margin","title":"margin"},{"url":"/en-US/docs/Web/CSS/CSS_grid_layout/Box_alignment_in_grid_layout","title":"Box alignment in grid layout"}],"quota":null,"created_at":"0000-00-00T00:00:00.000000Z"},"user":{"role":"user","content":"How to center a div with CSS?"},"assistant":{"role":"assistant","content":"To center a div using CSS, ..."}}]}"#;

    assert_eq!(
        expected,
        normalize_digits(&String::from_utf8_lossy(
            test::read_body(history).await.as_ref()
        ))
    );

    let feedback = logged_in_client
        .post(
            "/api/v1/plus/ai/help/feedback",
            None,
            Some(PostPayload::Json(serde_json::to_value(AIHelpFeedback {
                thumbs: Some(FeedbackTyp::ThumbsUp),
                message_id: MESSAGE_ID,
                feedback: None,
            })?)),
        )
        .await;
    assert!(feedback.status().is_success());

    let mut conn = pool.get()?;
    let thumbs = ai_help_feedback::table
        .select(ai_help_feedback::thumbs)
        .order_by(ai_help_feedback::created_at.desc())
        .first::<Option<bool>>(&mut conn)?;
    assert_eq!(thumbs, Some(true));
    let thumbs = ai_help_debug_feedback::table
        .select(ai_help_debug_feedback::thumbs)
        .order_by(ai_help_debug_feedback::created_at.desc())
        .first::<Option<bool>>(&mut conn)?;
    assert_eq!(thumbs, Some(true));

    let feedback = logged_in_client
        .post(
            "/api/v1/plus/ai/help/feedback",
            None,
            Some(PostPayload::Json(serde_json::to_value(AIHelpFeedback {
                thumbs: Some(FeedbackTyp::ThumbsDown),
                message_id: MESSAGE_ID,
                feedback: None,
            })?)),
        )
        .await;
    assert!(feedback.status().is_success());

    let mut conn = pool.get()?;
    let thumbs = ai_help_feedback::table
        .select(ai_help_feedback::thumbs)
        .order_by(ai_help_feedback::created_at.desc())
        .first::<Option<bool>>(&mut conn)?;
    assert_eq!(thumbs, Some(false));
    let thumbs = ai_help_debug_feedback::table
        .select(ai_help_debug_feedback::thumbs)
        .order_by(ai_help_debug_feedback::created_at.desc())
        .first::<Option<bool>>(&mut conn)?;
    assert_eq!(thumbs, Some(false));

    drop(stubr);
    Ok(())
}

fn normalize_digits(s: &str) -> String {
    let mut result = String::new();

    for c in s.chars() {
        if c.is_digit(10) {
            result.push('0');
        } else {
            result.push(c);
        }
    }

    result
}
