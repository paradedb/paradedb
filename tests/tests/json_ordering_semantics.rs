// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file for details.

mod fixtures;

use fixtures::*;
use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use rstest::*;
use sqlx::PgConnection;

use crate::fixtures::querygen::{compare, PgGucs};

/// Build the test table + index with fast json field.
fn setup(conn: &mut PgConnection) -> String {
    "CREATE EXTENSION IF NOT EXISTS pg_search;".execute(conn);
    "SET log_error_verbosity TO VERBOSE;".execute(conn);
    "SET log_min_duration_statement TO 1000;".execute(conn);

    let setup_sql = r#"
DROP TABLE IF EXISTS json_ordering_test;
CREATE TABLE json_ordering_test (
    id SERIAL8 NOT NULL PRIMARY KEY,
    metadata JSONB
);

CREATE INDEX idx_json_ordering_test ON json_ordering_test
USING bm25 (id, metadata)
WITH (
    key_field = 'id',
    json_fields = '{
        "metadata": { "fast": true }
    }'
);

-- help our cost estimates
ANALYZE json_ordering_test;
"#;
    setup_sql.execute(conn);
    setup_sql.to_string()
}

#[rstest]
#[tokio::test]
async fn json_lexical_ordering_matches_postgres() {
    let database = Db::new().await;
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    proptest!(|(values in proptest::collection::vec(0u32..1000, 12..30))| {
        let mut conn = pool.pull();
        let setup_sql = setup(&mut conn);

        // Insert rows with numeric-like strings so lexical vs numeric ordering differs (e.g. "10" < "2" lexically).
        // We quote the numbers so they are stored as JSON strings, not JSON numbers.
        for v in &values {
            let sql = format!("INSERT INTO json_ordering_test (metadata) VALUES ('{{\"code\": \"{}\"}}');", v);
            sql.execute(&mut conn);
        }
        // Include a few fixed values to amplify ordering edge cases.
        r#"INSERT INTO json_ordering_test (metadata) VALUES ('{"code": "2"}'), ('{"code": "10"}'), ('{"code": "1"}');"#.execute(&mut conn);

        // Baseline Postgres query (custom scan disabled automatically inside compare()).
        let pg_query = "SELECT metadata->>'code' FROM json_ordering_test ORDER BY metadata->>'code' LIMIT 100";

        // BM25 query to trigger custom scan + fast ordering pushdown.
        let bm25_query = "SELECT metadata->>'code' FROM json_ordering_test WHERE id @@@ paradedb.all() ORDER BY metadata->>'code' LIMIT 100";

        // Enable the custom scan GUC so ORDER BY pushdown can occur.
        // Enable only the custom scan to exercise ORDER BY pushdown.
        let gucs = PgGucs::with_custom_scan();

        compare(
            pg_query,
            bm25_query,
            &gucs,
            &mut conn,
            &setup_sql,
            |query, c| query.fetch::<(Option<String>,)>(c).into_iter().map(|(s,)| s).collect::<Vec<_>>(),
        )?;
    });
}
