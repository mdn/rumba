use crate::helpers::app::{drop_stubr, test_app_with_login};
use crate::helpers::db::{get_pool, reset};
use crate::helpers::http_client::TestHttpClient;
use actix_web::test;
use anyhow::Error;
use async_openai::types::ChatCompletionRequestMessage;
use async_openai::types::Role::{Assistant, User};
use chrono::{NaiveDateTime, Utc};
use diesel::dsl::count;
use diesel::prelude::*;
use diesel::{insert_into, ExpressionMethods, RunQueryDsl};
use rumba::ai::help::RefDoc;
use rumba::db::ai_help::{add_help_history, add_help_history_message};
use rumba::db::model::{AIHelpHistoryInsert, AIHelpHistoryMessageInsert, SettingsInsert};
use rumba::db::schema::ai_help_history;
use rumba::db::settings::create_or_update_settings;
use rumba::settings::SETTINGS;
use serde_json::Value::Null;
use std::ops::Sub;
use std::time::Duration;
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
async fn test_history_deletion() -> Result<(), Error> {
    let pool = reset()?;
    let app = test_app_with_login(&pool).await.unwrap();
    let service = test::init_service(app).await;
    let mut _logged_in_client = TestHttpClient::new(&service).await;
    let mut conn = pool.get()?;

    let history_deletion_period_in_sec = &SETTINGS
        .ai
        .as_ref()
        .map(|ai| ai.history_deletion_period_in_sec)
        .expect("ai.history_deletion_period_in_sec missing");

    // Add an old chat history entry, double our configured period ago.
    let ts = Utc::now()
        .sub(Duration::from_secs(history_deletion_period_in_sec * 2))
        .naive_utc();
    let history = AIHelpHistoryInsert {
        user_id: 1,
        chat_id: Uuid::from_u128(1),
        created_at: Some(ts),
        updated_at: Some(ts),
        label: "old entry".to_string(),
    };
    insert_into(ai_help_history::table)
        .values(history)
        .execute(&mut conn)?;

    // Add a newer chat history entry, half of our configured period ago.
    let ts = Utc::now()
        // .checked_sub_months(Months::new(2))
        .sub(Duration::from_secs(history_deletion_period_in_sec / 2))
        .naive_utc();
    let history = AIHelpHistoryInsert {
        user_id: 1,
        chat_id: Uuid::from_u128(2),
        created_at: Some(ts),
        updated_at: Some(ts),
        label: "new entry".to_string(),
    };
    insert_into(ai_help_history::table)
        .values(history)
        .execute(&mut conn)?;

    // check the history count before we run the delete job
    let rec_count = ai_help_history::table
        .filter(ai_help_history::user_id.eq(1))
        .select(count(ai_help_history::user_id))
        .first::<i64>(&mut conn)?;

    assert_eq!(2, rec_count);

    // Now, run the delete job.
    let req = test::TestRequest::post()
        .uri("/admin-api/v2/ai-history/")
        .insert_header((
            "Authorization",
            format!("Bearer {}", SETTINGS.auth.admin_update_bearer_token),
        ))
        .to_request();

    let res = test::call_service(&service, req).await;
    assert!(res.response().status().is_success());

    // Check database that the old entry is gone.
    // Loop until we see the old entry is gone because the
    // delete job runs asynchonously.
    let mut retry = 0;
    const MAX_RETRIES: u32 = 10;
    let mut records: Vec<NaiveDateTime>;
    loop {
        records = ai_help_history::table
            .filter(ai_help_history::user_id.eq(1))
            .select(ai_help_history::updated_at)
            .get_results(&mut conn)?;
        if records.len() == 1 {
            break;
        }

        actix_rt::time::sleep(Duration::from_millis(10)).await;
        retry += 1;
        if retry > MAX_RETRIES {
            break;
        }
    }

    assert_eq!(1, records.len());
    assert_eq!(ts, *records.get(0).unwrap());

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
