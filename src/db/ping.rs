use diesel::{
    insert_into, ExpressionMethods, PgConnection, PgJsonbExpressionMethods, QueryResult,
    RunQueryDsl,
};
use serde_json::Value;

use super::{
    model::{ActivityPingInsert, UserQuery},
    schema::activity_pings,
};

pub fn upsert_activity_ping(
    conn_pool: &mut PgConnection,
    user: UserQuery,
    data: Value,
) -> QueryResult<usize> {
    insert_into(activity_pings::table)
        .values(ActivityPingInsert {
            user_id: user.id,
            activity: data.clone(),
        })
        .on_conflict((activity_pings::user_id, activity_pings::ping_at))
        .do_update()
        .set(activity_pings::activity.eq(activity_pings::activity.concat(data)))
        .execute(conn_pool)
}
