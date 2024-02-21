use crate::helpers::app::{drop_stubr, test_app_with_login};
use crate::helpers::db::{get_pool, reset};
use crate::helpers::http_client::TestHttpClient;
use actix_web::test;
use anyhow::Error;
use async_openai::types::ChatCompletionRequestMessage;
use async_openai::types::Role::{Assistant, User};
use diesel::prelude::*;
use diesel::ExpressionMethods;
use rumba::ai::help::RefDoc;
use rumba::db::ai_help::{add_help_history, add_help_history_message};
use rumba::db::model::{AIHelpHistoryMessageInsert, SettingsInsert};
use rumba::db::schema::ai_help_history_messages;
use rumba::db::settings::create_or_update_settings;
use serde_json::Value::Null;
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
        sources: Some(serde_json::to_value(sources).unwrap_or(Null)),
        request: Some(serde_json::to_value(request).unwrap_or(Null)),
        response: Some(serde_json::to_value(response).unwrap_or(Null)),
    };
    let pool = get_pool();
    let mut conn = pool.get()?;
    add_help_history(&mut conn, 1, CHAT_ID)?;
    add_help_history_message(&mut conn, message_insert)?;
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_history() -> Result<(), Error> {
    let pool = reset()?;
    let app = test_app_with_login(&pool).await.unwrap();
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    add_history_log()?;
    let mut conn = pool.get()?;
    create_or_update_settings(
        &mut conn,
        SettingsInsert {
            user_id: 1,
            ai_help_history: Some(true),
            ..Default::default()
        },
    )?;
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
    drop_stubr(stubr).await;
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_history_message_without_parent() -> Result<(), Error> {
    let pool = reset()?;
    let app = test_app_with_login(&pool).await.unwrap();
    let service = test::init_service(app).await;
    let mut _logged_in_client = TestHttpClient::new(service).await;

    let mut conn = pool.get()?;
    // Make sure that history is enabled
    create_or_update_settings(
        &mut conn,
        SettingsInsert {
            user_id: 1,
            ai_help_history: Some(true),
            ..Default::default()
        },
    )?;

    // create a message with a non-existing parent
    let message = AIHelpHistoryMessageInsert {
        user_id: 1,
        chat_id: CHAT_ID,
        message_id: MESSAGE_ID,
        parent_id: Some(Uuid::from_u128(2)),
        created_at: None,
        sources: None,
        request: None,
        response: None,
    };
    let res = add_help_history_message(&mut conn, message);
    // We return OK, but the message should not be recorded.
    assert!(res.is_ok());
    let message_count = ai_help_history_messages::table
        .filter(ai_help_history_messages::user_id.eq(1))
        .count()
        .get_result::<i64>(&mut conn)?;
    assert_eq!(message_count, 0);
    drop_stubr(stubr).await;
    Ok(())
}

fn normalize_digits(s: &str) -> String {
    let mut result = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() {
            result.push('0');
        } else {
            result.push(c);
        }
    }

    result
}
