use diesel::{prelude::*, sql_query};
use pgvector::Vector;
use serde::{Deserialize, Serialize};

mod schema {
    table! {
        mdn_doc_section {
            id -> Int8,
            doc_id -> Int8,
            content -> Text,
            token_count -> Int4,
            embedding -> pgvector::sql_types::Vector,
            heading -> Text,
        }
    }
}

use self::schema::mdn_doc_section;

#[table_name="mdn_doc_section"]
#[derive(Queryable, QueryableByName, Insertable, Debug)]
pub struct MdnDoc {
    pub id: i64,
    pub doc_id: i64,
    pub content: String,
    pub token_count: i32,
    pub embedding: Vector,
    pub heading: String,
}

impl MdnDoc {
    pub fn find(conn: &mut PgConnection) -> MdnDoc {
        mdn_doc_section::table.first::<MdnDoc>(conn).unwrap()
    }
}