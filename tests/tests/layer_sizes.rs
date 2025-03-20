// Copyright (c) 2023-2025 Retake, Inc.
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

use bigdecimal::BigDecimal;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn one_layer_size(mut conn: PgConnection) {
    let result = r#"
        CREATE TABLE layer_sizes (id serial8 not null primary key);
        CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '1kb');
    "#.execute_result(&mut conn);

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(
        err.into_database_error().unwrap().to_string(),
        "There must be at least 2 layers in `layer_sizes`"
    );
}

#[rstest]
fn zero_layer_size(mut conn: PgConnection) {
    let result = r#"
        CREATE TABLE layer_sizes (id serial8 not null primary key);
        CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '0kb, 1kb');
    "#.execute_result(&mut conn);

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(
        err.into_database_error().unwrap().to_string(),
        "a single layer size must be greater than zero"
    );
}

#[rstest]
fn malformed_layer_size(mut conn: PgConnection) {
    let result = r#"
        CREATE TABLE layer_sizes (id serial8 not null primary key);
        CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '1kb, bob''s your uncle');
    "#.execute_result(&mut conn);

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(
        err.into_database_error().unwrap().to_string(),
        "invalid size: \" bob's your uncle\""
    );
}

#[rstest]
fn all_good_layer_sizes(mut conn: PgConnection) {
    let result = r#"
        CREATE TABLE layer_sizes (id serial8 not null primary key);
        CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '1kb, 10kb, 100MB');
    "#.execute_result(&mut conn);

    assert!(result.is_ok());
}

#[rstest]
fn default_layer_sizes(mut conn: PgConnection) {
    let result = r#"
        CREATE TABLE layer_sizes (id serial8 not null primary key);
        CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id');
    "#
    .execute_result(&mut conn);
    assert!(result.is_ok());

    let (layer_sizes,) = r#"SELECT paradedb.layer_sizes('idxlayer_sizes')"#
        .fetch_one::<(Vec<BigDecimal>,)>(&mut conn);
    assert_eq!(
        layer_sizes,
        vec![
            BigDecimal::from(100 * 1024),
            BigDecimal::from(1 * 1024 * 1024),
            BigDecimal::from(100 * 1024 * 1024)
        ]
    );
}

#[rstest]
fn layer_sizes_after_alter(mut conn: PgConnection) {
    r#"
        CREATE TABLE layer_sizes (id serial8 not null primary key);
        CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id');
        ALTER INDEX idxlayer_sizes SET (layer_sizes = '2kb, 3kb, 4kb');
    "#
    .execute(&mut conn);

    let (layer_sizes,) = r#"SELECT paradedb.layer_sizes('idxlayer_sizes')"#
        .fetch_one::<(Vec<BigDecimal>,)>(&mut conn);
    assert_eq!(
        layer_sizes,
        vec![
            BigDecimal::from(2 * 1024),
            BigDecimal::from(3 * 1024),
            BigDecimal::from(4 * 1024)
        ]
    );
}
