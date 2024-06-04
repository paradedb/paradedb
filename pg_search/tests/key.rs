mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn boolean_key(mut conn: PgConnection) {
    // Boolean keys are pretty useless, but they're supported!

    r#"
    CREATE TABLE test_table (
        id BOOLEAN,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (true, 'bluetooth'), (false, 'blue');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_table',
        index_name => 'test_index',
        key_field => 'id',
        text_fields => '{value: {tokenizer: {type: "ngram", min_gram: 4, max_gram: 4, prefix_only: false}}}'
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(bool, f32)> = r#"
    SELECT id, paradedb.rank_bm25(id) FROM test_index.search(
        query => paradedb.term(field => 'value', value => 'blue'),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(false, 0.25759196), (true, 0.14109309)]);
}

#[rstest]
fn uuid_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id UUID,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('eb833eb6-c598-4042-b84a-0045828fceea', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('28b6374a-67d3-41c8-93af-490712f9923e', 'redness');
    INSERT INTO test_table (id, value) VALUES ('f6e85626-298e-4112-9abb-3856f8aa046a', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('88345d21-7b89-4fd6-87e4-83a4f68dbc3c', 'great white');
    INSERT INTO test_table (id, value) VALUES ('40bc9216-66d0-4ae8-87ee-ddb02e3e1b33', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('02f9789d-4963-47d5-a189-d9c114f5cba4', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_table',
        index_name => 'test_index',
        key_field => 'id',
        text_fields => '{value: {tokenizer: {type: "ngram", min_gram: 4, max_gram: 4, prefix_only: false}}}'
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), paradedb.rank_bm25(id) FROM test_index.search(
        query => paradedb.term(field => 'value', value => 'blue'),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (
                "b5faacc0-9eba-441a-81f8-820b46a3b57e".to_string(),
                0.61846066
            ),
            (
                "38bf27a0-1aa8-42cd-9cb0-993025e0b8d0".to_string(),
                0.57459813
            ),
            (
                "f159c89e-2162-48cd-85e3-e42b71d2ecd0".to_string(),
                0.53654534
            ),
            (
                "40bc9216-66d0-4ae8-87ee-ddb02e3e1b33".to_string(),
                0.50321954
            ),
            (
                "ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad".to_string(),
                0.47379148
            ),
            (
                "eb833eb6-c598-4042-b84a-0045828fceea".to_string(),
                0.44761515
            ),
        ]
    );
}

#[rstest]
fn i64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id BIGINT,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1, 'text 1'), (2, 'text 2'), (3, 'text 3'), (3, 'text 4');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_table',
        index_name => 'test_index',
        key_field => 'id',
        text_fields => '{"value": {}}'
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i64, String)> = r#"
    SELECT * FROM test_index.search(
        query => paradedb.term(field => 'value', value => 'text'),
        stable_sort => true);
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (1, "text 1".to_string()),
            (2, "text 2".to_string()),
            (3, "text 4".to_string()),
            (3, "text 3".to_string())
        ]
    );
}
