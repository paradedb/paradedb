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
use sqlx::{PgConnection, types::BigDecimal};
use tests::fixtures::*;

#[rstest]
fn hybrid_deprecated(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_paradedb_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING paradedb (id, description, category, rating, in_stock, created_at, metadata);

    CREATE EXTENSION IF NOT EXISTS vector;

    UPDATE mock_items m
    SET embedding = ('[' ||
        ((m.id + 1) % 10 + 1)::integer || ',' ||
        ((m.id + 2) % 10 + 1)::integer || ',' ||
        ((m.id + 3) % 10 + 1)::integer || ',0,0,0,0,0]')::vector;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, BigDecimal)> = r#"
    WITH semantic_search AS (
        SELECT id, RANK () OVER (ORDER BY embedding <=> '[1,2,3,0,0,0,0,0]') AS rank
        FROM mock_items ORDER BY embedding <=> '[1,2,3,0,0,0,0,0]' LIMIT 20
    ),
    bm25_search AS (
        SELECT id, RANK () OVER (ORDER BY pdb.score(id) DESC) as rank
        FROM mock_items WHERE description ||| 'keyboard' LIMIT 20
    )
    SELECT
        COALESCE(semantic_search.id, bm25_search.id) AS id,
        (COALESCE(1.0 / (60 + semantic_search.rank), 0.0) * 0.1) +
        (COALESCE(1.0 / (60 + bm25_search.rank), 0.0) * 0.9) AS score
    FROM semantic_search
    FULL OUTER JOIN bm25_search ON semantic_search.id = bm25_search.id
    JOIN mock_items ON mock_items.id = COALESCE(semantic_search.id, bm25_search.id)
    ORDER BY score DESC
    LIMIT 5
    "#
    .fetch(&mut conn);

    assert_eq!(
        rows.into_iter().map(|t| t.0).collect::<Vec<_>>(),
        vec![2, 1, 19, 9, 29]
    );
}

#[rstest]
#[allow(clippy::excessive_precision)]
fn reciprocal_rank_fusion(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    r#"
    CREATE EXTENSION IF NOT EXISTS vector;

    UPDATE paradedb.bm25_search m
    SET embedding = ('[' ||
    ((m.id + 1) % 10 + 1)::integer || ',' ||
    ((m.id + 2) % 10 + 1)::integer || ',' ||
    ((m.id + 3) % 10 + 1)::integer || ',0,0,0,0,0]')::vector;

    CREATE INDEX on paradedb.bm25_search
    USING hnsw (embedding vector_l2_ops)"#
        .execute(&mut conn);

    let columns: Vec<(i32, f32, String)> = r#"
    WITH semantic AS (
        SELECT id, RANK () OVER (ORDER BY embedding <=> '[1,2,3,0,0,0,0,0]') AS rank
        FROM paradedb.bm25_search
        ORDER BY embedding <=> '[1,2,3,0,0,0,0,0]'
        LIMIT 20
    ),
    bm25 AS (
        SELECT id, RANK () OVER (ORDER BY pdb.score(id) DESC) as rank
        FROM paradedb.bm25_search WHERE description ||| 'keyboard' LIMIT 20
    )
    SELECT
        COALESCE(semantic.id, bm25.id) AS id,
        (COALESCE(1.0 / (60 + semantic.rank), 0.0) +
        COALESCE(1.0 / (60 + bm25.rank), 0.0))::REAL AS score,
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
            0.03062178588125292193,
            "Ergonomic metal keyboard".to_string()
        )
    );
    assert_eq!(
        columns[1],
        (2, 0.02990695613646433318, "Plastic Keyboard".to_string())
    );
    assert_eq!(
        columns[2],
        (
            19,
            0.01639344262295081967,
            "Artistic ceramic vase".to_string()
        )
    );
    assert_eq!(
        columns[3],
        (9, 0.01639344262295081967, "Modern wall clock".to_string())
    );
    assert_eq!(
        columns[4],
        (
            29,
            0.01639344262295081967,
            "Designer wall paintings".to_string()
        )
    );
}

#[rstest]
fn vector_in_custom_schema(database: Db) {
    let mut conn = async_std::task::block_on(database.connection());
    r#"
    CREATE SCHEMA "Vector Extensions";
    CREATE EXTENSION vector SCHEMA "Vector Extensions";
    CREATE EXTENSION pg_search;
    "#
    .execute(&mut conn);

    // Exercise the installed extension from a new session.
    let mut conn = async_std::task::block_on(database.connection());
    r#"
    CREATE TABLE custom_vectors (id integer PRIMARY KEY, embedding "Vector Extensions".vector(3));
    INSERT INTO custom_vectors VALUES (1, '[1,0,0]'), (2, '[0,1,0]');
    CREATE INDEX custom_vectors_idx ON custom_vectors
    USING paradedb (id, embedding "Vector Extensions".vector_cosine_ops);
    "#
    .execute(&mut conn);

    let (exists,) = r#"
        SELECT pg_catalog.to_regprocedure(
            'paradedb.vector_estimator_info(regclass,text,"Vector Extensions".vector[])'
        ) IS NOT NULL
    "#
    .fetch_one::<(bool,)>(&mut conn);
    assert!(exists);

    let query = r#"
        SELECT id FROM custom_vectors WHERE id @@@ pdb.all()
        ORDER BY embedding OPERATOR("Vector Extensions".<=>) '[0,1,0]'
        LIMIT 1
    "#;
    let plan: Vec<(String,)> = format!("EXPLAIN (COSTS OFF) {query}").fetch(&mut conn);
    assert!(plan.iter().any(|(line,)| line.contains("TopK Order By")));
    let (id,) = query.fetch_one::<(i32,)>(&mut conn);
    assert_eq!(id, 2);
}
