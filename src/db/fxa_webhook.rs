use crate::api::fxa_webhook::{ProfileChange, SubscriptionStateChange};
use crate::api::newsletter;
use crate::db::error::DbError;
use crate::db::model::{
    RawWebHookEventsTokenInsert, SettingsInsert, UserQuery, WebHookEventInsert,
};
use crate::db::settings::create_or_update_settings;
use crate::db::types::FxaEvent;
use crate::db::users::get_user_opt;
use crate::db::{schema, Pool};
use crate::fxa::{self, LoginManager};
use actix_rt::ArbiterHandle;
use actix_web::web;
use basket::Basket;
use chrono::{DateTime, Utc};
use diesel::insert_into;
use diesel::prelude::*;
use diesel::ExpressionMethods;
use serde_json::json;

use super::model::SubscriptionChangeInsert;
use super::types::{FxaEventStatus, Subscription};

pub fn log_failed_webhook_event(
    pool: web::Data<Pool>,
    token: &str,
    error: &str,
) -> Result<(), DbError> {
    let mut conn = pool.get()?;
    insert_into(schema::raw_webhook_events_tokens::table)
        .values(RawWebHookEventsTokenInsert {
            token: token.to_string(),
            error: error.to_string(),
        })
        .execute(&mut conn)?;
    Ok(())
}

pub fn delete_profile_from_webhook(
    pool: web::Data<Pool>,
    fxa_uid: String,
    issue_time: DateTime<Utc>,
) -> Result<(), DbError> {
    let fxa_event = WebHookEventInsert {
        fxa_uid: fxa_uid.clone(),
        change_time: None,
        issue_time: issue_time.naive_utc(),
        typ: FxaEvent::DeleteUser,
        status: FxaEventStatus::Pending,
        payload: json!({}),
    };
    let mut conn = pool.get()?;
    let id = insert_into(schema::webhook_events::table)
        .values(fxa_event)
        .returning(schema::webhook_events::id)
        .get_result::<i64>(&mut conn)?;
    match diesel::delete(schema::users::table.filter(schema::users::fxa_uid.eq(&fxa_uid)))
        .execute(&mut conn)
    {
        Ok(_) => {
            diesel::update(schema::webhook_events::table.filter(schema::webhook_events::id.eq(id)))
                .set(schema::webhook_events::status.eq(FxaEventStatus::Processed))
                .execute(&mut conn)?;
            Ok(())
        }
        Err(e) => {
            diesel::update(schema::webhook_events::table.filter(schema::webhook_events::id.eq(id)))
                .set(schema::webhook_events::status.eq(FxaEventStatus::Failed))
                .execute(&mut conn)?;
            Err(e.into())
        }
    }
}

pub async fn update_profile(
    pool: web::Data<Pool>,
    id: i64,
    user: UserQuery,
    login_manager: web::Data<LoginManager>,
) -> Result<(), DbError> {
    let mut conn = pool.get()?;

    match login_manager
        .get_and_update_user_info_with_refresh_token(&pool, user.fxa_refresh_token.clone())
        .await
    {
        Ok(_) => {
            diesel::update(schema::webhook_events::table.filter(schema::webhook_events::id.eq(id)))
                .set(schema::webhook_events::status.eq(FxaEventStatus::Processed))
                .execute(&mut conn)?;
            Ok(())
        }
        Err(e) => {
            diesel::update(schema::webhook_events::table.filter(schema::webhook_events::id.eq(id)))
                .set(schema::webhook_events::status.eq(FxaEventStatus::Failed))
                .execute(&mut conn)?;
            Err(e.into())
        }
    }
}

pub async fn run_update_profile(
    pool: web::Data<Pool>,
    id: i64,
    user: UserQuery,
    login_manager: web::Data<LoginManager>,
) {
    if let Err(e) = update_profile(pool, id, user, login_manager).await {
        error!("Error updating profile from fxa webhook event: {}", e);
    }
}

pub async fn update_profile_from_webhook(
    pool: web::Data<Pool>,
    arbiter: web::Data<ArbiterHandle>,
    fxa_uid: String,
    login_manager: web::Data<LoginManager>,
    update: ProfileChange,
    issue_time: DateTime<Utc>,
) -> Result<(), DbError> {
    let mut conn = pool.get()?;
    let user: Option<UserQuery> = get_user_opt(&mut conn, &fxa_uid)?;
    let mut fxa_event = WebHookEventInsert {
        fxa_uid,
        change_time: None,
        issue_time: issue_time.naive_utc(),
        typ: FxaEvent::ProfileChange,
        status: FxaEventStatus::Pending,
        payload: serde_json::value::to_value(update).unwrap_or_default(),
    };
    if let Some(user) = user {
        let id = insert_into(schema::webhook_events::table)
            .values(fxa_event)
            .returning(schema::webhook_events::id)
            .get_result::<i64>(&mut conn)?;
        debug!("spawning processing job");
        if !arbiter.spawn(run_update_profile(pool, id, user, login_manager)) {
            error!("Arbiter did fail trying to update profile");
            diesel::update(schema::webhook_events::table.filter(schema::webhook_events::id.eq(id)))
                .set(schema::webhook_events::status.eq(FxaEventStatus::Failed))
                .execute(&mut conn)?;
        }
        Ok(())
    } else {
        fxa_event.status = FxaEventStatus::Ignored;
        insert_into(schema::webhook_events::table)
            .values(fxa_event)
            .execute(&mut conn)?;
        Ok(())
    }
}

pub async fn update_subscription_state_from_webhook(
    pool: web::Data<Pool>,
    fxa_uid: String,
    update: SubscriptionStateChange,
    issue_time: DateTime<Utc>,
    basket: web::Data<Option<Basket>>,
) -> Result<(), DbError> {
    let mut conn = pool.get()?;
    let user: Option<UserQuery> = get_user_opt(&mut conn, &fxa_uid)?;
    let mut fxa_event = WebHookEventInsert {
        fxa_uid,
        change_time: Some(update.change_time.naive_utc()),
        issue_time: issue_time.naive_utc(),
        typ: FxaEvent::SubscriptionStateChange,
        status: FxaEventStatus::Pending,
        payload: serde_json::value::to_value(&update).unwrap(),
    };
    debug!("Got subscription state change event: {:?}", update);

    if let Some(user) = user {
        let ignore = schema::webhook_events::table
            .filter(
                schema::webhook_events::fxa_uid.eq(&fxa_event.fxa_uid).and(
                    schema::webhook_events::typ
                        .eq(&fxa_event.typ)
                        .and(schema::webhook_events::change_time.ge(&fxa_event.change_time)),
                ),
            )
            .count()
            .first::<i64>(&mut conn)?
            != 0;
        if !ignore {
            let id = insert_into(schema::webhook_events::table)
                .values(fxa_event)
                .returning(schema::webhook_events::id)
                .get_result::<i64>(&mut conn)?;
            // Filter out any unknown subscription types we get passed in
            // before we try to get the interesting first element of the
            // capabilities array.
            let capability = update
                .capabilities
                .into_iter()
                .find(|&c| c != fxa::types::Subscription::Unknown);
            let subscription: Subscription = match (update.is_active, capability) {
                (false, _) => Subscription::Core,
                (true, Some(c)) => Subscription::from(c),
                (true, None) => Subscription::Core,
            };
            if subscription == Subscription::Core {
                // drop permissions
                if let Some(basket) = basket.get_ref() {
                    if let Err(e) = newsletter::unsubscribe(&mut conn, &user, basket).await {
                        error!("error unsubscribing user: {}", e);
                    }
                }
                if let Err(e) = create_or_update_settings(
                    &mut conn,
                    SettingsInsert {
                        user_id: user.id,
                        no_ads: Some(false),
                        ..Default::default()
                    },
                ) {
                    error!("error resetting settings for user: {}", e);
                }
            }
            match diesel::update(schema::users::table.filter(schema::users::id.eq(user.id)))
                .set(schema::users::subscription_type.eq(subscription))
                .execute(&mut conn)
            {
                Ok(_) => {
                    diesel::update(
                        schema::webhook_events::table.filter(schema::webhook_events::id.eq(id)),
                    )
                    .set(schema::webhook_events::status.eq(FxaEventStatus::Processed))
                    .execute(&mut conn)?;
                    // return Ok(());
                }
                Err(e) => {
                    diesel::update(
                        schema::webhook_events::table.filter(schema::webhook_events::id.eq(id)),
                    )
                    .set(schema::webhook_events::status.eq(FxaEventStatus::Failed))
                    .execute(&mut conn)?;
                    return Err(e.into());
                }
            }
            // Record the subscription state change in its table.
            let old_subscription = user.get_subscription_type();
            if let Some(old_subscription) = old_subscription {
                // Do not record transitions that are not transitioning anything.
                // This can happen if the user cancels, but their subscription
                // has some time left (monthly/yearly subscription).
                // When the subscription actually ends, the system will send us a
                // new event.
                if old_subscription == subscription {
                    return Ok(());
                }
                // We have the user id, the old and new subscription,
                // they are different, so go ahead storing it.
                let subscription_change = SubscriptionChangeInsert {
                    user_id: user.id,
                    old_subscription_type: old_subscription,
                    new_subscription_type: subscription,
                    created_at: update.change_time.naive_utc(),
                };
                insert_into(schema::user_subscription_transitions::table)
                    .values(subscription_change)
                    .execute(&mut conn)?;
            }
            return Ok(());
        }
    }

    fxa_event.status = FxaEventStatus::Ignored;
    insert_into(schema::webhook_events::table)
        .values(fxa_event)
        .execute(&mut conn)?;
    Ok(())
}
