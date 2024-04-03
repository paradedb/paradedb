mod fixtures;

use approx::assert_relative_eq;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn bm25_search(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );
    CREATE TABLE mock_items_parquet USING parquet 
    AS SELECT id, description, rating, category FROM mock_items;
    "#
    .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items_parquet
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
            table_name => 'mock_items_parquet',
            key_field => 'id',
            text_fields => '{description: {tokenizer: {type: "en_stem"}}, category: {}}'
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM search_idx.search('description:keyboard OR category:electronics', stable_sort => true)
    LIMIT 5;
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
    FROM search_idx.search('description:"bluetooth speaker"~1', stable_sort => true)
    LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "Bluetooth-enabled speaker");

    r#"
    CALL paradedb.create_bm25(
            index_name => 'ngrams_idx',
            schema_name => 'public',
            table_name => 'mock_items_parquet',
            key_field => 'id',
            text_fields => '{description: {tokenizer: {type: "ngram", min_gram: 4, max_gram: 4, prefix_only: false}}, category: {}}'
    );
    "#.execute(&mut conn);
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM ngrams_idx.search('description:blue', stable_sort => true);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "Bluetooth-enabled speaker");

    let rows: Vec<(String, String, f32)> = r#"
    SELECT description, paradedb.highlight(id, field => 'description'), paradedb.rank_bm25(id)
    FROM ngrams_idx.search('description:blue', stable_sort => true)
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "Bluetooth-enabled speaker");
    assert_eq!(rows[0].1, "<b>Blue</b>tooth-enabled speaker");
    assert_relative_eq!(rows[0].2, 2.9903657, epsilon = 1e-6);
}
