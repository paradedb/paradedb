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
use sqlx::postgres::types::PgRange;
use sqlx::PgConnection;
use sqlx::Row;
use std::ops::Bound;

#[rstest]
fn integer_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_int4 INTEGER,
        value_int8 BIGINT
    );

    INSERT INTO test_table (value_int4, value_int8) VALUES 
        (-1111, -11111111),
        (2222, 22222222), 
        (3333, 33333333), 
        (4444, 44444444);
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_table',
        index_name => 'test_index',
        key_field => 'id',
        numeric_fields => paradedb.field('value_int4') || paradedb.field('value_int8')
    );
    "#
    .execute(&mut conn);

    // INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_int4', range => '[2222,4444]'::int4range)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);

    // INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_int8', range => '[0,50000000)'::int8range)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
}

#[rstest]
fn float_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_float4 FLOAT4,
        value_float8 FLOAT8,
        value_numeric NUMERIC
    );

    INSERT INTO test_table (value_float4, value_float8, value_numeric) VALUES
        (-1.1, -1111.1111, -111.11111),
        (2.2, 2222.2222, 222.22222),
        (3.3, 3333.3333, 333.33333),
        (4.4, 4444.4444, 444.44444);
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_table',
        index_name => 'test_index',
        key_field => 'id',
        numeric_fields => paradedb.field('value_float4') || paradedb.field('value_float8') || paradedb.field('value_numeric')
    );
    "#
    .execute(&mut conn);

    // FLOAT4
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, value_float4 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_float4', range => '[-2,3]'::numrange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // FLOAT8
    let rows: Vec<(i32, f64)> = r#"
    SELECT id, value_float8 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_float8', range => '(2222.2222, 3333.3333]'::numrange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);

    // NUMERIC - no sqlx::Type for numerics, so just select id
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_numeric', range => '[0,400)'::numrange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn datetime_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_date DATE,
        value_timestamp TIMESTAMP,
        value_timestamptz TIMESTAMP WITH TIME ZONE
    );

    INSERT INTO test_table (value_date, value_timestamp, value_timestamptz) VALUES 
        (DATE '2023-05-03', TIMESTAMP '2023-04-15 13:27:09', TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 PST'),
        (DATE '2022-07-14', TIMESTAMP '2022-05-16 07:38:43', TIMESTAMP WITH TIME ZONE '2022-05-16 07:38:43 EST'),
        (DATE '2021-04-30', TIMESTAMP '2021-06-08 08:49:21', TIMESTAMP WITH TIME ZONE '2021-06-08 08:49:21 CST'),
        (DATE '2020-06-28', TIMESTAMP '2020-07-09 15:52:13', TIMESTAMP WITH TIME ZONE '2020-07-09 15:52:13 MST');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_table',
        index_name => 'test_index',
        key_field => 'id',
        datetime_fields => paradedb.field('value_date') || 
                           paradedb.field('value_timestamp') || 
                           paradedb.field('value_timestamptz')
    );
    "#
    .execute(&mut conn);

    // DATE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
        paradedb.range(field => 'value_date', range => '[2020-05-20,2022-06-13]'::daterange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // TIMESTAMP
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
        paradedb.range(field => 'value_timestamp', range => '[2019-08-02 07:52:43, 2021-06-10 10:32:41]'::tsrange)
    ORDER BY id"#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // TIMESTAMP WITH TIME ZONE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
        paradedb.range(field => 'value_timestamptz', range => '[2020-07-09 17:52:13 EST, 2022-05-16 04:38:43 PST]'::tstzrange)
    ORDER BY id"#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
}

#[derive(Clone, Debug, PartialEq)]
enum BoundType {
    Included,
    Excluded,
    Unbounded,
}

impl BoundType {
    fn to_bound<T>(self, val: T) -> Bound<T> {
        match self {
            BoundType::Included => Bound::Included(val),
            BoundType::Excluded => Bound::Excluded(val),
            BoundType::Unbounded => Bound::Unbounded,
        }
    }
}

#[rstest]
async fn range_term_contains_int4range(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'deliveries',
        table_type => 'Deliveries'
    );

    CALL paradedb.create_bm25(
        index_name => 'deliveries',
        table_name => 'deliveries',
        key_field => 'delivery_id',
        range_fields => 
            paradedb.field('weights') || 
            paradedb.field('quantities') || 
            paradedb.field('prices') || 
            paradedb.field('ship_dates') ||
            paradedb.field('facility_arrival_times') ||
            paradedb.field('delivery_times')
    );
    "#
    .execute(&mut conn);

    let bound_types = vec![
        BoundType::Included,
        BoundType::Excluded,
        BoundType::Unbounded,
    ];

    let lower_bounds = vec![2, 10];
    let upper_bound = 10;

    for lower_bound_type in &bound_types {
        for upper_bound_type in &bound_types {
            for lower_bound in &lower_bounds {
                let range = PgRange {
                    start: lower_bound_type.clone().to_bound(*lower_bound),
                    end: upper_bound_type.clone().to_bound(upper_bound),
                };

                sqlx::query("INSERT INTO deliveries (weights) VALUES ($1)")
                    .bind(range)
                    .execute(&mut conn)
                    .await
                    .unwrap();
            }
        }
    }

    let lower_bounds = vec![1, 2, 3];
    let upper_bounds = vec![9, 10, 11];

    for lower_bound_type in &bound_types {
        for upper_bound_type in &bound_types {
            for lower_bound in &lower_bounds {
                for upper_bound in &upper_bounds {
                    let range = PgRange {
                        start: lower_bound_type.clone().to_bound(*lower_bound),
                        end: upper_bound_type.clone().to_bound(*upper_bound),
                    };

                    let expected: Vec<i32> = sqlx::query(&format!("SELECT delivery_id FROM deliveries WHERE '{}'::int4range @> weights ORDER BY delivery_id", range))
                        .fetch_all(&mut conn)
                        .await
                        .unwrap()
                        .into_iter()
                        .map(|row| row.get::<i32, _>("delivery_id"))
                        .collect();

                    let result: Vec<i32> = sqlx::query(&format!("SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('weights', '{}'::int4range, 'Contains') ORDER BY delivery_id", range))
                        .fetch_all(&mut conn)
                        .await
                        .unwrap()
                        .into_iter()
                        .map(|row| row.get::<i32, _>("delivery_id"))
                        .collect();

                    assert_eq!(expected, result, "query failed for range: {:?}", range);
                }
            }
        }
    }
}
