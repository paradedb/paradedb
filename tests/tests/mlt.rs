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

use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

#[rstest]
fn mlt_enables_scoring_issue1747(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id,) = "
    SELECT id FROM paradedb.bm25_search WHERE id @@@ pdb.more_like_this(
        key_value => 3,
        min_term_frequency => 1
    ) ORDER BY id LIMIT 1"
        .fetch_one::<(i32,)>(&mut conn);
    assert_eq!(id, 3);
}

#[rstest]
fn mlt_datetime_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE more_like_this_dt (
    id INT PRIMARY KEY,
    created_at TIMESTAMP
    );
    INSERT INTO more_like_this_dt (id, created_at) VALUES
    (1, '2012-01-01 00:00:00'),
    (2, '2013-01-03 00:00:00'),
    (3, '2014-01-02 00:00:00'),
    (4, '2015-01-01 00:00:00');
    "#
    .execute(&mut conn);

    r#"CREATE INDEX test_more_like_this_index on more_like_this_dt USING bm25 (id, created_at)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<i32> = r#"
    SELECT id FROM more_like_this_dt WHERE id @@@ pdb.more_like_this(
        document => '{"created_at": "2013-01-03 00:00:00"}'
    );
    "#
    .fetch_scalar(&mut conn);
    assert_eq!(rows, vec![2]);
}

// Regression test for https://github.com/paradedb/paradedb/issues/5065:
// pdb.more_like_this used to construct its internal SPI SELECT with the key_field
// column name unquoted, so a mixed-case identifier like "Id" was folded to lower
// case and triggered `column "id" does not exist`.
#[rstest]
fn mlt_mixed_case_key_field_issue5065(mut conn: PgConnection) {
    r#"
    CREATE TABLE mlt_mixed_case (
        "Id" SERIAL PRIMARY KEY,
        "Content" TEXT
    );
    INSERT INTO mlt_mixed_case ("Content") VALUES
        ('aaa bbb ccc'),
        ('aaa aaa'),
        ('ddd eee fff'),
        ('aaa aaa');
    CREATE INDEX mlt_mixed_case_idx ON mlt_mixed_case
        USING bm25 ("Id", "Content")
        WITH (key_field = 'Id');
    "#
    .execute(&mut conn);

    let rows: Vec<i32> = r#"
    SELECT "Id" FROM mlt_mixed_case
    WHERE "Id" @@@ pdb.more_like_this(1)
    ORDER BY "Id";
    "#
    .fetch_scalar(&mut conn);
    assert!(
        !rows.is_empty(),
        "more_like_this on a mixed-case key_field should return matches, got none"
    );
    assert!(
        rows.contains(&1),
        "more_like_this(1) should include the seed document, got {rows:?}"
    );
}

#[rstest]
fn mlt_scoring_nested(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Boolean must
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.boolean(
        must => pdb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // Boolean must_not
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.boolean(
        must_not => pdb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert!(results.is_empty());

    // Boolean should
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.boolean(
        should => pdb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // Boost
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.boost(
        factor => 1.5,
        query => pdb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // ConstScore
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.const_score(
        score => 5,
        query => pdb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // DisjunctionMax
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.disjunction_max(
        disjuncts => ARRAY[
            pdb.more_like_this(
                min_doc_frequency => 2,
                min_term_frequency => 1,
                document => '{"description": "keyboard"}'
            ),
            pdb.more_like_this(
                min_doc_frequency => 2,
                min_term_frequency => 1,
                document => '{"description": "shoes"}'
            )
        ]
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2, 3, 4, 5]);

    // Multiple nested
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.boolean(
        must_not => paradedb.parse('description:plastic'),
        should => paradedb.disjunction_max(
            disjuncts => ARRAY[
                paradedb.boost(
                    factor => 3,
                    query => pdb.more_like_this(
                        min_doc_frequency => 2,
                        min_term_frequency => 1,
                        document => '{"description": "keyboard"}'
                    )
                ),
                pdb.more_like_this(
                    min_doc_frequency => 2,
                    min_term_frequency => 1,
                    document => '{"description": "shoes"}'
                )
            ]
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 3, 4, 5]);
}
