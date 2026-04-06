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

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

/// Helper to create the ltree extension (it's a contrib module, should be available).
fn setup_ltree(conn: &mut PgConnection) {
    "CREATE EXTENSION IF NOT EXISTS ltree;".execute(conn);
}

#[rstest]
fn ltree_basic_index_and_search(mut conn: PgConnection) {
    setup_ltree(&mut conn);

    r#"
    CREATE TABLE test_ltree (
        id SERIAL PRIMARY KEY,
        path ltree
    );

    INSERT INTO test_ltree (path) VALUES
        ('Top.Science.Astronomy'),
        ('Top.Science.Astronomy.Astrophysics'),
        ('Top.Science.Astronomy.Cosmology'),
        ('Top.Collections.Pictures.Astronomy'),
        ('Top.Hobbies.Amateurs_Astronomy');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_ltree_idx ON test_ltree
    USING bm25 (id, path) WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Search for an exact ltree path using term query
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_ltree
    WHERE test_ltree @@@ paradedb.term(field => 'path', value => 'Top.Science.Astronomy')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);
}

#[rstest]
fn ltree_returns_correct_value(mut conn: PgConnection) {
    setup_ltree(&mut conn);

    r#"
    CREATE TABLE test_ltree (
        id SERIAL PRIMARY KEY,
        path ltree
    );

    INSERT INTO test_ltree (path) VALUES
        ('Top.Science.Astronomy'),
        ('Top.Hobbies.Amateurs_Astronomy');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_ltree_idx ON test_ltree
    USING bm25 (id, path) WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Verify ltree value is returned correctly (exercises try_into_datum for ltree)
    let rows: Vec<(i32, String)> = r#"
    SELECT id, path::text FROM test_ltree
    WHERE test_ltree @@@ paradedb.term(field => 'path', value => 'Top.Science.Astronomy')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1, "Top.Science.Astronomy".to_string())]);
}

#[rstest]
fn ltree_multiple_matches(mut conn: PgConnection) {
    setup_ltree(&mut conn);

    r#"
    CREATE TABLE test_ltree (
        id SERIAL PRIMARY KEY,
        category ltree
    );

    INSERT INTO test_ltree (category) VALUES
        ('Electronics.Phones.Mobile'),
        ('Electronics.Phones.Landline'),
        ('Electronics.Computers.Laptop'),
        ('Electronics.Phones.Mobile');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_ltree_idx ON test_ltree
    USING bm25 (id, category) WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_ltree
    WHERE test_ltree @@@ paradedb.term(field => 'category', value => 'Electronics.Phones.Mobile')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,), (4,)]);
}

#[rstest]
fn ltree_with_parse_query(mut conn: PgConnection) {
    setup_ltree(&mut conn);

    r#"
    CREATE TABLE test_ltree (
        id SERIAL PRIMARY KEY,
        path ltree,
        description TEXT
    );

    INSERT INTO test_ltree (path, description) VALUES
        ('Top.Science.Astronomy', 'Study of celestial objects'),
        ('Top.Science.Biology', 'Study of living organisms'),
        ('Top.Hobbies.Reading', 'Recreational reading');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_ltree_idx ON test_ltree
    USING bm25 (id, path, description) WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Use parse query on the ltree field
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_ltree
    WHERE test_ltree @@@ paradedb.parse(query_string => 'path:Top.Science.Biology')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);
}

#[rstest]
fn ltree_combined_with_text_search(mut conn: PgConnection) {
    setup_ltree(&mut conn);

    r#"
    CREATE TABLE test_ltree (
        id SERIAL PRIMARY KEY,
        path ltree,
        title TEXT
    );

    INSERT INTO test_ltree (path, title) VALUES
        ('Products.Electronics.Phones', 'Smartphone Guide'),
        ('Products.Electronics.Laptops', 'Laptop Review'),
        ('Products.Books.Fiction', 'Novel Recommendations');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_ltree_idx ON test_ltree
    USING bm25 (id, path, title) WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Boolean query combining ltree and text search
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_ltree
    WHERE test_ltree @@@ paradedb.boolean(
        must => ARRAY[
            paradedb.term(field => 'path', value => 'Products.Electronics.Phones'),
            paradedb.parse(query_string => 'title:Smartphone')
        ]
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);
}
