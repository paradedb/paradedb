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

        for v in &values {
            let sql = format!("INSERT INTO json_ordering_test (metadata) VALUES ('{{\"code\": \"{}\"}}');", v);
            sql.execute(&mut conn);
        }

        // Include a few fixed values to amplify ordering edge cases.
        r#"INSERT INTO json_ordering_test (metadata) VALUES ('{"code": "2"}'), ('{"code": "10"}'), ('{"code": "1"}');"#.execute(&mut conn);

        // Baseline Postgres query
        let pg_query = "SELECT metadata->>'code' FROM json_ordering_test ORDER BY metadata->>'code' LIMIT 100";

        // BM25 query to trigger custom scan + fast ordering pushdown.
        let bm25_query = "SELECT metadata->>'code' FROM json_ordering_test WHERE id @@@ paradedb.all() ORDER BY metadata->>'code' LIMIT 100";

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
