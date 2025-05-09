// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use crate::fixtures::querygen::arb_joins_and_wheres;
use crate::fixtures::querygen::wheregen::arb_wheres;

use fixtures::*;

use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use rstest::*;
use sqlx::PgConnection;

fn generated_queries_setup(conn: &mut PgConnection, tables: &[&str]) -> String {
    "CREATE EXTENSION pg_search;".execute(conn);

    let mut setup_sql = String::new();

    for tname in tables {
        let sql = format!(
            r#"
CREATE TABLE {tname}
(
    id    serial8 not null primary key,
    name  text,
    color varchar,
    age   varchar
);

INSERT into {tname} (name, color, age)
VALUES ('bob', 'blue', 20);

INSERT into {tname} (name, color, age)
SELECT(ARRAY ['alice','bob','cloe', 'sally','brandy','brisket','anchovy']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['red','green','blue', 'orange','purple','pink','yellow']::text[])[(floor(random() * 7) + 1)::int],
      (floor(random() * 100) + 1)::int::text
FROM generate_series(1, 10);    -- could make larger, but 10 finds failures and is fast

CREATE INDEX idx{tname} ON {tname} USING bm25 (id, name, color, age)
WITH (
key_field = 'id',
text_fields = '
            {{
                "name": {{ "tokenizer": {{ "type": "keyword" }} }},
                "color": {{ "tokenizer": {{ "type": "keyword" }} }},
                "age": {{ "tokenizer": {{ "type": "keyword" }} }}
            }}'
);
CREATE INDEX idx{tname}_name ON {tname} (name);
CREATE INDEX idx{tname}_color ON {tname} (color);
CREATE INDEX idx{tname}_age ON {tname} (age);
ANALYZE;
"#,
            tname = tname
        );

        (&sql).execute(conn);
        setup_sql.push_str(&sql);
    }

    setup_sql
}

#[rstest]
#[tokio::test]
async fn generated_join_queries(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    generated_queries_setup(&mut pool.pull(), &["users", "products", "orders"]);

    proptest!(|(
        (join, where_expr) in arb_joins_and_wheres(
            vec!["users", "orders", "products"],
            vec![("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
    )| {
        let join_clause = join.join_clause;

        let from = format!("SELECT COUNT(*) {join_clause} ");

        let pg = format!("{from} WHERE {}", where_expr.to_sql("@@@"));
        let bm25 = format!("{from} WHERE {}", where_expr.to_sql(" = "));

        let (pg_count,) = (&pg).fetch_one::<(i64,)>(&mut pool.pull());
        let (bm25_count,) = (&bm25).fetch_one::<(i64,)>(&mut pool.pull());
        prop_assert_eq!(
            pg_count,
            bm25_count,
            "\npg:\n  {}\t{}\nbm25:\n  {}\t{}\n",
            pg_count,
            pg,
            bm25_count,
            bm25,
        );
    });
}

#[rstest]
#[tokio::test]
async fn generated_single_relation_queries(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let table_name = "users";
    generated_queries_setup(&mut pool.pull(), &[table_name]);

    proptest!(|(
        where_expr in arb_wheres(
            vec![table_name],
            vec![("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
    )| {
        let where_clause = where_expr.to_sql(" = ");
        let pg = format!("SELECT count(*) FROM {table_name} WHERE {where_clause}");
        let bm25 = format!(
            "SELECT count(*) FROM {table_name} WHERE ({where_clause}) AND id @@@ paradedb.all()"
        ); // force a pushdown

        let mut conn = pool.pull();
        let (pg_count,) = (&pg).fetch_one::<(i64,)>(&mut conn);
        let (bm25_count,) = (&bm25).fetch_one::<(i64,)>(&mut conn);

        prop_assert_eq!(
            pg_count,
            bm25_count,
            "\npg:\n  {}\t{}\nbm25:\n  {}\t{}\n",
            pg_count,
            pg,
            bm25_count,
            bm25,
        );
    });
}
