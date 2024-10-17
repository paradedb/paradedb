// Copyright (c) 2023-2024 Retake, Inc.
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
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

#[rstest]
fn sort_by_lower(mut conn: PgConnection) {
    // ensure our custom scan wins against our small test table
    r#"
        SET enable_indexscan TO off;
        CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

        CALL paradedb.create_bm25(
            index_name => 'bm25_search',
            table_name => 'bm25_search',
            schema_name => 'paradedb',
            key_field => 'id',
            text_fields => paradedb.field('description') || paradedb.field('category', fast=>true, normalizer=>'lowercase'),
            numeric_fields => paradedb.field('rating'),
            boolean_fields => paradedb.field('in_stock'),
            json_fields => paradedb.field('metadata'),
            datetime_fields => paradedb.field('created_at') || paradedb.field('last_updated_date') || paradedb.field('latest_available_time')        
        );        
    "#.execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE description @@@ 'keyboard OR shoes' ORDER BY lower(category) LIMIT 5".fetch_one::<(Value,)>(&mut conn);
    let plan = plan
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("Plan")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Plans")
        .unwrap()
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_object()
        .unwrap();
    eprintln!("{plan:#?}");
    assert_eq!(
        plan.get("   Sort Field"),
        Some(&Value::String(String::from("category")))
    );
}

#[rstest]
fn sort_by_raw(mut conn: PgConnection) {
    // ensure our custom scan wins against our small test table
    r#"
        SET enable_indexscan TO off;
        CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

        CALL paradedb.create_bm25(
            index_name => 'bm25_search',
            table_name => 'bm25_search',
            schema_name => 'paradedb',
            key_field => 'id',
            text_fields => paradedb.field('description') || paradedb.field('category', fast=>true, normalizer=>'raw'),
            numeric_fields => paradedb.field('rating'),
            boolean_fields => paradedb.field('in_stock'),
            json_fields => paradedb.field('metadata'),
            datetime_fields => paradedb.field('created_at') || paradedb.field('last_updated_date') || paradedb.field('latest_available_time')        
        );        
    "#.execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE description @@@ 'keyboard OR shoes' ORDER BY category LIMIT 5".fetch_one::<(Value,)>(&mut conn);
    let plan = plan
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("Plan")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Plans")
        .unwrap()
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_object()
        .unwrap();
    eprintln!("{plan:#?}");
    assert_eq!(
        plan.get("   Sort Field"),
        Some(&Value::String(String::from("category")))
    );
}

#[rstest]
fn sort_by_row_return_scores(mut conn: PgConnection) {
    // ensure our custom scan wins against our small test table
    r#"
        SET enable_indexscan TO off;
        CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

        CALL paradedb.create_bm25(
            index_name => 'bm25_search',
            table_name => 'bm25_search',
            schema_name => 'paradedb',
            key_field => 'id',
            text_fields => paradedb.field('description') || paradedb.field('category', fast=>true, normalizer=>'raw'),
            numeric_fields => paradedb.field('rating'),
            boolean_fields => paradedb.field('in_stock'),
            json_fields => paradedb.field('metadata'),
            datetime_fields => paradedb.field('created_at') || paradedb.field('last_updated_date') || paradedb.field('latest_available_time')        
        );        
    "#.execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT paradedb.score(id), * FROM paradedb.bm25_search WHERE description @@@ 'keyboard OR shoes' ORDER BY category LIMIT 5".fetch_one::<(Value,)>(&mut conn);
    let plan = plan
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("Plan")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Plans")
        .unwrap()
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("Plans")
        .unwrap()
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_object()
        .unwrap();
    eprintln!("{plan:#?}");
    assert_eq!(plan.get("   Sort Field"), None);
    assert_eq!(plan.get("Scores"), Some(&Value::Bool(true)));
}
