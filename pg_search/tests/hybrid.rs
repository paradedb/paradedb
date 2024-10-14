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
use sqlx::types::BigDecimal;
use sqlx::PgConnection;
use std::str::FromStr;

#[rstest]
fn hybrid_deprecated(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    r#"
    CREATE EXTENSION vector;
    ALTER TABLE paradedb.bm25_search ADD COLUMN embedding vector(3);

    UPDATE paradedb.bm25_search m
    SET embedding = ('[' ||
    ((m.id + 1) % 10 + 1)::integer || ',' ||
    ((m.id + 2) % 10 + 1)::integer || ',' ||
    ((m.id + 3) % 10 + 1)::integer || ']')::vector;

    CREATE INDEX on paradedb.bm25_search
    USING hnsw (embedding vector_l2_ops)"#
        .execute(&mut conn);

    // Test with string query.
    let columns: SimpleProductsTableVec = r#"
    SELECT m.*, s.score_hybrid
    FROM paradedb.bm25_search m
    LEFT JOIN (
        SELECT * FROM bm25_search.score_hybrid(
            bm25_query => 'description:keyboard OR category:electronics',
            similarity_query => '''[1,2,3]'' <-> embedding',
            bm25_weight => 0.9,
            similarity_weight => 0.1
        )
    ) s ON m.id = s.id
    LIMIT 5"#
        .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![2, 1, 29, 39, 9]);

    // New score_hybrid function
    // Test with query object.
    let columns: SimpleProductsTableVec = r#"
    SELECT m.*, s.score_hybrid
    FROM paradedb.bm25_search m
    LEFT JOIN (
        SELECT * FROM bm25_search.score_hybrid(
            bm25_query => paradedb.parse('description:keyboard OR category:electronics'),
            similarity_query => '''[1,2,3]'' <-> embedding',
            bm25_weight => 0.9,
            similarity_weight => 0.1
        )
    ) s ON m.id = s.id
    LIMIT 5"#
        .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![2, 1, 29, 39, 9]);

    // Test with string query.
    let columns: SimpleProductsTableVec = r#"
    SELECT m.*, s.score_hybrid
    FROM paradedb.bm25_search m
    LEFT JOIN (
        SELECT * FROM bm25_search.score_hybrid(
            bm25_query => 'description:keyboard OR category:electronics',
            similarity_query => '''[1,2,3]'' <-> embedding',
            bm25_weight => 0.9,
            similarity_weight => 0.1
        )
    ) s ON m.id = s.id
    LIMIT 5"#
        .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![2, 1, 29, 39, 9]);
}

#[rstest]
#[allow(clippy::excessive_precision)]
fn reciprocal_rank_fusion(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    r#"
    CREATE EXTENSION vector;
    ALTER TABLE paradedb.bm25_search ADD COLUMN embedding vector(3);

    UPDATE paradedb.bm25_search m
    SET embedding = ('[' ||
    ((m.id + 1) % 10 + 1)::integer || ',' ||
    ((m.id + 2) % 10 + 1)::integer || ',' ||
    ((m.id + 3) % 10 + 1)::integer || ']')::vector;

    CREATE INDEX on paradedb.bm25_search
    USING hnsw (embedding vector_l2_ops)"#
        .execute(&mut conn);

    let columns: Vec<(i32, BigDecimal, String)> = r#"
    WITH semantic AS (
        SELECT id, RANK () OVER (ORDER BY embedding <=> '[1,2,3]') AS rank
        FROM paradedb.bm25_search
        ORDER BY embedding <=> '[1,2,3]'
        LIMIT 20
    ),
    bm25 AS (
        SELECT id, RANK () OVER (ORDER BY paradedb.score(id) DESC) as rank
        FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' LIMIT 20
    )
    SELECT
        COALESCE(semantic.id, bm25.id) AS id,
        (COALESCE(1.0 / (60 + semantic.rank), 0.0) +
        COALESCE(1.0 / (60 + bm25.rank), 0.0))::NUMERIC AS score,
        paradedb.bm25_search.description
    FROM semantic
    FULL OUTER JOIN bm25 ON semantic.id = bm25.id
    JOIN paradedb.bm25_search ON paradedb.bm25_search.id = COALESCE(semantic.id, bm25.id)
    ORDER BY score DESC
    LIMIT 5;
    "#
    .fetch(&mut conn);

    assert_eq!(
        columns[0],
        (
            1,
            BigDecimal::from_str("0.03088619624613922547").unwrap(),
            "Ergonomic metal keyboard".to_string()
        )
    );
    assert_eq!(
        columns[1],
        (
            2,
            BigDecimal::from_str("0.02964254577157802964").unwrap(),
            "Plastic Keyboard".to_string()
        )
    );
    assert_eq!(
        columns[2],
        (
            19,
            BigDecimal::from_str("0.01639344262295081967").unwrap(),
            "Artistic ceramic vase".to_string()
        )
    );
    assert_eq!(
        columns[3],
        (
            9,
            BigDecimal::from_str("0.01639344262295081967").unwrap(),
            "Modern wall clock".to_string()
        )
    );
    assert_eq!(
        columns[4],
        (
            29,
            BigDecimal::from_str("0.01639344262295081967").unwrap(),
            "Designer wall paintings".to_string()
        )
    );
}
