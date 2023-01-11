use actix_identity::Identity;
use actix_web::{web::Data, HttpResponse};
use basket::{Basket, YesNo};
use diesel::PgConnection;
use serde::{Deserialize, Serialize};

use crate::{
    api::{error::ApiError, settings::SettingUpdateRequest},
    db::{self, model::UserQuery, users::get_user, Pool},
};

const MDN_PLUS_LIST: &str = "mdnplus";

#[derive(Deserialize, Serialize)]
struct UserLookup {
    email: String,
    newsletters: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct Subscribed {
    pub subscribed: bool,
}

pub async fn subscribe_handler(
    pool: Data<Pool>,
    user_id: Identity,
    basket: Data<Option<Basket>>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    if let Some(basket) = &**basket {
        return subscribe(&mut conn, &user, basket).await;
    }
    Ok(HttpResponse::NotImplemented().finish())
}

pub async fn subscribe(
    conn: &mut PgConnection,
    user: &UserQuery,
    basket: &Basket,
) -> Result<HttpResponse, ApiError> {
    basket
        .subscribe_private(&user.email, vec![MDN_PLUS_LIST.into()], None)
        .await?;
    db::settings::create_or_update_settings(
        conn,
        user,
        SettingUpdateRequest {
            mdnplus_newsletter: Some(true),
            ..Default::default()
        },
    )?;
    Ok(HttpResponse::Created().json(Subscribed { subscribed: true }))
}

pub async fn unsubscribe_handler(
    pool: Data<Pool>,
    user_id: Identity,
    basket: Data<Option<Basket>>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    if let Some(basket) = &**basket {
        return unsubscribe(&mut conn, &user, basket).await;
    }
    Ok(HttpResponse::NotImplemented().finish())
}

pub async fn unsubscribe(
    conn: &mut PgConnection,
    user: &UserQuery,
    basket: &Basket,
) -> Result<HttpResponse, ApiError> {
    let value = basket.lookup_user(&user.email).await;
    let token = match &value {
        Ok(j) if j["token"].is_string() => j["token"].as_str().unwrap_or_default(),
        Ok(_) => {
            error!("Invalid JSON when retrieving token for {}", &user.email);
            return Err(ApiError::JsonProcessingError);
        }
        Err(_) => return Ok(HttpResponse::NotFound().finish()),
    };
    basket
        .unsubscribe(token, vec![MDN_PLUS_LIST.into()], YesNo::N)
        .await?;
    db::settings::create_or_update_settings(
        conn,
        user,
        SettingUpdateRequest {
            mdnplus_newsletter: Some(false),
            ..Default::default()
        },
    )?;
    Ok(HttpResponse::Created().json(Subscribed { subscribed: false }))
}

pub async fn is_subscribed(
    pool: Data<Pool>,
    user_id: Identity,
    basket: Data<Option<Basket>>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = pool.get()?;
    let user = get_user(&mut conn, user_id.id().unwrap())?;
    if let Some(basket) = &**basket {
        let value = basket.lookup_user(&user.email).await;
        let subscribed = match value {
            Ok(value) => {
                let basket_user: UserLookup = serde_json::from_value(value)?;
                basket_user.email == user.email
                    && basket_user.newsletters.contains(&MDN_PLUS_LIST.to_string())
            }
            Err(_) => false,
        };
        let settings = db::settings::get_settings(&mut conn, &user)?;
        if subscribed != settings.map(|s| s.mdnplus_newsletter).unwrap_or_default() {
            db::settings::create_or_update_settings(
                &mut conn,
                &user,
                SettingUpdateRequest {
                    mdnplus_newsletter: Some(subscribed),
                    ..Default::default()
                },
            )?;
        }
        return Ok(HttpResponse::Created().json(Subscribed { subscribed }));
    };
    Ok(HttpResponse::NotImplemented().finish())
}
