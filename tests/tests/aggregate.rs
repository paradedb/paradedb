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

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn test_aggregate_with_mvcc(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
    CREATE INDEX idxbm25_search ON paradedb.bm25_search
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
    WITH (
        key_field='id',
        text_fields='{
            "category": {"fast": true, "normalizer": "raw"}
        }',
        numeric_fields='{"rating": {"fast": true}}'
    );
    INSERT INTO paradedb.bm25_search (description, category, rating) VALUES
        ('keyboard', 'Electronics', 4.5),
        ('keyboard', 'Electronics', 3.8),
        ('keyboard', 'Accessories', 4.2);

    DELETE FROM paradedb.bm25_search WHERE category = 'Accessories';
    "#
        .execute(&mut conn);

    // Test with MVCC enabled (default)
    let result = r#"
    SELECT paradedb.aggregate(
        'paradedb.idxbm25_search',
        paradedb.parse('description:keyboard'),
        '{
            "category": {
                "terms": {
                    "field": "category",
                    "size": 10
                }
            }
        }'::json
    )
    "#
    .fetch_one::<(serde_json::Value,)>(&mut conn);

    // Verify the aggregation results
    let buckets = result
        .0
        .pointer("/category/buckets")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(buckets.len(), 1); // Should have 1 category
}

#[rstest]
fn test_aggregate_without_mvcc(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
    CREATE INDEX idxbm25_search ON paradedb.bm25_search
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
    WITH (
        key_field='id',
        text_fields='{
            "description": {},
            "category": {"fast": true, "normalizer": "raw"}
        }',
        numeric_fields='{"rating": {"fast": true}}',
        boolean_fields='{"in_stock": {}}',
        json_fields='{"metadata": {}}',
        datetime_fields='{
            "created_at": {},
            "last_updated_date": {},
            "latest_available_time": {}
        }'
    );
    INSERT INTO paradedb.bm25_search (description, category, rating) VALUES
        ('keyboard', 'Electronics', 4.5),
        ('keyboard', 'Electronics', 3.8),
        ('keyboard', 'Accessories', 4.2);

    DELETE FROM paradedb.bm25_search WHERE category = 'Accessories';
    "#
        .execute(&mut conn);

    // Test with MVCC disabled
    let result = r#"
    SELECT paradedb.aggregate(
        'paradedb.idxbm25_search',
        paradedb.parse('description:keyboard'),
        '{
            "category": {
                "terms": {
                    "field": "category",
                    "size": 10
                }
            }
        }'::json,
        false
    )
    "#
    .fetch_one::<(serde_json::Value,)>(&mut conn);

    // Verify the aggregation results
    let buckets = result
        .0
        .pointer("/category/buckets")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(buckets.len(), 2); // Should have 2 categories
}
