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

use std::fmt::Debug;

use crate::fixtures::querygen::arb_joins_and_wheres;
use crate::fixtures::querygen::joingen::JoinType;
use crate::fixtures::querygen::pagegen::arb_paging_exprs;
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
    id    SERIAL8 NOT NULL PRIMARY KEY,
    uuid  UUID NOT NULL,
    name  TEXT,
    color VARCHAR,
    age   VARCHAR
);

-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idx{tname} ON {tname} USING bm25 (id, uuid, name, color, age)
WITH (
key_field = 'id',
text_fields = '
            {{
                "uuid": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }},
                "name": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }},
                "color": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }},
                "age": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }}
            }}'
);

INSERT into {tname} (uuid, name, color, age)
VALUES (gen_random_uuid(), 'bob', 'blue', 20);

INSERT into {tname} (uuid, name, color, age)
SELECT
      gen_random_uuid(),
      (ARRAY ['alice','bob','cloe', 'sally','brandy','brisket','anchovy']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['red','green','blue', 'orange','purple','pink','yellow']::text[])[(floor(random() * 7) + 1)::int],
      (floor(random() * 100) + 1)::int::text
FROM generate_series(1, {row_count});

CREATE INDEX idx{tname}_uuid ON {tname} (uuid);
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

/// Run the given pg and bm25 queries on the given connection, and compare their results across a
/// variety of configurations of the extension.
///
/// TODO: The configurations of the extension in the loop below could in theory also be proptested
/// properties: if performance becomes a concern, we should lift them out, and apply them using the
/// proptest properties instead.
fn compare<R, F>(
    pg_query: String,
    bm25_query: String,
    conn: &mut PgConnection,
    run_query: F,
) -> Result<(), TestCaseError>
where
    R: Eq + Debug,
    F: Fn(&str, &mut PgConnection) -> R,
{
    // the postgres query is always run with the paradedb custom scan turned off
    // this ensures we get the actual, known-to-be-correct result from Postgres'
    // plan, and not from ours where we did some kind of pushdown
    r#"
        SET max_parallel_workers TO 8;
        SET enable_seqscan TO ON;
        SET enable_indexscan TO ON;
        SET paradedb.enable_custom_scan TO OFF;
    "#
    .execute(conn);

    let pg_result = run_query(&pg_query, conn);

    // and for the "bm25" query, we run it a number of times with more and more scan types disabled,
    // always ensuring that paradedb's custom scan is turned on
    "SET paradedb.enable_custom_scan TO ON;".execute(conn);
    for scan_type in [
        "SET enable_seqscan TO OFF",
        "SET enable_indexscan TO OFF",
        "SET max_parallel_workers TO 0",
    ] {
        scan_type.execute(conn);

        let bm25_result = run_query(&bm25_query, conn);
        prop_assert_eq!(
            &pg_result,
            &bm25_result,
            "\nscan_type={}\npg:\n  {}\nbm25:\n  {}\nexplain:\n{}\n",
            scan_type,
            pg_query,
            bm25_query,
            format!("EXPLAIN ANALYZE {bm25_query}")
                .fetch::<(String,)>(conn)
                .into_iter()
                .map(|(s,)| s)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    Ok(())
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

        compare(
            format!("{from} WHERE {}", where_expr.to_sql(" = ")),
            format!("{from} WHERE {}", where_expr.to_sql("@@@")),
            &mut pool.pull(),
            |query, conn| query.fetch_one::<(i64,)>(conn).0,
        )?;
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

        compare(
            format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql(" = ")),
            format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql("@@@")),
            &mut pool.pull(),
            |query, conn| query.fetch_dynamic(conn).len(),
        )?;
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
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 10)]);
    eprintln!("{setup_sql}");

    proptest!(|(
        where_expr in arb_wheres(
            vec![table_name],
            vec![("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
    )| {
        compare(
            format!("SELECT id FROM {table_name} WHERE {}", where_expr.to_sql(" = ")),
            format!("SELECT id FROM {table_name} WHERE {}", where_expr.to_sql("@@@")),
            &mut pool.pull(),
            |query, conn| {
                let mut rows = query.fetch::<(i64,)>(conn);
                rows.sort();
                rows
            }
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_paging_small(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 1000)]);
    eprintln!("{setup_sql}");

    proptest!(|(
        where_expr in arb_wheres(vec![table_name], vec![("name", "bob")]),
        // TODO: Until https://github.com/paradedb/paradedb/issues/2642 is resolved, we do not
        // tiebreak appropriately for compound columns, and so we do not pass any non-tiebreak
        // columns here.
        paging_exprs in arb_paging_exprs(table_name, vec![], vec!["id", "uuid"]),
    )| {
        compare(
            format!("SELECT id FROM {table_name} WHERE {} {paging_exprs}", where_expr.to_sql(" = ")),
            format!("SELECT id FROM {table_name} WHERE {} {paging_exprs}", where_expr.to_sql("@@@")),
            &mut pool.pull(),
            |query, conn| query.fetch::<(i64,)>(conn),
        )?;
    });
}

/// Generates paging expressions on a large table, which was necessary to reproduce
/// https://github.com/paradedb/tantivy/pull/51
///
/// TODO: Explore whether this could use https://github.com/paradedb/paradedb/pull/2681
/// to use a large segment count rather than a large table size.
#[rstest]
#[tokio::test]
async fn generated_paging_large(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 100000)]);
    eprintln!("{setup_sql}");

    proptest!(|(
        paging_exprs in arb_paging_exprs(table_name, vec![], vec!["uuid"]),
    )| {
        compare(
            format!("SELECT uuid::text FROM {table_name} WHERE name  =  'bob' {paging_exprs}"),
            format!("SELECT uuid::text FROM {table_name} WHERE name @@@ 'bob' {paging_exprs}"),
            &mut pool.pull(),
            |query, conn| query.fetch::<(String,)>(conn),
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_subquery(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let outer_table_name = "products";
    let inner_table_name = "orders";
    let setup_sql = generated_queries_setup(
        &mut pool.pull(),
        &[(outer_table_name, 10), (inner_table_name, 10)],
    );
    eprintln!("{setup_sql}");

    proptest!(|(
        outer_where_expr in arb_wheres(
            vec![outer_table_name],
            vec![("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
        inner_where_expr in arb_wheres(
            vec![inner_table_name],
            vec![("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
        subquery_column in proptest::sample::select(&["name", "color", "age"]),
        // TODO: Until https://github.com/paradedb/paradedb/issues/2642 is resolved, we do not
        // tiebreak appropriately for compound columns, and so we do not pass any non-tiebreak
        // columns here.
        // TODO: Not ordering by `uuid` until https://github.com/paradedb/paradedb/issues/2703
        // is fixed.
        paging_exprs in arb_paging_exprs(inner_table_name, vec![], vec!["id"]),
    )| {
        let pg = format!(
            "SELECT COUNT(*) FROM {outer_table_name} \
            WHERE {outer_table_name}.{subquery_column} IN (\
                SELECT {subquery_column} FROM {inner_table_name} WHERE {} {paging_exprs}\
            ) AND {}",
            inner_where_expr.to_sql(" = "),
            outer_where_expr.to_sql(" = "),
        );
        let bm25 = format!(
            "SELECT COUNT(*) FROM {outer_table_name} \
            WHERE {outer_table_name}.{subquery_column} IN (\
                SELECT {subquery_column} FROM {inner_table_name} WHERE {} {paging_exprs}\
            ) AND {}",
            inner_where_expr.to_sql("@@@"),
            outer_where_expr.to_sql("@@@"),
        );

        compare(
            pg,
            bm25,
            &mut pool.pull(),
            |query, conn| query.fetch_one::<(i64,)>(conn),
        )?;
    });
}
