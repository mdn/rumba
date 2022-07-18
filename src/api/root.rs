use actix_web::{
    dev::HttpServiceFactory,
    web::{self, Data},
};
use maud::{html, Markup, Render};
use serde::Deserialize;

use crate::{
    api::{error::ApiError, user_middleware::UserId},
    db::{
        model::UserQuery,
        types::Subscription,
        users::{find_user_by_email, get_user, root_update_user},
        Pool,
    },
    helpers::{deserialize_checkbox, deserialize_enforce_plus},
};

impl Render for Subscription {
    fn render(&self) -> maud::Markup {
        html! {
            (self.as_str())
        }
    }
}

impl Render for UserQuery {
    fn render(&self) -> maud::Markup {
        html! {
            form method="post" style="display:flex;gap:1rem;" {
                span {
                    (self.email)
                }
                span{
                    (self.fxa_uid)
                }
                span {
                    "enforce_plus: " (select_enforce_plus(self.enforce_plus))
                }
                span {
                    "is_admin " input type="checkbox" name="is_admin" checked[self.is_admin];
                }
                input type="hidden" name="fxa_uid" value=(self.fxa_uid);
                input type="submit" value="Save";
            }
        }
    }
}

fn subscription_to_option(
    subscription: Option<Subscription>,
    current: Option<Subscription>,
) -> Markup {
    let selected = subscription == current;
    if let Some(subscription) = subscription {
        html! {
            option selected[selected] value=(subscription.as_str()) { (subscription) }
        }
    } else {
        html! {
            option selected[selected] value { "-" }
        }
    }
}

fn select_enforce_plus(current: Option<Subscription>) -> Markup {
    html! {
        select name="enforce_plus" {
            option {
                (subscription_to_option(None, current))
                (subscription_to_option(Some(Subscription::Core), current))
                (subscription_to_option(Some(Subscription::MdnPlus_5m), current))
                (subscription_to_option(Some(Subscription::MdnPlus_5y), current))
                (subscription_to_option(Some(Subscription::MdnPlus_10m), current))
                (subscription_to_option(Some(Subscription::MdnPlus_10y), current))
            }
        }
    }
}

#[derive(Deserialize)]
pub struct RootQuery {
    email: Option<String>,
}

#[derive(Deserialize)]
pub struct RootUserUpdateQuery {
    pub fxa_uid: String,
    #[serde(deserialize_with = "deserialize_enforce_plus")]
    pub enforce_plus: Option<Subscription>,
    #[serde(default, deserialize_with = "deserialize_checkbox")]
    pub is_admin: bool,
}

async fn update_user(
    pool: Data<Pool>,
    query: web::Form<RootUserUpdateQuery>,
    user_id: UserId,
) -> Result<Markup, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id)?;
    if !me.is_admin {
        return Err(ApiError::Unauthorized);
    }
    let res = root_update_user(&mut conn_pool, query.into_inner());
    Ok(html! {
        html {
            meta http-equiv="Content-Security-Policy" content="default-src 'self';"
            body {
                h1 { "🤖 Rumba Root 🪄"}
                section {
                    @if let Err(e) = res {
                        (e.to_string())
                    } @else {
                        "Success 🎉"
                    }
                }
            }
        }
    })
}

async fn index(
    pool: Data<Pool>,
    query: web::Query<RootQuery>,
    user_id: UserId,
) -> Result<Markup, ApiError> {
    let mut conn_pool = pool.get()?;
    let me: UserQuery = get_user(&mut conn_pool, user_id.id)?;
    if !me.is_admin {
        return Err(ApiError::Unauthorized);
    }
    let user = if let Some(ref email) = query.email {
        find_user_by_email(&mut conn_pool, email)?
    } else {
        None
    };
    Ok(html! {
        html {
            meta http-equiv="Content-Security-Policy" content="default-src 'self'; style-src 'unsafe-inline';";
            body {
                h1 { "🤖 Rumba Root 🪄"}
                section {
                    h2 { "Modify user"}
                    @if let Some(user) = user {
                        (user)
                    } @else if let Some(ref email) = query.email {
                        "no user for email " (email)
                    }
                    form method="get" {
                        input placeholder="enter an email" type="email" name="email";
                        input type="submit" value="get";
                    }
                }
            }
        }
    })
}

pub fn root_service() -> impl HttpServiceFactory {
    web::scope("/root").service(
        web::resource("/")
            .route(web::get().to(index))
            .route(web::post().to(update_user)),
    )
}
