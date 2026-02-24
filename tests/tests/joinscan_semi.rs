// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use crate::fixtures::querygen::{compare, PgGucs};
use fixtures::*;
use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

fn semi_join_setup(conn: &mut PgConnection) -> String {
    let setup_sql = r#"
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS semijoin_left CASCADE;
DROP TABLE IF EXISTS semijoin_right CASCADE;

CREATE TABLE semijoin_left (
    id SERIAL8 PRIMARY KEY,
    name TEXT NOT NULL,
    age INTEGER NOT NULL
);

CREATE TABLE semijoin_right (
    id SERIAL8 PRIMARY KEY,
    name TEXT NOT NULL,
    age INTEGER NOT NULL
);

INSERT INTO semijoin_left (name, age)
SELECT
    (ARRAY['alpha', 'beta', 'gamma', 'delta', 'epsilon', 'zeta'])[(g % 6) + 1],
    (g % 40) + 20
FROM generate_series(1, 800) AS g;

INSERT INTO semijoin_right (name, age)
SELECT
    (ARRAY['alpha', 'beta', 'gamma', 'delta'])[(g % 4) + 1],
    (g % 40) + 20
FROM generate_series(1, 120) AS g;

CREATE INDEX semijoin_left_bm25 ON semijoin_left
USING bm25 (id, name, age)
WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true, "tokenizer": {"type": "keyword"}}}',
    numeric_fields = '{"age": {"fast": true}}'
);

CREATE INDEX semijoin_right_bm25 ON semijoin_right
USING bm25 (id, name, age)
WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true, "tokenizer": {"type": "keyword"}}}',
    numeric_fields = '{"age": {"fast": true}}'
);

ANALYZE semijoin_left;
ANALYZE semijoin_right;
"#;

    setup_sql.execute(conn);
    setup_sql.to_string()
}

fn assert_uses_semi_joinscan(conn: &mut PgConnection, query: &str) {
    let gucs = PgGucs {
        join_custom_scan: true,
        ..PgGucs::default()
    };
    gucs.set().execute(conn);

    let explain_query = format!("EXPLAIN (FORMAT JSON) {query}");
    let (plan,): (Value,) = explain_query.fetch_one(conn);
    let plan_str = format!("{plan:#?}");

    assert!(
        plan_str.contains("ParadeDB Join Scan") && plan_str.contains("Semi"),
        "Query should use ParadeDB Join Scan with Semi join type, but got plan: {plan_str}\nQuery: {query}",
    );
}

#[rstest]
#[tokio::test]
async fn generated_semijoin_exists_equivalence(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    proptest!(|(
        left_term in proptest::sample::select(vec!["alpha", "beta", "gamma", "delta"]),
        right_term in proptest::sample::select(vec!["alpha", "beta", "gamma", "delta"]),
        limit in 1usize..=200usize,
    )| {
        let setup_sql = semi_join_setup(&mut pool.pull());
        let query = format!(
            "SELECT l.id, l.name \
             FROM semijoin_left l \
             WHERE l.name @@@ '{left_term}' \
               AND EXISTS ( \
                   SELECT 1 \
                   FROM semijoin_right r \
                   WHERE r.age = l.age \
                     AND r.name @@@ '{right_term}' \
               ) \
             ORDER BY l.id \
             LIMIT {limit}"
        );

        assert_uses_semi_joinscan(&mut pool.pull(), &query);

        let gucs = PgGucs {
            join_custom_scan: true,
            ..PgGucs::default()
        };

        compare(
            &query,
            &query,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |sql, conn| sql.fetch::<(i64, String)>(conn),
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_semijoin_in_equivalence(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    proptest!(|(
        left_term in proptest::sample::select(vec!["alpha", "beta", "gamma", "delta"]),
        right_term in proptest::sample::select(vec!["alpha", "beta", "gamma", "delta"]),
        limit in 1usize..=200usize,
    )| {
        let setup_sql = semi_join_setup(&mut pool.pull());
        let query = format!(
            "SELECT l.id, l.name \
             FROM semijoin_left l \
             WHERE l.name @@@ '{left_term}' \
               AND l.age IN ( \
                   SELECT r.age \
                   FROM semijoin_right r \
                   WHERE r.name @@@ '{right_term}' \
               ) \
             ORDER BY l.id \
             LIMIT {limit}"
        );

        assert_uses_semi_joinscan(&mut pool.pull(), &query);

        let gucs = PgGucs {
            join_custom_scan: true,
            ..PgGucs::default()
        };

        compare(
            &query,
            &query,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |sql, conn| sql.fetch::<(i64, String)>(conn),
        )?;
    });
}
