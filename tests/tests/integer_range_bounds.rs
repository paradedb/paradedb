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

//! Queries on integer and integer range fields with bounds or values that are fractional, NaN,
//! infinite, or at or beyond the edges of the integer types return the same rows as the
//! equivalent plain SQL.
//! Timestamps are stored as i64, with '-infinity' and 'infinity' at the edges.

use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

/// Checks that each pg_search condition selects the same rows of `table` as the plain SQL
/// condition selects of `{table}_plain`, an unindexed copy, and that there are `expected` of them.
fn assert_same_rows(conn: &mut PgConnection, table: &str, cases: &[(&str, &str, usize)]) {
    for &(search, plain, expected) in cases {
        let rows: Vec<(i32,)> =
            format!("SELECT id FROM {table} WHERE {search} ORDER BY id").fetch(conn);
        let plain_rows: Vec<(i32,)> =
            format!("SELECT id FROM {table}_plain WHERE {plain} ORDER BY id").fetch(conn);
        assert_eq!(rows, plain_rows, "{search}");
        assert_eq!(rows.len(), expected, "{search}");
    }
}

#[rstest]
fn bigint_bounds(mut conn: PgConnection) {
    // -446744073709551616 is what 18000000000000000000 becomes if it wraps around as a bigint.
    r#"
    CREATE TABLE bigints (id SERIAL PRIMARY KEY, x BIGINT);
    INSERT INTO bigints (x) SELECT generate_series(1, 1000);
    INSERT INTO bigints (x) VALUES
        (-9223372036854775808), (-446744073709551616), (9223372036854775807);
    CREATE INDEX bigints_idx ON bigints USING paradedb (id, x);
    CREATE TABLE bigints_plain AS TABLE bigints;
    "#
    .execute(&mut conn);

    assert_same_rows(
        &mut conn,
        "bigints",
        &[
            // JSON bounds at and beyond the maximum.
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"included":9223372036854775807},"upper_bound":null}}'::jsonb"#,
                "x >= 9223372036854775807",
                1,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"included":9223372036854775808},"upper_bound":null}}'::jsonb"#,
                "x >= 9223372036854775808",
                0,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"excluded":9223372036854775807},"upper_bound":null}}'::jsonb"#,
                "x > 9223372036854775807",
                0,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":null,"upper_bound":{"included":9223372036854775807}}}'::jsonb"#,
                "x <= 9223372036854775807",
                1003,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":null,"upper_bound":{"included":18000000000000000000}}}'::jsonb"#,
                "x <= 18000000000000000000",
                1003,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"excluded":500},"upper_bound":{"included":600}}}'::jsonb"#,
                "x > 500 AND x <= 600",
                100,
            ),
            // pdb.range bounds at and beyond either edge.
            (
                "x @@@ pdb.range(numrange(NULL, 18000000000000000000, '[]'))",
                "x <= 18000000000000000000",
                1003,
            ),
            (
                "x @@@ pdb.range(numrange(NULL, 9223372036854775807, '[]'))",
                "x <= 9223372036854775807",
                1003,
            ),
            (
                "x @@@ pdb.range(numrange(9223372036854775807, NULL, '()'))",
                "x > 9223372036854775807",
                0,
            ),
            (
                "x @@@ pdb.range(numrange(9223372036854775807, NULL, '[]'))",
                "x >= 9223372036854775807",
                1,
            ),
            (
                "x @@@ pdb.range(numrange(18000000000000000000, NULL, '[]'))",
                "x >= 18000000000000000000",
                0,
            ),
            (
                "x @@@ pdb.range(numrange(NULL, -9223372036854775809, '[]'))",
                "x <= -9223372036854775809",
                0,
            ),
            (
                "x @@@ pdb.range(numrange(-9223372036854775809, NULL, '()'))",
                "x > -9223372036854775809",
                1003,
            ),
            (
                "x @@@ pdb.range(numrange(500, 600, '(]'))",
                "x > 500 AND x <= 600",
                100,
            ),
            // Terms beyond either edge match nothing.
            (
                r#"id @@@ '{"term":{"field":"x","value":18000000000000000000}}'::jsonb"#,
                "x = 18000000000000000000",
                0,
            ),
            (
                r#"id @@@ '{"term_set":{"terms":[{"field":"x","value":18000000000000000000},{"field":"x","value":5}]}}'::jsonb"#,
                "x IN (18000000000000000000, 5)",
                1,
            ),
            (
                "x @@@ pdb.term(18000000000000000000::numeric)",
                "x = 18000000000000000000",
                0,
            ),
            (
                "x @@@ pdb.term(-9223372036854775809::numeric)",
                "x = -9223372036854775809",
                0,
            ),
            (
                "x @@@ pdb.term_set(ARRAY[-9223372036854775809, 5, 18000000000000000000]::numeric[])",
                "x IN (-9223372036854775809, 5, 18000000000000000000)",
                1,
            ),
            // JSON bounds with a fractional part or beyond either edge as a float.
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"included":2.5},"upper_bound":null}}'::jsonb"#,
                "x >= 2.5",
                999,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":null,"upper_bound":{"excluded":2.5}}}'::jsonb"#,
                "x < 2.5",
                4,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"excluded":-2.5},"upper_bound":null}}'::jsonb"#,
                "x > -2.5",
                1001,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"included":5.0},"upper_bound":null}}'::jsonb"#,
                "x >= 5.0",
                997,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"included":-1e19},"upper_bound":null}}'::jsonb"#,
                "x >= -1e19",
                1003,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":null,"upper_bound":{"included":-1e19}}}'::jsonb"#,
                "x <= -1e19",
                0,
            ),
            // String bounds read as Postgres reads a numeric, with NaN above every number.
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"included":"1e19"},"upper_bound":null}}'::jsonb"#,
                "x >= 1e19",
                0,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"included":"2.5"},"upper_bound":null}}'::jsonb"#,
                "x >= 2.5",
                999,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"included":"NaN"},"upper_bound":null}}'::jsonb"#,
                "x >= 'NaN'::numeric",
                0,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":null,"upper_bound":{"excluded":"NaN"}}}'::jsonb"#,
                "x < 'NaN'::numeric",
                1003,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"included":"infinity"},"upper_bound":null}}'::jsonb"#,
                "x >= 'infinity'::numeric",
                0,
            ),
            (
                r#"id @@@ '{"range":{"field":"x","lower_bound":{"excluded":"-infinity"},"upper_bound":null}}'::jsonb"#,
                "x > '-infinity'::numeric",
                1003,
            ),
            // pdb.range bounds with a fractional part or at infinity.
            (
                "x @@@ pdb.range(numrange(2.5, NULL, '[]'))",
                "x >= 2.5",
                999,
            ),
            ("x @@@ pdb.range(numrange(NULL, 2.5, '[]'))", "x <= 2.5", 4),
            (
                "x @@@ pdb.range(numrange(9223372036854775806.5, NULL, '[]'))",
                "x >= 9223372036854775806.5",
                1,
            ),
            (
                "x @@@ pdb.range(numrange(1, 'Infinity', '[)'))",
                "x >= 1",
                1001,
            ),
            // Terms that no integer equals match nothing.
            ("x @@@ pdb.term(2.5::numeric)", "x = 2.5", 0),
            ("x @@@ pdb.term('NaN'::numeric)", "x = 'NaN'::numeric", 0),
            (
                "x @@@ pdb.term('Infinity'::numeric)",
                "x = 'Infinity'::numeric",
                0,
            ),
            (
                "x @@@ pdb.term(9223372036854775807.5::numeric)",
                "x = 9223372036854775807.5",
                0,
            ),
            (
                "x @@@ pdb.term_set(ARRAY[2.5, 5, 'NaN']::numeric[])",
                "x IN (2.5, 5, 'NaN')",
                1,
            ),
            (
                r#"id @@@ '{"term":{"field":"x","value":5.0}}'::jsonb"#,
                "x = 5.0",
                1,
            ),
            (
                r#"id @@@ '{"term":{"field":"x","value":"1e19"}}'::jsonb"#,
                "x = 1e19",
                0,
            ),
            (
                r#"id @@@ '{"term_set":{"terms":[{"field":"x","value":2.5},{"field":"x","value":"5"}]}}'::jsonb"#,
                "x IN (2.5, 5)",
                1,
            ),
            // A bigint constant at the maximum is pushed down as a range.
            (
                "id @@@ pdb.all() AND x > 9223372036854775807::bigint",
                "x > 9223372036854775807::bigint",
                0,
            ),
            (
                "id @@@ pdb.all() AND x <= 9223372036854775807::bigint",
                "x <= 9223372036854775807::bigint",
                1003,
            ),
        ],
    );
}

#[rstest]
fn oid_bounds(mut conn: PgConnection) {
    // An oid field holds values from 0 to 18446744073709551615 in the index.
    r#"
    CREATE TABLE oids (id SERIAL PRIMARY KEY, o OID);
    INSERT INTO oids (o) VALUES (1), (5), (4294967295);
    CREATE INDEX oids_idx ON oids USING paradedb (id, o);
    CREATE TABLE oids_plain AS TABLE oids;
    "#
    .execute(&mut conn);

    assert_same_rows(
        &mut conn,
        "oids",
        &[
            (
                r#"id @@@ '{"range":{"field":"o","lower_bound":null,"upper_bound":{"included":18446744073709551615}}}'::jsonb"#,
                "o::bigint <= 18446744073709551615",
                3,
            ),
            (
                r#"id @@@ '{"range":{"field":"o","lower_bound":{"excluded":18446744073709551615},"upper_bound":null}}'::jsonb"#,
                "o::bigint > 18446744073709551615",
                0,
            ),
            (
                "o @@@ pdb.range(numrange(-5, NULL, '[]'))",
                "o::bigint >= -5",
                3,
            ),
            (
                r#"id @@@ '{"range":{"field":"o","lower_bound":{"included":-0.5},"upper_bound":null}}'::jsonb"#,
                "o::bigint >= -0.5",
                3,
            ),
            (
                r#"id @@@ '{"range":{"field":"o","lower_bound":null,"upper_bound":{"included":"2e19"}}}'::jsonb"#,
                "o::bigint <= 2e19",
                3,
            ),
            (
                r#"id @@@ '{"range":{"field":"o","lower_bound":{"excluded":"2e19"},"upper_bound":null}}'::jsonb"#,
                "o::bigint > 2e19",
                0,
            ),
            ("o @@@ pdb.term(4.5::numeric)", "o::bigint = 4.5", 0),
        ],
    );
}

#[rstest]
fn timestamp_infinity_bounds(mut conn: PgConnection) {
    r#"
    CREATE TABLE timestamps (id SERIAL PRIMARY KEY, t TIMESTAMP);
    INSERT INTO timestamps (t) VALUES ('-infinity'), ('2024-01-01'), ('infinity');
    CREATE INDEX timestamps_idx ON timestamps USING paradedb (id, t);
    CREATE TABLE timestamps_plain AS TABLE timestamps;
    "#
    .execute(&mut conn);

    assert_same_rows(
        &mut conn,
        "timestamps",
        &[
            (
                r#"id @@@ '{"range":{"field":"t","lower_bound":{"excluded":"infinity"},"upper_bound":null}}'::jsonb"#,
                "t > 'infinity'",
                0,
            ),
            (
                r#"id @@@ '{"range":{"field":"t","lower_bound":null,"upper_bound":{"included":"infinity"}}}'::jsonb"#,
                "t <= 'infinity'",
                3,
            ),
        ],
    );
}

#[rstest]
fn segment_pruning_with_a_term_beyond_the_maximum(mut conn: PgConnection) {
    // Each insert writes its own segment, so the maximum sits alone in one.
    r#"
    CREATE TABLE pruning (id SERIAL PRIMARY KEY, x BIGINT NOT NULL);
    CREATE INDEX pruning_idx ON pruning USING paradedb (id, x)
    WITH (partition_by = 'x', background_layer_sizes = '0');
    SET paradedb.global_mutable_segment_rows = 0;
    INSERT INTO pruning (x) VALUES (5);
    INSERT INTO pruning (x) VALUES (9223372036854775807);
    CREATE TABLE pruning_plain AS TABLE pruning;
    "#
    .execute(&mut conn);

    assert_same_rows(
        &mut conn,
        "pruning",
        &[(
            "id @@@ pdb.all() AND NOT x @@@ pdb.term(18000000000000000000::numeric)",
            "NOT x = 18000000000000000000",
            2,
        )],
    );
}

#[rstest]
fn int_range_fields(mut conn: PgConnection) {
    // The ranges hold integers, so a query range holds the integers between its bounds, and one
    // that holds none is the empty range.
    r#"
    CREATE TABLE ranges (id SERIAL PRIMARY KEY, r8 INT8RANGE, r4 INT4RANGE);
    INSERT INTO ranges (r8, r4) VALUES
        ('[1,10)', '[1,10)'),
        ('[0,5)', '[0,5)'),
        ('[5,)', '[5,)'),
        ('(,5)', '(,5)'),
        ('(,)', '(,)'),
        ('empty', 'empty'),
        (NULL, NULL),
        ('[-9223372036854775808,0)', '[-2147483648,0)'),
        ('[1,9223372036854775807)', '[1,2147483647)'),
        ('[9223372036854775807,)', '[2147483647,)');
    CREATE INDEX ranges_idx ON ranges USING paradedb (id, r8, r4);
    CREATE TABLE ranges_plain AS SELECT * FROM ranges;
    "#
    .execute(&mut conn);

    assert_same_rows(
        &mut conn,
        "ranges",
        &[
            // A value lies in the ranges that hold the integer it equals.
            (
                r#"id @@@ '{"range_term":{"field":"r8","value":9223372036854775807}}'::jsonb"#,
                "r8 @> 9223372036854775807::bigint",
                3,
            ),
            (
                r#"id @@@ '{"range_term":{"field":"r8","value":-9223372036854775808}}'::jsonb"#,
                "r8 @> (-9223372036854775808)::bigint",
                3,
            ),
            (
                r#"id @@@ '{"range_term":{"field":"r8","value":5.0}}'::jsonb"#,
                "r8 @> 5::bigint",
                4,
            ),
            (
                r#"id @@@ '{"range_term":{"field":"r4","value":"2147483647"}}'::jsonb"#,
                "r4 @> 2147483647",
                3,
            ),
            // No range holds a value that no integer equals: one with a fractional part, NaN, or
            // one beyond the integers of the field.
            (
                r#"id @@@ '{"range_term":{"field":"r8","value":2.5}}'::jsonb"#,
                "false",
                0,
            ),
            (
                r#"id @@@ '{"range_term":{"field":"r4","value":"-2.5"}}'::jsonb"#,
                "false",
                0,
            ),
            ("r8 @@@ pdb.range_term('NaN'::numeric)", "false", 0),
            (
                "r8 @@@ pdb.range_term(18000000000000000000::numeric)",
                "false",
                0,
            ),
            (
                "r8 @@@ pdb.range_term(-9223372036854775809::numeric)",
                "false",
                0,
            ),
            (
                r#"id @@@ '{"range_term":{"field":"r4","value":3000000000}}'::jsonb"#,
                "false",
                0,
            ),
            // A bound that equals an integer, written as a float or a string, acts as the integer.
            (
                r#"id @@@ '{"range_intersects":{"field":"r8","lower_bound":{"excluded":4.0},"upper_bound":null}}'::jsonb"#,
                "r8 && int8range(4, NULL, '()')",
                5,
            ),
            (
                r#"id @@@ '{"range_contains":{"field":"r8","lower_bound":null,"upper_bound":{"included":"4"}}}'::jsonb"#,
                "int8range(NULL, 4, '(]') @> r8",
                4,
            ),
            (
                r#"id @@@ '{"range_within":{"field":"r8","lower_bound":{"excluded":"4.0"},"upper_bound":null}}'::jsonb"#,
                "r8 @> int8range(4, NULL, '()')",
                2,
            ),
            (
                r#"id @@@ '{"range_intersects":{"field":"r4","lower_bound":{"excluded":4.0},"upper_bound":null}}'::jsonb"#,
                "r4 && int4range(4, NULL, '()')",
                5,
            ),
            // A bound with a fractional part admits the integers on its side of it.
            (
                r#"id @@@ '{"range_contains":{"field":"r8","lower_bound":{"included":0.5},"upper_bound":{"included":10.5}}}'::jsonb"#,
                "int8range(1, 10, '[]') @> r8",
                2,
            ),
            (
                r#"id @@@ '{"range_intersects":{"field":"r8","lower_bound":{"included":2.5},"upper_bound":{"included":2.7}}}'::jsonb"#,
                "r8 && 'empty'::int8range",
                0,
            ),
            (
                r#"id @@@ '{"range_within":{"field":"r4","lower_bound":{"included":2.5},"upper_bound":{"included":2.7}}}'::jsonb"#,
                "r4 @> 'empty'::int4range",
                9,
            ),
            // No integer of the field lies above its largest one, so such a range is empty.
            (
                r#"id @@@ '{"range_intersects":{"field":"r8","lower_bound":{"excluded":9223372036854775807},"upper_bound":null}}'::jsonb"#,
                "r8 && 'empty'::int8range",
                0,
            ),
            (
                r#"id @@@ '{"range_intersects":{"field":"r8","lower_bound":{"excluded":9223372036854775806},"upper_bound":null}}'::jsonb"#,
                "r8 && int8range(9223372036854775806, NULL, '()')",
                3,
            ),
            (
                r#"id @@@ '{"range_within":{"field":"r8","lower_bound":{"excluded":9223372036854775807},"upper_bound":null}}'::jsonb"#,
                "r8 @> 'empty'::int8range",
                9,
            ),
            (
                r#"id @@@ '{"range_contains":{"field":"r8","lower_bound":{"excluded":9223372036854775807},"upper_bound":null}}'::jsonb"#,
                "'empty'::int8range @> r8",
                1,
            ),
            (
                r#"id @@@ '{"range_intersects":{"field":"r8","lower_bound":{"included":18000000000000000000},"upper_bound":null}}'::jsonb"#,
                "r8 && 'empty'::int8range",
                0,
            ),
            (
                r#"id @@@ '{"range_within":{"field":"r8","lower_bound":{"included":"NaN"},"upper_bound":null}}'::jsonb"#,
                "r8 @> 'empty'::int8range",
                9,
            ),
            (
                r#"id @@@ '{"range_intersects":{"field":"r4","lower_bound":{"excluded":2147483647},"upper_bound":null}}'::jsonb"#,
                "r4 && 'empty'::int4range",
                0,
            ),
            (
                r#"id @@@ '{"range_within":{"field":"r4","lower_bound":{"included":3000000000},"upper_bound":null}}'::jsonb"#,
                "r4 @> 'empty'::int4range",
                9,
            ),
            // A bound beyond the edges of bigint admits the integers up to the edge. No range of
            // the field ends with the largest integer included, so an upper bound of it acts as
            // one excluding it or as an unbounded one.
            (
                r#"id @@@ '{"range_intersects":{"field":"r8","lower_bound":null,"upper_bound":{"included":-10000000000000000000}}}'::jsonb"#,
                "r8 && int8range(NULL, -9223372036854775808, '()')",
                2,
            ),
            (
                r#"id @@@ '{"range_contains":{"field":"r8","lower_bound":{"included":-10000000000000000000},"upper_bound":{"included":18000000000000000000}}}'::jsonb"#,
                "int8range(-9223372036854775808, 9223372036854775807) @> r8",
                5,
            ),
            (
                r#"id @@@ '{"range_within":{"field":"r8","lower_bound":{"included":-10000000000000000000},"upper_bound":{"included":-5}}}'::jsonb"#,
                "r8 @> int8range(-9223372036854775808, -5, '[]')",
                3,
            ),
            (
                r#"id @@@ '{"range_within":{"field":"r8","lower_bound":{"included":5},"upper_bound":{"included":20000000000000000000}}}'::jsonb"#,
                "r8 @> int8range(5, NULL)",
                2,
            ),
        ],
    );
}

#[rstest]
fn non_numeric_bound_on_an_integer_field(
    mut conn: PgConnection,
    #[values("abc", "", " ", "-")] bound: &str,
) {
    r#"
    CREATE TABLE words (id SERIAL PRIMARY KEY, x BIGINT);
    INSERT INTO words (x) VALUES (1);
    CREATE INDEX words_idx ON words USING paradedb (id, x);
    "#
    .execute(&mut conn);

    let result = format!(
        r#"SELECT id FROM words WHERE id @@@ '{{"range":{{"field":"x","lower_bound":{{"included":"{bound}"}},"upper_bound":null}}}}'::jsonb"#
    )
    .fetch_result::<(i32,)>(&mut conn);
    let error = result.expect_err("a bound that is not a number should be an error");
    assert!(error.to_string().contains("is not a number"), "{error}");
}
