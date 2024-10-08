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
use pgvector::Vector;
use rstest::*;
use sqlx::types::BigDecimal;
use sqlx::PgConnection;
use std::str::FromStr;

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
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field('description') || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        boolean_fields => paradedb.field('in_stock'),
        datetime_fields => paradedb.field('created_at'),
        json_fields => paradedb.field('metadata')
    )"#
    .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' OR category @@@ 'footwear' AND rating @@@ '>2'
    ORDER BY description
    LIMIT 5"#
        .fetch(&mut conn);
    assert_eq!(rows[0].0, "Comfortable slippers".to_string());
    assert_eq!(rows[1].0, "Generic shoes".to_string());
    assert_eq!(rows[2].0, "Sleek running shoes".to_string());
    assert_eq!(rows[3].0, "Sturdy hiking boots".to_string());
    assert_eq!(rows[4].0, "White jogging shoes".to_string());

    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, paradedb.score(id)
    FROM mock_items
    WHERE description @@@ 'shoes' OR category @@@ 'footwear' AND rating @@@ '>2'
    ORDER BY score DESC
    LIMIT 5"#
        .fetch(&mut conn);
    assert_eq!(rows[0].0, "Generic shoes".to_string());
    assert_eq!(rows[1].0, "Sleek running shoes".to_string());
    assert_eq!(rows[2].0, "White jogging shoes".to_string());
    assert_eq!(rows[3].0, "Sturdy hiking boots".to_string());
    assert_eq!(rows[4].0, "Comfortable slippers".to_string());
    assert_eq!(rows[0].3, 5.8135376);
    assert_eq!(rows[1].3, 5.4211845);
    assert_eq!(rows[2].3, 5.4211845);
    assert_eq!(rows[3].3, 2.9362776);
    assert_eq!(rows[4].3, 2.9362776);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '"white shoes"~1'
    LIMIT 5"#
        .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "White jogging shoes");

    r#"
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'orders',
        table_type => 'Orders'
    );

    ALTER TABLE orders
    ADD CONSTRAINT foreign_key_product_id
    FOREIGN KEY (product_id)
    REFERENCES mock_items(id);

    CALL paradedb.create_bm25(
        index_name => 'orders_idx',
        table_name => 'orders',
        key_field => 'order_id',
        text_fields => paradedb.field('customer_name')
    );"#
    .execute(&mut conn);

    let rows: Vec<(i32, i32, i32, BigDecimal, String)> = r#"
    SELECT * FROM orders LIMIT 3"#
        .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            (
                1,
                1,
                3,
                BigDecimal::from_str("99.99").unwrap(),
                "John Doe".into()
            ),
            (
                2,
                2,
                1,
                BigDecimal::from_str("49.99").unwrap(),
                "Jane Smith".into()
            ),
            (
                3,
                3,
                5,
                BigDecimal::from_str("249.95").unwrap(),
                "Alice Johnson".into()
            ),
        ]
    );

    let rows: Vec<(i32, String, String)> = r#"
    SELECT o.order_id, o.customer_name, m.description
    FROM orders o
    JOIN mock_items m ON o.product_id = m.id
    WHERE o.customer_name @@@ 'Johnson' AND m.description @@@ 'shoes'
    ORDER BY order_id
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3, "Alice Johnson".into(), "Sleek running shoes".into()),
            (6, "Alice Johnson".into(), "White jogging shoes".into()),
            (36, "Alice Johnson".into(), "White jogging shoes".into()),
        ]
    );

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
    USING hnsw (embedding vector_cosine_ops);
    "#
    .execute(&mut conn);
    let rows: Vec<(String, String, i32, Vector)> = r#"
    SELECT description, category, rating, embedding
    FROM mock_items
    ORDER BY embedding <=> '[1,2,3]', description
    LIMIT 3;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].0, "Artistic ceramic vase");
    assert_eq!(rows[1].0, "Designer wall paintings");
    assert_eq!(rows[2].0, "Handcrafted wooden frame");
    assert_eq!(rows[0].3, Vector::from(vec![1.0, 2.0, 3.0]));
    assert_eq!(rows[1].3, Vector::from(vec![1.0, 2.0, 3.0]));
    assert_eq!(rows[2].3, Vector::from(vec![1.0, 2.0, 3.0]));
}

#[rstest]
fn full_text_search(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CALL paradedb.create_bm25(
        index_name => 'search_idx',
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field('description') || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        boolean_fields => paradedb.field('in_stock'),
        datetime_fields => paradedb.field('created_at'),
        json_fields => paradedb.field('metadata')
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String, i32)> = r#"
    SELECT id, description, rating
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ '>3'
    ORDER BY id
    "#
    .fetch(&mut conn);
    assert_eq!(rows[0], (3, "Sleek running shoes".into(), 5));
    assert_eq!(rows[1], (5, "Generic shoes".into(), 4));

    // Basic term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Multiple terms
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'keyboard' OR category @@@ 'toy'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Not term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '(shoes running -white)'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Basic phrase
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '"plastic keyboard"'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Slop operator
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '"ergonomic keyboard"~1'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Phrase prefix
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '"plastic keyb"*'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Basic filtering
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating > 2
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ '>2'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Numeric filter
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ '4'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ '>=4'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Datetime filter
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND created_at @@@ '"2023-04-20T16:38:02Z"'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Boolean filter
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND in_stock @@@ 'true'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Range filter
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ '[1 TO 4]'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND created_at @@@ '[2020-01-31T00:00:00Z TO 2024-01-31T00:00:00Z]'
    "#.fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '[book TO camera]'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 8);

    // Set filter
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ 'IN [2 3 4]'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Pagination
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes'
    LIMIT 1 OFFSET 2
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // BM25 scoring
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, paradedb.score(id)
    FROM mock_items
    WHERE description @@@ 'shoes'
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Highlighting
    let rows: Vec<(i32, String)> = r#"
    SELECT id, paradedb.snippet(description)
    FROM mock_items
    WHERE description @@@ 'shoes'
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, paradedb.snippet(description, start_tag => '<i>', end_tag => '</i>')
    FROM mock_items
    WHERE description @@@ 'shoes'
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert!(rows[0].1.contains("<i>"));
    assert!(rows[0].1.contains("</i>"));

    // Order by score
    let rows: Vec<(String, i32, String, f32)> = r#"
        SELECT description, rating, category, paradedb.score(id)
        FROM mock_items
        WHERE description @@@ 'shoes'
        ORDER BY score DESC
        LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].3, 2.8772602);
    assert_eq!(rows[1].3, 2.4849067);
    assert_eq!(rows[2].3, 2.4849067);

    // Order by field
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes'
    ORDER BY rating DESC
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("Sleek running shoes".into(), 5, "Footwear".into()),
            ("Generic shoes".into(), 4, "Footwear".into()),
            ("White jogging shoes".into(), 3, "Footwear".into()),
        ]
    );

    // Tiebreaking
    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, paradedb.score(id)
    FROM mock_items
    WHERE category @@@ 'electronics'
    ORDER BY score DESC, rating DESC
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].3, 2.1096356);
    assert_eq!(rows[1].3, 2.1096356);
    assert_eq!(rows[2].3, 2.1096356);
    assert_eq!(rows[3].3, 2.1096356);
    assert_eq!(rows[4].3, 2.1096356);

    // Constant boosting
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, paradedb.score(id)
    FROM mock_items
    WHERE description @@@ 'shoes^2' OR category @@@ 'footwear'
    ORDER BY score DESC
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].1, 7.690798);
    assert_eq!(rows[1].1, 6.9060907);
    assert_eq!(rows[2].1, 6.9060907);
    assert_eq!(rows[3].1, 1.9362776);
    assert_eq!(rows[4].1, 1.9362776);

    // Boost by field
    let rows: Vec<(i32, f64)> = r#"
    SELECT id, paradedb.score(mock_items) * COALESCE(rating, 1) as score
    FROM mock_items
    WHERE description @@@ 'shoes'
    ORDER BY score DESC 
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3, 12.424533367156982),
            (5, 11.509040832519531),
            (4, 7.4547200202941895),
        ]
    );
}

#[rstest]
fn term_level_queries(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CALL paradedb.create_bm25(
        index_name => 'search_idx',
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field('description') || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        boolean_fields => paradedb.field('in_stock'),
        datetime_fields => paradedb.field('created_at'),
        json_fields => paradedb.field('metadata')
    );
    "#
    .execute(&mut conn);

    // Exists
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.exists('rating')
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
    must => ARRAY[
        paradedb.term('description', 'shoes'),
        paradedb.exists('rating')
    ])
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Fuzzy term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.fuzzy_term('description', 'shoez')
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Range
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.range(
        field => 'rating',
        range => int4range(1, 3, '[)')
    )"#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 4);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.range(
        field => 'rating',
        range => int4range(1, 3, '[]')
    )"#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 13);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.range(
        field => 'rating',
        range => int4range(1, NULL, '[)')
    )"#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 41);

    // Regex
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.regex('description', '(plush|leather)')
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.term('description', 'shoes')
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.term('rating', 4)
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 16);

    // Term set
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.term_set(
        terms => ARRAY[
            paradedb.term('description', 'shoes'),
            paradedb.term('description', 'novel')
        ]
    )
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);
}

#[rstest]
fn phrase_level_queries(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CALL paradedb.create_bm25(
        index_name => 'search_idx',
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field('description') || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        boolean_fields => paradedb.field('in_stock'),
        datetime_fields => paradedb.field('created_at'),
        json_fields => paradedb.field('metadata')
    );
    "#
    .execute(&mut conn);

    // Fuzzy phrase
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.fuzzy_phrase('description', 'ruining shoez')
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Phrase
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.phrase('description', ARRAY['running', 'shoes'])
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.phrase_prefix('description', ARRAY['running', 'sh'])
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
#[ignore]
fn json_queries(mut conn: PgConnection) {
    // TODO: Unignore when JSON query builder support is merged in
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CALL paradedb.create_bm25(
        index_name => 'search_idx',
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field('description') || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        boolean_fields => paradedb.field('in_stock'),
        datetime_fields => paradedb.field('created_at'),
        json_fields => paradedb.field('metadata')
    );
    "#
    .execute(&mut conn);

    // Term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.term('metadata.color', 'white')
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
}

#[rstest]
fn compound_queries(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CALL paradedb.create_bm25(
        index_name => 'search_idx',
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field('description') || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        boolean_fields => paradedb.field('in_stock'),
        datetime_fields => paradedb.field('created_at'),
        json_fields => paradedb.field('metadata')
    );
    "#
    .execute(&mut conn);

    // All
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
        should => ARRAY[paradedb.all()],
        must_not => ARRAY[paradedb.term('description', 'shoes')]
    )
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 38);

    // Boolean
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
        should => ARRAY[
        paradedb.term('description', 'headphones')
        ],
        must => ARRAY[
        paradedb.term('category', 'electronics'),
        paradedb.fuzzy_term('description', 'bluetooht')
        ],
        must_not => ARRAY[
        paradedb.range('rating', int4range(NULL, 2, '()'))
        ]
    )
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Boost
    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, paradedb.score(id)
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
    should => ARRAY[
        paradedb.term('description', 'shoes'),
        paradedb.boost(2.0, paradedb.term('description', 'running'))
    ])
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Const score
    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, paradedb.score(id)
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
    should => ARRAY[
        paradedb.const_score(1.0, paradedb.term('description', 'shoes')),
        paradedb.term('description', 'running')
    ])
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Disjunction max
    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, paradedb.score(id)
    FROM mock_items
    WHERE id @@@ paradedb.disjunction_max(ARRAY[
        paradedb.term('description', 'shoes'),
        paradedb.term('description', 'running')
    ])
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Empty
    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.empty()
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 0);

    // Parse
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.parse('description:"running shoes" OR category:footwear')
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.boolean(should => ARRAY[
        paradedb.phrase('description', ARRAY['running', 'shoes']),
        paradedb.term('category', 'footwear')
    ])"#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
#[ignore]
fn specialized_queries(mut conn: PgConnection) {
    // TODO: Unignore once scoring is enabled for more_like_this
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CALL paradedb.create_bm25(
        index_name => 'search_idx',
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field('description') || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        boolean_fields => paradedb.field('in_stock'),
        datetime_fields => paradedb.field('created_at'),
        json_fields => paradedb.field('metadata')
    );
    "#
    .execute(&mut conn);

    // More like this
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@  paradedb.more_like_this(
        with_document_id => 2,
        with_min_term_frequency => 1,
        with_min_word_length => 2,
        with_max_word_length => 5,
        with_boost_factor => 1.0,
        with_stop_words => ARRAY['and', 'the', 'for']
    )
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 0);
}

#[rstest]
fn autocomplete(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CALL paradedb.create_bm25(
        index_name => 'search_idx',
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field('description') || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        boolean_fields => paradedb.field('in_stock'),
        datetime_fields => paradedb.field('created_at'),
        json_fields => paradedb.field('metadata')
    );
    "#
    .execute(&mut conn);

    let expected = vec![
        ("Sleek running shoes".into(), 5, "Footwear".into()),
        ("Generic shoes".into(), 4, "Footwear".into()),
        ("White jogging shoes".into(), 3, "Footwear".into()),
    ];

    // Fuzzy term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE id @@@ paradedb.fuzzy_term(
        field => 'description',
        value => 'shoez'
    ) ORDER BY rating DESC
    "#
    .fetch(&mut conn);
    assert_eq!(rows, expected);

    // Fuzzy phrase
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE id @@@ paradedb.fuzzy_phrase(
        field => 'description',
        value => 'ruining shoez'
    ) ORDER BY rating DESC
    "#
    .fetch(&mut conn);
    assert_eq!(rows, expected);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE id @@@ paradedb.fuzzy_phrase(
        field => 'description',
        value => 'ruining shoez',
        match_all_terms => true
    )
    "#
    .fetch(&mut conn);
    assert_eq!(rows, vec![expected[0].clone()]);

    // Multiple fuzzy fields
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE id @@@ paradedb.boolean(
        should => ARRAY[
            paradedb.fuzzy_phrase(field => 'description', value => 'ruining shoez'),
            paradedb.fuzzy_phrase(field => 'category', value => 'ruining shoez')
        ]
    ) ORDER BY rating DESC
    "#
    .fetch(&mut conn);
    assert_eq!(rows, expected);

    r#"
    CALL paradedb.drop_bm25('search_idx');
    CALL paradedb.create_bm25(
        index_name => 'ngrams_idx',
        schema_name => 'public',
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field(
            'description',
            tokenizer => paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false)
        )
    );
    "#
    .execute(&mut conn);

    // Ngram term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE description @@@ 'sho'
    ORDER BY rating DESC
    "#
    .fetch(&mut conn);
    assert_eq!(rows, expected);

    // Ngram term set
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE id @@@ paradedb.fuzzy_phrase(
        field => 'description',
        value => 'hsoes',
        distance => 0
    ) ORDER BY rating DESC
    "#
    .fetch(&mut conn);
    assert_eq!(rows, expected);
}

#[rstest]
fn hybrid_search(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CALL paradedb.create_bm25(
        index_name => 'search_idx',
        table_name => 'mock_items',
        key_field => 'id',
        text_fields => paradedb.field('description') || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        boolean_fields => paradedb.field('in_stock'),
        datetime_fields => paradedb.field('created_at'),
        json_fields => paradedb.field('metadata')
    );

    CREATE EXTENSION vector;
    ALTER TABLE mock_items ADD COLUMN embedding vector(3);

    UPDATE mock_items m
    SET embedding = ('[' ||
        ((m.id + 1) % 10 + 1)::integer || ',' ||
        ((m.id + 2) % 10 + 1)::integer || ',' ||
        ((m.id + 3) % 10 + 1)::integer || ']')::vector;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, BigDecimal, String, Vector)> = r#"
    WITH semantic_search AS (
        SELECT id, RANK () OVER (ORDER BY embedding <=> '[1,2,3]') AS rank
        FROM mock_items ORDER BY embedding <=> '[1,2,3]' LIMIT 20
    ),
    bm25_search AS (
        SELECT id, RANK () OVER (ORDER BY paradedb.score(id) DESC) as rank
        FROM mock_items WHERE description @@@ 'keyboard' LIMIT 20
    )
    SELECT
        COALESCE(semantic_search.id, bm25_search.id) AS id,
        COALESCE(1.0 / (60 + semantic_search.rank), 0.0) +
        COALESCE(1.0 / (60 + bm25_search.rank), 0.0) AS score,
        mock_items.description,
        mock_items.embedding
    FROM semantic_search
    FULL OUTER JOIN bm25_search ON semantic_search.id = bm25_search.id
    JOIN mock_items ON mock_items.id = COALESCE(semantic_search.id, bm25_search.id)
    ORDER BY score DESC, description
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            (
                1,
                BigDecimal::from_str("0.03062178588125292193").unwrap(),
                String::from("Ergonomic metal keyboard"),
                Vector::from(vec![3.0, 4.0, 5.0])
            ),
            (
                2,
                BigDecimal::from_str("0.02990695613646433318").unwrap(),
                String::from("Plastic Keyboard"),
                Vector::from(vec![4.0, 5.0, 6.0])
            ),
            (
                19,
                BigDecimal::from_str("0.01639344262295081967").unwrap(),
                String::from("Artistic ceramic vase"),
                Vector::from(vec![1.0, 2.0, 3.0])
            ),
            (
                29,
                BigDecimal::from_str("0.01639344262295081967").unwrap(),
                String::from("Designer wall paintings"),
                Vector::from(vec![1.0, 2.0, 3.0])
            ),
            (
                39,
                BigDecimal::from_str("0.01639344262295081967").unwrap(),
                String::from("Handcrafted wooden frame"),
                Vector::from(vec![1.0, 2.0, 3.0])
            ),
        ]
    );
}

#[rstest]
fn create_bm25_test_tables(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'orders',
        table_type => 'Orders'
    );

    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'parts',
        table_type => 'Parts'
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, i32, i32, f32, String)> = r#"
        SELECT order_id, product_id, order_quantity, order_total::REAL, customer_name FROM orders
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 64);
    assert_eq!(rows[0], (1, 1, 3, 99.99, "John Doe".into()));

    let rows: Vec<(i32, i32, String)> = r#"
        SELECT part_id, parent_part_id, description FROM parts
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 36);
    assert_eq!(rows[0], (1, 0, "Chassis Assembly".into()));
}
