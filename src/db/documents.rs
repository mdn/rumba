use crate::db::model::{DocumentInsert, DocumentMetadata};
use crate::db::schema;

use diesel::r2d2::ConnectionManager;
use diesel::{insert_into, PgConnection, QueryResult, RunQueryDsl};
use r2d2::PooledConnection;

use crate::settings::SETTINGS;

pub async fn create_or_update_document(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    document: DocumentMetadata,
    uri: String,
) -> QueryResult<i64> {
    let absolute_uri = format!("{}{}", SETTINGS.application.document_base_url, uri);
    let title = document.title.clone();
    let metadata = serde_json::to_value(&document).ok();
    let paths = document.paths;
    let insert = DocumentInsert {
        title,
        absolute_uri,
        uri,
        metadata,
        paths,
        updated_at: chrono::offset::Utc::now().naive_utc(),
    };

    insert_into(schema::documents::table)
        .values(&insert)
        .on_conflict(schema::documents::uri)
        .do_update()
        .set(&insert)
        .returning(schema::documents::id)
        .get_result(conn)
}
