use crate::helpers::app::test_app_with_login;
use crate::helpers::db::{get_pool, reset};
use crate::helpers::http_client::{PostPayload, TestHttpClient};
use crate::helpers::{read_json, wait_for_stubr};
use actix_web::test;
use anyhow::Error;
use diesel::{QueryDsl, RunQueryDsl};
use rumba::ai::constants::ASK_DEFAULT;
use rumba::api::root::RootSetIsAdminQuery;
use rumba::db::ai_help::{add_help_log, AIHelpFeedback, FeedbackTyp};
use rumba::db::model::{AIHelpLogs, AIHelpLogsInsert};
use rumba::db::schema::ai_help_logs;
use rumba::db::users::root_set_is_admin;
use serde_json::json;
use uuid::Uuid;

const CHAT_ID: Uuid = Uuid::nil();

fn add_history_log() -> Result<(), Error> {
    let insert = AIHelpLogsInsert {
        user_id: 1,
        variant: ASK_DEFAULT.name.to_string(),
        chat_id: CHAT_ID,
        message_id: 1,
        created_at: None,
        request: json!({
            "model": "gpt-3.5-turbo",
            "messages": [{"role": "user", "content": "How to center a div with CSS?"}],
            "max_tokens": null,
            "temperature": 0.0
        }),
        response: json!({
            "meta": {
                "type": "metadata",
                "quota": null,
                "chat_id": "00000000-0000-0000-0000-000000000000",
                "sources": [{
                        "url": "/en-US/docs/Learn/CSS/Howto/Center_an_item",
                        "slug": "Learn/CSS/Howto/Center_an_item",
                        "title": "How to center an item"
                    }, {
                        "url": "/en-US/docs/Web/CSS/margin",
                        "slug": "Web/CSS/margin",
                        "title": "margin"
                    }, {
                        "url": "/en-US/docs/Web/CSS/CSS_grid_layout/Box_alignment_in_grid_layout",
                        "slug": "Web/CSS/CSS_grid_layout/Box_alignment_in_grid_layout",
                        "title": "Box alignment in grid layout"
                    }],
                "message_id": 1
                },
            "answer": {
                "role": "assistant",
                "content": "To center a div using CSS, ..."}
            }
        ),
    };
    let pool = get_pool();
    let mut conn = pool.get()?;
    add_help_log(&mut conn, &insert)?;
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
            "/api/v1/plus/ai/ask/history/00000000-0000-0000-0000-000000000000",
            None,
        )
        .await;
    assert!(history.status().is_success());
    let expected = r#"{"chat_id":"00000000-0000-0000-0000-000000000000","messages":[{"metadata":{"type":"metadata","chat_id":"00000000-0000-0000-0000-000000000000","message_id":1,"sources":[{"url":"/en-US/docs/Learn/CSS/Howto/Center_an_item","slug":"Learn/CSS/Howto/Center_an_item","title":"How to center an item"},{"url":"/en-US/docs/Web/CSS/margin","slug":"Web/CSS/margin","title":"margin"},{"url":"/en-US/docs/Web/CSS/CSS_grid_layout/Box_alignment_in_grid_layout","slug":"Web/CSS/CSS_grid_layout/Box_alignment_in_grid_layout","title":"Box alignment in grid layout"}],"quota":null},"user":{"role":"user","content":"How to center a div with CSS?"},"assistant":{"role":"assistant","content":"To center a div using CSS, ..."}}]}"#;

    assert_eq!(
        expected,
        String::from_utf8_lossy(test::read_body(history).await.as_ref())
    );

    let feedback = logged_in_client
        .post(
            "/api/v1/plus/ai/ask/feedback",
            None,
            Some(PostPayload::Json(serde_json::to_value(AIHelpFeedback {
                thumbs: Some(FeedbackTyp::ThumbsUp),
                chat_id: CHAT_ID,
                message_id: 1,
                feedback: None,
            })?)),
        )
        .await;
    assert!(feedback.status().is_success());

    let mut conn = pool.get()?;
    let row: AIHelpLogs = ai_help_logs::table.first(&mut conn)?;
    assert_eq!(row.thumbs, Some(true));

    let feedback = logged_in_client
        .post(
            "/api/v1/plus/ai/ask/feedback",
            None,
            Some(PostPayload::Json(serde_json::to_value(AIHelpFeedback {
                thumbs: Some(FeedbackTyp::ThumbsDown),
                chat_id: CHAT_ID,
                message_id: 1,
                feedback: None,
            })?)),
        )
        .await;
    assert!(feedback.status().is_success());

    let mut conn = pool.get()?;
    let row: AIHelpLogs = ai_help_logs::table
        .select(ai_help_logs::all_columns)
        .first(&mut conn)?;
    assert_eq!(row.thumbs, Some(false));

    drop(stubr);
    Ok(())
}
