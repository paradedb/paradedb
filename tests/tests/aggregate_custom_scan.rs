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

// Tests for ParadeDB's Aggregate Custom Scan implementation
mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

fn assert_uses_custom_scan(conn: &mut PgConnection, enabled: bool, query: impl AsRef<str>) {
    let (plan,) = format!(" EXPLAIN (FORMAT JSON) {}", query.as_ref()).fetch_one::<(Value,)>(conn);
    eprintln!("{plan:#?}");
    assert_eq!(
        enabled,
        plan.to_string().contains("ParadeDB Aggregate Scan")
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
fn test_group_by(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    // Cannot use aggregate scan with GROUP BY yet.
    assert_uses_custom_scan(
        &mut conn,
        false,
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
fn test_no_bm25_index(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'no_bm25', schema_name => 'paradedb');"
        .execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    // Do not use the aggregate custom scan on non-bm25 indexed tables.
    assert_uses_custom_scan(&mut conn, false, "SELECT COUNT(*) FROM paradedb.no_bm25");
}

#[rstest]
fn test_other_aggregates(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    // Do not use the aggregate custom scan for aggregates that we do not support yet.
    for aggregate_func in ["SUM(rating)", "AVG(rating)", "MIN(rating)", "MAX(rating)"] {
        assert_uses_custom_scan(
            &mut conn,
            false,
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
