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

use approx::assert_relative_eq;
use fixtures::*;
use pgvector::Vector;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn quickstart(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    )
    "#
    .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    LIMIT 3;
    "#
    .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("Ergonomic metal keyboard".into(), 4, "Electronics".into()),
            ("Plastic Keyboard".into(), 4, "Electronics".into()),
            ("Sleek running shoes".into(), 5, "Footwear".into())
        ]
    );

    r#"
    CALL paradedb.create_bm25(
            index_name => 'search_idx',
            schema_name => 'public',
            table_name => 'mock_items',
            key_field => 'id',
            text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category')
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM search_idx.search('description:keyboard OR category:electronics', stable_sort => true, limit_rows => 5);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].0, "Plastic Keyboard".to_string());
    assert_eq!(rows[1].0, "Ergonomic metal keyboard".to_string());
    assert_eq!(rows[2].0, "Innovative wireless earbuds".to_string());
    assert_eq!(rows[3].0, "Fast charging power bank".to_string());
    assert_eq!(rows[4].0, "Bluetooth-enabled speaker".to_string());

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM search_idx.search('description:"bluetooth speaker"~1', stable_sort => true, limit_rows => 5);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "Bluetooth-enabled speaker");

    r#"
    CREATE EXTENSION vector;
    ALTER TABLE mock_items ADD COLUMN embedding vector(3);
    "#
    .execute(&mut conn);
    r#"
    UPDATE mock_items m
    SET embedding = ('[' ||
        ((m.id + 1) % 10 + 1)::integer || ',' ||
        ((m.id + 2) % 10 + 1)::integer || ',' ||
        ((m.id + 3) % 10 + 1)::integer || ']')::vector;
    "#
    .execute(&mut conn);
    let rows: Vec<(String, i32, String, Vector)> = r#"
    SELECT description, rating, category, embedding
    FROM mock_items
    LIMIT 3;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].0, "Ergonomic metal keyboard");
    assert_eq!(rows[1].0, "Plastic Keyboard");
    assert_eq!(rows[2].0, "Sleek running shoes");
    assert_eq!(rows[0].3, Vector::from(vec![3.0, 4.0, 5.0]));
    assert_eq!(rows[1].3, Vector::from(vec![4.0, 5.0, 6.0]));
    assert_eq!(rows[2].3, Vector::from(vec![5.0, 6.0, 7.0]));

    r#"
    CREATE INDEX on mock_items
    USING hnsw (embedding vector_l2_ops);
    "#
    .execute(&mut conn);
    let rows: Vec<(String, String, i32, Vector)> = r#"
    SELECT description, category, rating, embedding
    FROM mock_items
    ORDER BY embedding <-> '[1,2,3]'
    LIMIT 3;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].0, "Artistic ceramic vase");
    assert_eq!(rows[1].0, "Modern wall clock");
    assert_eq!(rows[2].0, "Designer wall paintings");
    assert_eq!(rows[0].3, Vector::from(vec![1.0, 2.0, 3.0]));
    assert_eq!(rows[1].3, Vector::from(vec![1.0, 2.0, 3.0]));
    assert_eq!(rows[2].3, Vector::from(vec![1.0, 2.0, 3.0]));

    let rows: Vec<(i32, f32)> = r#"
    SELECT * FROM search_idx.score_hybrid(
        bm25_query => 'description:keyboard OR category:electronics',
        similarity_query => '''[1,2,3]'' <-> embedding',
        bm25_weight => 0.9,
        similarity_weight => 0.1
    ) LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows[0].0, 2); // For integer comparison, regular assert_eq! is fine
    assert_eq!(rows[1].0, 1);
    assert_eq!(rows[2].0, 29);
    assert_eq!(rows[3].0, 39);
    assert_eq!(rows[4].0, 9);
    assert_relative_eq!(rows[0].1, 0.95714283, epsilon = 1e-6); // Adjust epsilon as needed
    assert_relative_eq!(rows[1].1, 0.8490507, epsilon = 1e-6);
    assert_relative_eq!(rows[2].1, 0.1, epsilon = 1e-6);
    assert_relative_eq!(rows[3].1, 0.1, epsilon = 1e-6);
    assert_relative_eq!(rows[4].1, 0.1, epsilon = 1e-6);

    let rows: Vec<(String, String, Vector, f32)> = r#"
    SELECT m.description, m.category, m.embedding, s.score_hybrid
    FROM mock_items m
    LEFT JOIN (
        SELECT * FROM search_idx.score_hybrid(
            bm25_query => 'description:keyboard OR category:electronics',
            similarity_query => '''[1,2,3]'' <-> embedding',
            bm25_weight => 0.9,
            similarity_weight => 0.1
        )
    ) s
    ON m.id = s.id
    LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].0, "Plastic Keyboard");
    assert_eq!(rows[1].0, "Ergonomic metal keyboard");
    assert_eq!(rows[2].0, "Designer wall paintings");
    assert_eq!(rows[3].0, "Handcrafted wooden frame");
    assert_eq!(rows[4].0, "Modern wall clock");
    assert_eq!(rows[0].2, Vector::from(vec![4.0, 5.0, 6.0]));
    assert_eq!(rows[1].2, Vector::from(vec![3.0, 4.0, 5.0]));
    assert_eq!(rows[2].2, Vector::from(vec![1.0, 2.0, 3.0]));
    assert_eq!(rows[3].2, Vector::from(vec![1.0, 2.0, 3.0]));
    assert_eq!(rows[4].2, Vector::from(vec![1.0, 2.0, 3.0]));
    assert_relative_eq!(rows[0].3, 0.95714283, epsilon = 1e-6);
    assert_relative_eq!(rows[1].3, 0.8490507, epsilon = 1e-6);
    assert_relative_eq!(rows[2].3, 0.1, epsilon = 1e-6);
    assert_relative_eq!(rows[3].3, 0.1, epsilon = 1e-6);
    assert_relative_eq!(rows[4].3, 0.1, epsilon = 1e-6);
}

/// The quickstart guide in our docs shows the `ngram_idx` happening between creating a basic index
/// and doing a similarity search.  As we only allow one index on a table at a time, we need to test
/// this bit separately
#[rstest]
fn quickstart_ngram_idx(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    )
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
            index_name => 'ngrams_idx',
            schema_name => 'public',
            table_name => 'mock_items',
            key_field => 'id',
            text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('ngram', min_gram => 4, max_gram => 4, prefix_only => false)) || paradedb.field('category')
    );
    "#.execute(&mut conn);
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM ngrams_idx.search('description:blue', stable_sort => true);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "Bluetooth-enabled speaker");

    let rows: Vec<(i32, String, f32)> = r#"
        SELECT * FROM ngrams_idx.snippet(
        'description:blue', 
        highlight_field => 'description'
    )
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, 32);
    assert_eq!(rows[0].1, "<b>Blue</b>tooth-enabled speaker");
    assert_relative_eq!(rows[0].2, 2.9903657, epsilon = 1e-6);

    let rows: Vec<(String, String, f32)> = r#"
    WITH snippet AS (
        SELECT * FROM ngrams_idx.snippet(
        'description:blue', 
        highlight_field => 'description'
        )
    )
    SELECT description, snippet, score_bm25
    FROM snippet
    LEFT JOIN mock_items ON snippet.id = mock_items.id
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "Bluetooth-enabled speaker");
    assert_eq!(rows[0].1, "<b>Blue</b>tooth-enabled speaker");
    assert_relative_eq!(rows[0].2, 2.9903657, epsilon = 1e-6);
}

#[rstest]
fn identical_queries(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );
    CALL paradedb.create_bm25(
            index_name => 'search_idx',
            schema_name => 'public',
            table_name => 'mock_items',
            key_field => 'id',
            text_fields => paradedb.field('description') || paradedb.field('category')
    );
    "#
    .execute(&mut conn);

    let rows1: SimpleProductsTableVec =
        "SELECT * FROM search_idx.search('description:shoes', stable_sort => true)"
            .fetch_collect(&mut conn);
    let rows2: SimpleProductsTableVec = "SELECT * FROM search_idx.search(
            query => paradedb.parse('description:shoes'),
            stable_sort => true
        )"
    .fetch_collect(&mut conn);
    let rows3: SimpleProductsTableVec = r#"
        SELECT * FROM search_idx.search(
	        query => paradedb.term(
	        	field => 'description',
	        	value => 'shoes'
	        ),
	        stable_sort => true
        )"#
    .fetch_collect(&mut conn);

    assert_eq!(rows1.id, rows2.id);
    assert_eq!(rows2.id, rows3.id);
}

#[rstest]
fn score_bm25(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'mock_items'
    );
    CALL paradedb.create_bm25(
        index_name => 'search_idx',
        schema_name => 'public',
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category'),
        numeric_fields => paradedb.field('rating')
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, f32)> = "
    SELECT * FROM search_idx.score_bm25(
        'description:keyboard',
        limit_rows => 10
    )"
    .fetch(&mut conn);

    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    let expected = [2, 1];
    assert_eq!(ids, expected);

    let ranks: Vec<_> = rows.iter().map(|r| r.1).collect();
    let expected = [3.2668595, 2.8213787];
    assert_eq!(ranks, expected);

    let rows: Vec<(i32, String, f32)> = "
    WITH scores AS (
        SELECT * FROM search_idx.score_bm25(
        'description:keyboard',
        limit_rows => 10
        )
    )
    SELECT scores.id, description, score_bm25
    FROM scores
    LEFT JOIN mock_items ON scores.id = mock_items.id;
    "
    .fetch(&mut conn);

    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    let expected = [2, 1];
    assert_eq!(ids, expected);

    let descriptions: Vec<_> = rows.iter().map(|r| r.1.clone()).collect();
    let expected = ["Plastic Keyboard", "Ergonomic metal keyboard"];
    assert_eq!(descriptions, expected);

    let ranks: Vec<_> = rows.iter().map(|r| r.2).collect();
    let expected = [3.2668595, 2.8213787];
    assert_eq!(ranks, expected);
}

#[rstest]
fn snippet(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'mock_items'
    );
    CALL paradedb.create_bm25(
        index_name => 'search_idx',
        schema_name => 'public',
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category'),
        numeric_fields => paradedb.field('rating')
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String, f32)> = "
    SELECT * FROM search_idx.snippet(
        'description:keyboard',
        highlight_field => 'description',
        max_num_chars => 100
    )"
    .fetch(&mut conn);

    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    let expected = [2, 1];
    assert_eq!(ids, expected);

    let snippets: Vec<_> = rows.iter().map(|r| r.1.clone()).collect();
    let expected = ["Plastic <b>Keyboard</b>", "Ergonomic metal <b>keyboard</b>"];
    assert_eq!(snippets, expected);

    let ranks: Vec<_> = rows.iter().map(|r| r.2).collect();
    let expected = [3.2668595, 2.8213787];
    assert_eq!(ranks, expected);

    let rows: Vec<(i32, String, String, f32)> = "
    WITH snippets AS (
        SELECT * FROM search_idx.snippet(
        'description:keyboard',
        highlight_field => 'description'
        )
    )
    SELECT snippets.id, description, snippet, score_bm25
    FROM snippets
    LEFT JOIN mock_items ON snippets.id = mock_items.id;
    "
    .fetch(&mut conn);

    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    let expected = [2, 1];
    assert_eq!(ids, expected);

    let descriptions: Vec<_> = rows.iter().map(|r| r.1.clone()).collect();
    let expected = ["Plastic Keyboard", "Ergonomic metal keyboard"];
    assert_eq!(descriptions, expected);

    let snippets: Vec<_> = rows.iter().map(|r| r.2.clone()).collect();
    let expected = ["Plastic <b>Keyboard</b>", "Ergonomic metal <b>keyboard</b>"];
    assert_eq!(snippets, expected);

    let ranks: Vec<_> = rows.iter().map(|r| r.3).collect();
    let expected = [3.2668595, 2.8213787];
    assert_eq!(ranks, expected);
}

#[rstest]
fn joined_tables(mut conn: PgConnection) {
    r#"
    CREATE TABLE mock_products (
        id SERIAL PRIMARY KEY,
        product_name TEXT
    );

    INSERT INTO mock_products (product_name) 
    VALUES ('Flat Screen TV'), ('MP3 Player');

    CREATE TABLE mock_reviews (
        review_id SERIAL PRIMARY KEY,
        product_id INT REFERENCES mock_products(id),
        review TEXT
    );

    INSERT INTO mock_reviews (product_id, review)
    VALUES (1, 'Amazing resolution'), (2, 'Amazing sound'), (2, 'Would recommend');

    CREATE MATERIALIZED VIEW product_reviews 
    AS SELECT r.review_id, p.product_name, r.review 
    FROM mock_reviews r 
    LEFT JOIN mock_products p ON p.id = r.product_id;

    CALL paradedb.create_bm25(
        index_name => 'product_reviews',
        table_name => 'product_reviews',
        key_field => 'review_id',
        text_fields => paradedb.field('review') || paradedb.field('product_name')
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String, String)> =
        "SELECT * FROM product_reviews.search('review:amazing OR product_name:tv')"
            .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            (1, "Flat Screen TV".into(), "Amazing resolution".into()),
            (2, "MP3 Player".into(), "Amazing sound".into())
        ]
    );

    r#"
    CREATE MATERIALIZED VIEW product_reviews_agg
    AS SELECT p.id, p.product_name, array_agg(r.review) AS reviews
    FROM mock_reviews r
    LEFT JOIN mock_products p
    ON p.id = r.product_id
    GROUP BY p.product_name, p.id;

    CALL paradedb.create_bm25(
        index_name => 'product_reviews_agg',
        table_name => 'product_reviews_agg',
        key_field => 'id',
        text_fields => paradedb.field('reviews') || paradedb.field('product_name')
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String, Vec<String>)> =
        "SELECT * FROM product_reviews_agg.search('reviews:sound')".fetch(&mut conn);
    assert_eq!(
        rows,
        vec![(
            2,
            "MP3 Player".into(),
            vec!["Amazing sound".into(), "Would recommend".into()]
        )]
    );
}

#[rstest]
fn multiple_tokenizers_example(mut conn: PgConnection) {
    // Create the test table
    "CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');"
        .execute(&mut conn);

    // Create the BM25 index with multiple tokenizers
    r#"
    CALL paradedb.create_bm25(
      index_name => 'search_idx',
      table_name => 'mock_items',
      schema_name => 'public',
      key_field => 'id',
      text_fields =>
        paradedb.field('description', tokenizer => paradedb.tokenizer('whitespace')) ||
        paradedb.field('description', alias => 'description_ngram', tokenizer => paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false)) ||
        paradedb.field('description', alias => 'description_stem', tokenizer => paradedb.tokenizer('en_stem'))
    );"#
    .execute(&mut conn);

    // Exact phrase match using default tokenizer
    let rows: Vec<(i32, String)> = r#"
    SELECT id, description 
    FROM search_idx.search('description:"Ergonomic metal keyboard"', stable_sort => true)"#
        .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "Ergonomic metal keyboard");

    // Partial match using ngram tokenizer
    let rows: Vec<(i32, String)> = r#"
    SELECT id, description 
    FROM search_idx.search('description_ngram:key', stable_sort => true)"#
        .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .any(|(_, desc)| desc == "Ergonomic metal keyboard"));
    assert!(rows.iter().any(|(_, desc)| desc == "Plastic Keyboard"));

    // Stemmed search using en_stem tokenizer
    let rows: Vec<(i32, String)> = r#"
    SELECT id, description 
    FROM search_idx.search('description_stem:running', stable_sort => true)"#
        .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert!(rows.iter().any(|(_, desc)| desc == "Sleek running shoes"));

    // Combining different tokenizers in a single query
    let rows: Vec<(i32, String)> = r#"
    SELECT id, description 
    FROM search_idx.search('description_ngram:cam AND description_stem:digitally', stable_sort => true)"#
        .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert!(rows
        .iter()
        .any(|(_, desc)| desc == "Compact digital camera"));

    //  Using default tokenizer for exact match and stem for related concepts
    let rows: Vec<(i32, String)> = r#"
    SELECT id, description 
    FROM search_idx.search('description:"Soft cotton" OR description_stem:shirts', stable_sort => true)"#
        .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert!(rows.iter().any(|(_, desc)| desc == "Soft cotton shirt"));
}
