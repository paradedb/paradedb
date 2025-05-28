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
use crate::fixtures::querygen::joingen::JoinType;
use crate::fixtures::querygen::wheregen::arb_wheres;

use fixtures::*;

use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use rstest::*;
use sqlx::PgConnection;

fn generated_queries_setup(conn: &mut PgConnection, tables: &[(&str, usize)]) -> String {
    "CREATE EXTENSION pg_search;".execute(conn);
    "SET log_error_verbosity TO VERBOSE;".execute(conn);
    "SET log_min_duration_statement TO 1000;".execute(conn);

    let mut setup_sql = String::new();

    for (tname, row_count) in tables {
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
FROM generate_series(1, {row_count});

CREATE INDEX idx{tname} ON {tname} USING bm25 (id, name, color, age)
WITH (
key_field = 'id',
text_fields = '
            {{
                "name": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }},
                "color": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }},
                "age": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }}
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

///
/// Tests all JoinTypes against small tables (which are particularly important for joins which
/// result in e.g. the cartesian product).
///
#[rstest]
#[tokio::test]
async fn generated_joins_small(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let tables_and_sizes = [("users", 10), ("products", 10), ("orders", 10)];
    let tables = tables_and_sizes
        .iter()
        .map(|(table, _)| table)
        .collect::<Vec<_>>();
    let setup_sql = generated_queries_setup(&mut pool.pull(), &tables_and_sizes);
    eprintln!("{setup_sql}");

    proptest!(|(
        (join, where_expr) in arb_joins_and_wheres(
            any::<JoinType>(),
            tables,
            vec![("id", "3"), ("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
    )| {
        let join_clause = join.to_sql();

        let from = format!("SELECT COUNT(*) {join_clause} ");

        let pg = format!("{from} WHERE {}", where_expr.to_sql(" = "));
        let bm25 = format!("{from} WHERE {}", where_expr.to_sql("@@@"));

        let (pg_cnt,) = (&pg).fetch_one::<(i64,)>(&mut pool.pull());
        let (bm25_cnt,) = (&bm25).fetch_one::<(i64,)>(&mut pool.pull());
        prop_assert_eq!(
            pg_cnt,
            bm25_cnt,
            "\npg:\n  {}\nbm25:\n  {}\nexplain:\n{}\n",
            pg,
            bm25,
            format!("EXPLAIN {bm25}").fetch::<(String,)>(&mut pool.pull()).into_iter().map(|(s,)| s).collect::<Vec<_>>().join("\n"),
        );
    });
}

///
/// Tests only the smallest JoinType against larger tables, with a target list, and a limit.
///
#[rstest]
#[tokio::test]
async fn generated_joins_large_limit(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let tables_and_sizes = [("users", 10000), ("products", 10000), ("orders", 10000)];
    let tables = tables_and_sizes
        .iter()
        .map(|(table, _)| table)
        .collect::<Vec<_>>();
    let setup_sql = generated_queries_setup(&mut pool.pull(), &tables_and_sizes);
    eprintln!("{setup_sql}");

    proptest!(|(
        (join, where_expr) in arb_joins_and_wheres(
            Just(JoinType::Inner),
            tables,
            vec![("id", "3"), ("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
        target_list in proptest::sample::subsequence(vec!["id", "name", "color", "age"], 1..=4),
    )| {
        let join_clause = join.to_sql();
        let used_tables = join.used_tables();

        let target_list =
            target_list
                .into_iter()
                .map(|column| format!("{}.{column}", used_tables[0]))
                .collect::<Vec<_>>()
                .join(", ");
        let from = format!("SELECT {target_list} {join_clause} ");

        let pg = format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql(" = "));
        let bm25 = format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql("@@@"));

        // Because we use a generated target list, we fetch as dynamic to allow for comparison.
        let pg_rows = (&pg).fetch_dynamic(&mut pool.pull());
        let bm25_rows = (&bm25).fetch_dynamic(&mut pool.pull());
        prop_assert_eq!(
            pg_rows.len(),
            bm25_rows.len(),
            "\npg:\n  {}\nbm25:\n  {}\nexplain:\n{}\n",
            pg,
            bm25,
            format!("EXPLAIN {bm25}").fetch::<(String,)>(&mut pool.pull()).into_iter().map(|(s,)| s).collect::<Vec<_>>().join("\n"),
        );
    });
}

#[rstest]
#[tokio::test]
async fn generated_single_relation(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let table_name = "users";
    generated_queries_setup(&mut pool.pull(), &[(table_name, 10)]);

    proptest!(|(
        where_expr in arb_wheres(
            vec![table_name],
            vec![("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
    )| {
        let where_clause = where_expr.to_sql(" = ");
        let pg = format!("SELECT COUNT(*) FROM {table_name} WHERE {where_clause}");
        let bm25 = format!(
            "SELECT COUNT(*) FROM {table_name} WHERE ({where_clause}) AND id @@@ paradedb.all()"
        ); // force a pushdown

        let (pg_cnt,) = (&pg).fetch_one::<(i64,)>(&mut pool.pull());
        let (bm25_cnt,) = (&bm25).fetch_one::<(i64,)>(&mut pool.pull());
        prop_assert_eq!(
            pg_cnt,
            bm25_cnt,
            "\npg:\n  {}\nbm25:\n  {}\nexplain:\n{}\n",
            pg,
            bm25,
            format!("EXPLAIN {bm25}").fetch::<(String,)>(&mut pool.pull()).into_iter().map(|(s,)| s).collect::<Vec<_>>().join("\n"),
        );
    });
}
