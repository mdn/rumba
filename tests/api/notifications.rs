use crate::helpers::app::test_app_with_login;
use crate::helpers::db::{get_pool, reset};
use crate::helpers::http_client::{TestHttpClient};
use crate::helpers::read_json;
use actix_web::test;
use anyhow::Error;
use rumba::db;
use rumba::db::model::{DocumentMetadata, NotificationDataInsert, NotificationInsert};


#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_get_notifications() -> Result<(), Error> {
    reset()?;

    let app = test_app_with_login().await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;

    //Database was reset so we can naively assume user_id = 1.
    let _ids = create_notifications(1, 100).await;
    let mut offset = 0;
    let mut limit = 10;

    let res = logged_in_client.get(format!("/api/v1/notifications/?offset={}&limit={}", offset,limit), None).await;
    assert_eq!(res.response().status(), 200);

    let res_json = read_json(res).await;
    assert_eq!(res_json["items"].as_array().unwrap().len(),10);
    

    Ok(())
}
#[actix_rt::test]
async fn test_mark_all_read() -> Result<(), Error> {
    Ok(())
}
#[actix_rt::test]
async fn test_unstar_many() -> Result<(), Error> {
    Ok(())
}

#[actix_rt::test]
async fn test_star_many() -> Result<(), Error> {
    reset()?;
    Ok(())
}

async fn create_notifications(user_id: i64, number: usize) -> Vec<i64> {
    let conn_pool = get_pool();
    let mut notification_ids: Vec<i64>= vec![];
    for i in 1..number {
        let uri = format!("{}/{}", "https://developer.allizom.org/", i);
        let document = DocumentMetadata {
            parents: None,
            mdn_url: uri.to_string(),
            title: format!("{} {}", "Test", i),
        };

        let document_id = db::documents::create_or_update_document(
            &mut conn_pool.get().unwrap(),
            document,
            uri.to_string(),
        )
        .await;

        let data = NotificationDataInsert {
            text: format!("Test text {}", i),
            url: uri.to_string(),
            data: None,
            title: format!("Test title {}", i),
            type_: if i % 2 == 0 {
                db::types::NotificationTypeEnum::Compat
            } else {
                db::types::NotificationTypeEnum::Content
            },
            document_id: document_id.unwrap(),
        };

        let notification_data_id =
            db::notifications::create_notification_data(&mut conn_pool.get().unwrap(), data)
                .await
                .unwrap();
        let notification_insert = NotificationInsert {
            deleted_at: None,
            read: false,
            starred: false,
            notification_data_id: notification_data_id,
            user_id: user_id,
        };

        let id = db::notifications::create_notification(
            &mut conn_pool.get().unwrap(),
            user_id,
            notification_data_id,
        ).await.unwrap();
        notification_ids.push(id);
    }
    notification_ids
}
