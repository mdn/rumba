use actix_identity::Identity;
use actix_web::{web::Data, HttpResponse};
use basket::{Basket, YesNo};
use serde::{Deserialize, Serialize};

use crate::{
    api::error::ApiError,
    db::{users::get_user, Pool},
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

pub async fn subscribe(
    pool: Data<Pool>,
    user_id: Identity,
    basket: Data<Option<Basket>>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user = get_user(&mut conn_pool, user_id.id().unwrap())?;
    if let Some(basket) = &**basket {
        basket
            .subscribe_private(&user.email, vec![MDN_PLUS_LIST.into()], None)
            .await?;
        return Ok(HttpResponse::Created().json(Subscribed { subscribed: true }));
    }
    Ok(HttpResponse::NotImplemented().finish())
}

pub async fn unsubscribe(
    pool: Data<Pool>,
    user_id: Identity,
    basket: Data<Option<Basket>>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user = get_user(&mut conn_pool, user_id.id().unwrap())?;
    if let Some(basket) = &**basket {
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
        return Ok(HttpResponse::Created().json(Subscribed { subscribed: false }));
    }
    Ok(HttpResponse::NotImplemented().finish())
}

pub async fn is_subscribed(
    pool: Data<Pool>,
    user_id: Identity,
    basket: Data<Option<Basket>>,
) -> Result<HttpResponse, ApiError> {
    let mut conn_pool = pool.get()?;
    let user = get_user(&mut conn_pool, user_id.id().unwrap())?;
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
        return Ok(HttpResponse::Created().json(Subscribed { subscribed }));
    };
    Ok(HttpResponse::NotImplemented().finish())
}
