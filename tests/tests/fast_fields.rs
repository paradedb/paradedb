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
use serde_json::Value;
use sqlx::PgConnection;

#[rstest]
fn plans_numeric_fast_field(mut conn: PgConnection) {
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
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT rating FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);

    assert_eq!(
        Some(&Value::String("rating".into())),
        plan.pointer("/0/Plan/Fast Fields")
    )
}

#[rstest]
fn plans_many_numeric_fast_fields(mut conn: PgConnection) {
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
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT id, rating FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);

    assert_eq!(
        Some(&Value::String("id, rating".into())),
        plan.pointer("/0/Plan/Fast Fields")
    )
}

#[rstest]
fn plans_many_numeric_fast_fields_with_score(mut conn: PgConnection) {
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
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT id, paradedb.score(id), rating FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);
    assert_eq!(
        Some(&Value::String("id, paradedb.score(), rating".into())),
        plan.pointer("/0/Plan/Fast Fields")
    )
}

// string "fast fields" are only supported as part of an aggregate query.  They're basically slower
// in all other cases
#[rstest]
fn plans_string_fast_field(mut conn: PgConnection) {
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
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT category, count(*) FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard' GROUP BY category".fetch_one::<(Value,)>(&mut conn);
    assert_eq!(
        Some(&Value::String("category".into())),
        plan.pointer("/0/Plan/Plans/0/Plans/0/String Fast Fields")
    )
}

// only selecting a string field does use a "fast field"-style plan
#[rstest]
fn does_plan_string_fast_field(mut conn: PgConnection) {
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
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT category FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);
    assert_eq!(
        Some(&Value::String("Custom Scan".into())),
        plan.pointer("/0/Plan/Node Type")
    )
}

#[ignore = "figure out why query plan changed"]
#[rstest]
fn numeric_fast_field_in_window_func(mut conn: PgConnection) {
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
    "#
    .execute(&mut conn);

    let (plan,) = r#"EXPLAIN (ANALYZE, FORMAT JSON)
    WITH RankedContacts AS (
        SELECT id,
               ROW_NUMBER() OVER (PARTITION BY rating ORDER BY id) AS rn
        FROM paradedb.bm25_search
        WHERE id @@@ 'description:shoes'
        )
    SELECT id
    FROM RankedContacts
    WHERE rn <= 10
    LIMIT 100 OFFSET 100;
    "#
    .fetch_one::<(Value,)>(&mut conn);
    assert_eq!(
        Some(&Value::String("NumericFastFieldExecState".into())),
        plan.pointer("/0/Plan/Plans/0/Plans/0/Plans/0/Plans/Exec Method")
    )
}
