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

// Tests for ParadeDB's Aggregate Custom Scan implementation

use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;
use tests::fixtures::*;
use time::Date;
use time::macros::date;

fn assert_uses_custom_scan(conn: &mut PgConnection, enabled: bool, query: impl AsRef<str>) {
    let (plan,) = format!(" EXPLAIN (FORMAT JSON) {}", query.as_ref()).fetch_one::<(Value,)>(conn);
    eprintln!("{plan:#?}");
    assert_eq!(
        enabled,
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );
}

fn assert_uses_datafusion_aggregate_scan(conn: &mut PgConnection, query: impl AsRef<str>) {
    let (plan,) = format!("EXPLAIN (FORMAT JSON) {}", query.as_ref()).fetch_one::<(Value,)>(conn);

    let plan = plan.to_string();
    assert!(
        plan.contains("ParadeDB Aggregate Scan"),
        "expected ParadeDB Aggregate Scan:\n{plan}"
    );
    assert!(
        plan.contains("DataFusion Physical Plan"),
        "expected DataFusion backend:\n{plan}"
    );
}

#[rstest]
fn test_count(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Use the aggregate custom scan only if it is enabled.
    for enabled in [true, false] {
        format!("SET paradedb.enable_aggregate_custom_scan TO {enabled};").execute(&mut conn);

        let query = "SELECT COUNT(*) FROM paradedb.bm25_search WHERE description @@@ 'keyboard'";

        assert_uses_custom_scan(&mut conn, enabled, query);

        let (count,) = query.fetch_one::<(i64,)>(&mut conn);
        assert_eq!(count, 2, "With custom scan: {enabled}");
    }
}

#[rstest]
fn test_count_with_group_by(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);
    "SET client_min_messages TO warning;".execute(&mut conn);

    // First test simple COUNT(*) without GROUP BY
    let simple_count = "SELECT COUNT(*) FROM paradedb.bm25_search";
    eprintln!("Testing simple COUNT(*)");
    let (plan,) = format!("EXPLAIN (FORMAT JSON) {simple_count}").fetch_one::<(Value,)>(&mut conn);
    eprintln!("Simple COUNT(*) plan: {plan:#?}");
    eprintln!(
        "Uses aggregate scan: {}",
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );

    // Test COUNT(*) with WHERE clause (like the working test)
    let count_with_where =
        "SELECT COUNT(*) FROM paradedb.bm25_search WHERE description @@@ 'keyboard'";
    eprintln!("\nTesting COUNT(*) with WHERE clause");
    let (plan,) =
        format!("EXPLAIN (FORMAT JSON) {count_with_where}").fetch_one::<(Value,)>(&mut conn);
    eprintln!(
        "COUNT(*) with WHERE plan uses aggregate scan: {}",
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );

    // Then test WITHOUT WHERE clause but WITH GROUP BY
    let query_no_where = r#"
        SELECT rating, COUNT(*) 
        FROM paradedb.bm25_search 
        GROUP BY rating 
        ORDER BY rating
    "#;

    eprintln!("Testing query without WHERE clause");
    let (plan,) =
        format!("EXPLAIN (FORMAT JSON) {query_no_where}").fetch_one::<(Value,)>(&mut conn);
    eprintln!("Plan without WHERE: {plan:#?}");
    eprintln!(
        "Uses aggregate scan: {}",
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );

    // Then test WITH WHERE clause
    let query = r#"
        SELECT rating, COUNT(*) 
        FROM paradedb.bm25_search 
        WHERE description @@@ 'shoes' 
        GROUP BY rating 
        ORDER BY rating
    "#;

    // Verify it uses the aggregate custom scan
    assert_uses_custom_scan(&mut conn, true, query);

    // Execute and verify results
    let results: Vec<(i32, i64)> = query.fetch(&mut conn);
    assert_eq!(results.len(), 3); // We should have 3 distinct ratings for shoes
    assert_eq!(results[0], (3, 1)); // rating 3, count 1
    assert_eq!(results[1], (4, 1)); // rating 4, count 1
    assert_eq!(results[2], (5, 1)); // rating 5, count 1
}

#[rstest]
fn test_group_by(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    // Supports GROUP BY with aggregate scan
    assert_uses_custom_scan(
        &mut conn,
        true,
        r#"
        SELECT rating, COUNT(*)
        FROM paradedb.bm25_search WHERE
        description @@@ 'keyboard'
        GROUP BY rating
        ORDER BY rating
        "#,
    );
}

#[rstest]
fn test_group_by_null_bucket(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    assert_uses_custom_scan(
        &mut conn,
        true,
        r#"
        SELECT rating, COUNT(*)
        FROM paradedb.bm25_search
        WHERE description @@@ 'keyboard'
        GROUP BY rating
        ORDER BY rating NULLS FIRST
    "#,
    );
}

#[rstest]
fn test_no_bm25_index(mut conn: PgConnection) {
    "CALL paradedb.create_paradedb_test_table(table_name => 'no_bm25', schema_name => 'paradedb');"
        .execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    // Do not use the aggregate custom scan on non-bm25 indexed tables.
    assert_uses_custom_scan(&mut conn, false, "SELECT COUNT(*) FROM paradedb.no_bm25");
}

#[rstest]
fn test_other_aggregates(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    for aggregate_func in ["SUM(rating)", "AVG(rating)", "MIN(rating)", "MAX(rating)"] {
        assert_uses_custom_scan(
            &mut conn,
            true,
            format!(
                r#"
                SELECT {aggregate_func}
                FROM paradedb.bm25_search WHERE
                description @@@ 'keyboard'
                "#
            ),
        );
    }
}

#[rstest]
fn test_group_by_date_function(mut conn: PgConnection) {
    // 2024-01-01 has three rows spanning both edges of the day, which must
    // collapse into a single date group. 2024-01-04 has no rows and must not
    // appear as an empty date group. The two NULL rows must form their own group,
    // as they do for a plain `GROUP BY <timestamp column>`.
    r#"
    CREATE TABLE date_pushdown_events (
        id SERIAL PRIMARY KEY,
        created_at TIMESTAMP
    );
    INSERT INTO date_pushdown_events (created_at) VALUES
        ('2024-01-01 00:00:00'),
        ('2024-01-01 08:00:00'),
        ('2024-01-01 23:59:59'),
        ('2024-01-02 09:00:00'),
        ('2024-01-03 12:00:00'),
        ('2024-01-05 18:00:00'),
        (NULL),
        (NULL);
    CREATE INDEX date_pushdown_events_idx ON date_pushdown_events
        USING paradedb (id, created_at) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    let query = "SELECT DATE(created_at) AS day, COUNT(*) AS cnt \
                 FROM date_pushdown_events \
                 WHERE id @@@ pdb.all() \
                 GROUP BY DATE(created_at)";

    let rows = format!("{query} ORDER BY day NULLS LAST").fetch::<(Option<Date>, i64)>(&mut conn);

    assert_eq!(
        rows,
        vec![
            (Some(date!(2024 - 01 - 01)), 3), // 00:00:00, 08:00:00 and 23:59:59 collapse
            (Some(date!(2024 - 01 - 02)), 1),
            (Some(date!(2024 - 01 - 03)), 1),
            (Some(date!(2024 - 01 - 05)), 1), // 01-04 absent: no empty date group
            (None, 2),                        // both NULL-timestamp rows
        ],
        "one row per date group, plus the NULL group"
    );

    // DATE(timestamp) grouping must be executed by the DataFusion backend.
    assert_uses_datafusion_aggregate_scan(&mut conn, query);

    // ORDER BY an aggregate with LIMIT is the common dashboard TopK shape.
    // The counts are deliberately distinct at the cutoff: 2024-01-01 has
    // 3 rows and the NULL group has 2.
    let topk_query = "SELECT DATE(created_at) AS day, COUNT(*) AS cnt \
                      FROM date_pushdown_events \
                      WHERE id @@@ pdb.all() \
                      GROUP BY DATE(created_at) \
                      ORDER BY COUNT(*) DESC \
                      LIMIT 2";

    assert_uses_datafusion_aggregate_scan(&mut conn, topk_query);

    let topk_rows = topk_query.fetch::<(Option<Date>, i64)>(&mut conn);
    assert_eq!(
        topk_rows,
        vec![(Some(date!(2024 - 01 - 01)), 3), (None, 2)],
        "DataFusion TopK must retain the NULL date group"
    );

    // Parity: the same query planned by Postgres must give the same answer.
    "SET paradedb.enable_aggregate_custom_scan TO off;".execute(&mut conn);
    assert_uses_custom_scan(&mut conn, false, query);

    // Use distinct prepared statements for the fallback queries; see #6136.
    let fallback = format!("{query} ORDER BY day NULLS LAST -- fallback")
        .fetch::<(Option<Date>, i64)>(&mut conn);

    assert_eq!(rows, fallback, "pushdown must match Postgres exactly");

    let topk_fallback = format!("{topk_query} -- fallback").fetch::<(Option<Date>, i64)>(&mut conn);
    assert_eq!(
        topk_rows, topk_fallback,
        "DataFusion TopK must match Postgres exactly"
    );
}

#[rstest]
fn test_group_by_date_null_group_metrics(mut conn: PgConnection) {
    // DataFusion groups the NULL result of DATE(created_at) natively. Verify
    // that the NULL group carries every aggregate value, not only COUNT(*).
    r#"
    CREATE TABLE date_pushdown_metrics (
        id SERIAL PRIMARY KEY,
        created_at TIMESTAMP,
        amount INTEGER NOT NULL
    );
    INSERT INTO date_pushdown_metrics (created_at, amount) VALUES
        ('2024-01-01 08:00:00', 10),
        ('2024-01-01 20:00:00', 20),
        ('2024-01-02 09:00:00', 40),
        (NULL, 100),
        (NULL, 200);
    CREATE INDEX date_pushdown_metrics_id ON date_pushdown_metrics
        USING paradedb (id, created_at, amount) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    let query = "SELECT DATE(created_at) AS day, COUNT(*) AS cnt, SUM(amount) AS total \
                       FROM date_pushdown_metrics \
                       WHERE id @@@ pdb.all() \
                       GROUP BY DATE(created_at)";

    let rows =
        format!("{query} ORDER BY day NULLS LAST").fetch::<(Option<Date>, i64, i64)>(&mut conn);

    assert_eq!(
        rows,
        vec![
            (Some(date!(2024 - 01 - 01)), 2, 30),
            (Some(date!(2024 - 01 - 02)), 1, 40),
            (None, 2, 300), // Null group should have the aggregates
        ],
        "NULL group must have real metrics"
    );

    // Guard against the values silently coming from Postgres or Tantivy instead.
    assert_uses_datafusion_aggregate_scan(&mut conn, query);

    // Parity: the same query planned by Postgres must give the same answer.
    "SET paradedb.enable_aggregate_custom_scan TO off;".execute(&mut conn);
    assert_uses_custom_scan(&mut conn, false, query);

    let fallback = format!("{query} ORDER BY day NULLS LAST -- fallback")
        .fetch::<(Option<Date>, i64, i64)>(&mut conn);

    assert_eq!(rows, fallback, "pushdown must match Postgres exactly");
}

#[rstest]
fn test_group_by_date_with_filter(mut conn: PgConnection) {
    // DataFusion must apply the aggregate FILTER independently inside each
    // date group, including the group produced by a NULL timestamp.
    r#"
    CREATE TABLE date_pushdown_filter (
        id SERIAL PRIMARY KEY,
        created_at TIMESTAMP,
        amount INTEGER NOT NULL
    );
    INSERT INTO date_pushdown_filter (created_at, amount) VALUES
        ('2024-01-01 08:00:00', 10),
        ('2024-01-01 20:00:00', 20),
        ('2024-01-02 09:00:00', 40),
        (NULL, 100),
        (NULL, 200);
    CREATE INDEX date_pushdown_filter_idx ON date_pushdown_filter
        USING paradedb (id, created_at, amount) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    let query = "SELECT DATE(created_at) AS day, COUNT(*) FILTER (WHERE amount > 50) AS cnt \
                   FROM date_pushdown_filter \
                   WHERE id @@@ pdb.all() \
                   GROUP BY DATE(created_at)";

    assert_uses_datafusion_aggregate_scan(&mut conn, query);

    let rows = format!("{query} ORDER BY day NULLS LAST").fetch::<(Option<Date>, i64)>(&mut conn);

    assert_eq!(
        rows,
        vec![
            (Some(date!(2024 - 01 - 01)), 0),
            (Some(date!(2024 - 01 - 02)), 0),
            (None, 2), // both NULL-date rows exceed 50
        ],
        "DataFusion must compute the filtered NULL group correctly"
    );

    // Verify parity with native PostgreSQL aggregation.
    "SET paradedb.enable_aggregate_custom_scan TO off;".execute(&mut conn);
    assert_uses_custom_scan(&mut conn, false, query);

    let fallback = format!("{query} ORDER BY day NULLS LAST -- fallback")
        .fetch::<(Option<Date>, i64)>(&mut conn);

    assert_eq!(rows, fallback, "DataFusion must match Postgres exactly");
}

#[rstest]
fn test_group_by_date_multi_column(mut conn: PgConnection) {
    // DataFusion must combine a transformed DATE(timestamp) grouping column
    // with an ordinary identity grouping column. NULL dates must remain split
    // by region rather than being collapsed into one NULL group.
    r#"
    CREATE TABLE date_pushdown_multi (
        id SERIAL PRIMARY KEY,
        created_at TIMESTAMP,
        region TEXT
    );
    INSERT INTO date_pushdown_multi (created_at, region) VALUES
        ('2024-01-01 08:00:00', 'east'),
        ('2024-01-01 20:00:00', 'west'),
        ('2024-01-02 09:00:00', 'east'),
        (NULL, 'east'),
        (NULL, 'west');
    CREATE INDEX date_pushdown_multi_idx ON date_pushdown_multi
        USING paradedb (id, created_at, region)
        WITH (key_field = 'id', text_fields = '{"region": {"fast": true}}');
    "#
    .execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    let query = "SELECT DATE(created_at) AS day, region, COUNT(*) AS cnt \
                 FROM date_pushdown_multi \
                 WHERE id @@@ pdb.all() \
                 GROUP BY DATE(created_at), region";

    assert_uses_datafusion_aggregate_scan(&mut conn, query);

    let rows = format!("{query} ORDER BY day NULLS LAST, region")
        .fetch::<(Option<Date>, String, i64)>(&mut conn);

    assert_eq!(
        rows,
        vec![
            (Some(date!(2024 - 01 - 01)), "east".into(), 1),
            (Some(date!(2024 - 01 - 01)), "west".into(), 1),
            (Some(date!(2024 - 01 - 02)), "east".into(), 1),
            (None, "east".into(), 1),
            (None, "west".into(), 1),
        ],
        "DataFusion must group NULL dates by the remaining column"
    );

    // Verify that the transform stays attached to DATE(created_at) regardless
    // of where that expression appears in the GROUP BY list.
    assert_uses_datafusion_aggregate_scan(
        &mut conn,
        "SELECT region, DATE(created_at) AS day, COUNT(*) AS cnt \
         FROM date_pushdown_multi \
         WHERE id @@@ pdb.all() \
         GROUP BY region, DATE(created_at)",
    );

    // Verify parity with native PostgreSQL aggregation.
    "SET paradedb.enable_aggregate_custom_scan TO off;".execute(&mut conn);
    assert_uses_custom_scan(&mut conn, false, query);

    let fallback =
        format!("{query} ORDER BY day NULLS LAST, region -- fallback")
            .fetch::<(Option<Date>, String, i64)>(&mut conn);

    assert_eq!(rows, fallback, "DataFusion must match Postgres exactly");
}

#[rstest]
fn test_group_by_date_over_cast_falls_back(mut conn: PgConnection) {
    // The DataFusion transform accepts DATE() only over a bare timestamp
    // column. Parsing text as a timestamp can depend on session settings such
    // as DateStyle, so DATE(text_col::timestamp) must remain in PostgreSQL.

    r#"
      CREATE TABLE date_pushdown_cast (
          id SERIAL PRIMARY KEY,
          timestamp_text TEXT
      );
      INSERT INTO date_pushdown_cast (timestamp_text) VALUES
          ('2024-01-01 08:00:00'),
          ('2024-01-01 20:00:00'),
          ('2024-01-02 09:00:00'),
          (NULL);
      CREATE INDEX date_pushdown_cast_idx ON date_pushdown_cast
          USING paradedb (id, timestamp_text)
          WITH (key_field = 'id', text_fields = '{"timestamp_text": {"fast": true}}');
      "#
    .execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    let query = "SELECT DATE(timestamp_text::timestamp) AS day, COUNT(*) AS cnt \
                   FROM date_pushdown_cast \
                   WHERE id @@@ pdb.all() \
                   GROUP BY DATE(timestamp_text::timestamp)";

    assert_uses_custom_scan(&mut conn, false, query);

    let rows = format!("{query} ORDER BY day NULLS LAST").fetch::<(Option<Date>, i64)>(&mut conn);

    assert_eq!(
        rows,
        vec![
            (Some(date!(2024 - 01 - 01)), 2),
            (Some(date!(2024 - 01 - 02)), 1),
            (None, 1),
        ],
        "fallback must compute DATE() over the cast correctly"
    );
}
