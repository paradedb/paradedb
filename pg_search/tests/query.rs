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

use core::panic;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn boolean_tree(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.boolean(
            should => ARRAY[
                paradedb.parse('description:shoes'),
                paradedb.phrase_prefix(field => 'description', phrases => ARRAY['book']),
                paradedb.term(field => 'description', value => 'speaker'),
			    paradedb.fuzzy_term(field => 'description', value => 'wolo', transposition_cost_one => false, distance => 1, prefix => true)
            ]
        ),
        lenient_parsing => false,
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![32, 5, 3, 4, 7, 34, 37, 10, 33, 39, 41]);
}

#[rstest]
fn fuzzy_term(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.fuzzy_term(field => 'category', value => 'elector', prefix => true),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 12, 22, 32], "wrong results");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.term(field => 'category', value => 'electornics'),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert!(columns.is_empty(), "without fuzzy field should be empty");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.fuzzy_term(
            field => 'description',
            value => 'keybaord',
            transposition_cost_one => false,
            distance => 1,
            prefix => true
        ),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert!(
        columns.is_empty(),
        "transposition_cost_one false should be empty"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.fuzzy_term(
            field => 'description',
            value => 'keybaord',
            transposition_cost_one => true,
            distance => 1,
            prefix => true
        ),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(
        columns.id,
        vec![1, 2],
        "incorrect transposition_cost_one true"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.fuzzy_term(
            field => 'description',
            value => 'keybaord',
            prefix => true
        ),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2], "incorrect defaults");
}

#[rstest]
fn single_queries(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // All
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.all(),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Boost
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.boost(query => paradedb.all(), boost => 1.5),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // ConstScore
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.const_score(query => paradedb.all(), score => 3.9),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // DisjunctionMax
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.disjunction_max(disjuncts => ARRAY[paradedb.parse('description:shoes')]),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Empty
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.empty(),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 0);

    // FuzzyTerm
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
		paradedb.fuzzy_term(field => 'description', value => 'wolo', transposition_cost_one => false, distance => 1, prefix => true),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 4);

    // Parse
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.parse('description:teddy'),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // PhrasePrefix
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.phrase_prefix(field => 'description', phrases => ARRAY['har']),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Phrase with invalid term list
    match r#"
       SELECT * FROM bm25_search.search(
        query => paradedb.phrase(field => 'description', phrases => ARRAY['robot']),
        stable_sort => true
    )"#
    .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("required to have strictly more than one term")),
        _ => panic!("phrase prefix query should require multiple terms"),
    }

    // Phrase
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.phrase(
            field => 'description',
            phrases => ARRAY['robot', 'building', 'kit']
        ),
        stable_sort => true

    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Range
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.range(field => 'last_updated_date', range => '[2023-05-01,2023-05-03]'::daterange),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 7);

    // Regex
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.regex(
            field => 'description',
            pattern => '(hardcover|plush|leather|running|wireless)'
        ),
        stable_sort => true

    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);

    // Term
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.term(field => 'description', value => 'shoes'),
        stable_sort => true

    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Term with no field (should search all columns)
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.term(value => 'shoes'),
        stable_sort => true

    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // TermSet with invalid term list
    match r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.term_set(
            terms => ARRAY[
                paradedb.regex(field => 'description', pattern => '.+')
            ]
        ),
        stable_sort => true
    )"#
    .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("only term queries can be passed to term_set")),
        _ => panic!("term set query should only accept terms"),
    }

    // TermSet
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.term_set(
            terms => ARRAY[
                paradedb.term(field => 'description', value => 'shoes'),
                paradedb.term(field => 'description', value => 'novel')
            ]
        ),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);
}

#[rstest]
fn exists_query(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Simple exists query
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.exists('rating')
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Non fast field should fail
    match r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.exists('description')
    )"#
    .execute_result(&mut conn)
    {
        Err(err) => assert!(err.to_string().contains("not a fast field")),
        _ => panic!("exists() over non-fast field should fail"),
    }

    // Exists with boolean query
    "INSERT INTO paradedb.bm25_search (id, description, rating) VALUES (42, 'shoes', NULL)"
        .execute(&mut conn);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.boolean(
            must => ARRAY[
                paradedb.exists('rating'),
                paradedb.parse('description:shoes')
            ]
        )
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);
}

#[rstest]
fn more_like_this_raw(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES 
        ('apple'),
        ('banana'), 
        ('cherry'), 
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    match r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(),
        stable_sort => true
    );
    "#
    .fetch_result::<()>(&mut conn)
    {
        Err(err) => {
            assert_eq!(err
            .to_string()
            , "error returned from database: more_like_this must be called with either with_document_id or with_document_fields")
        }
        _ => panic!("with_document_id or with_document_fields validation failed"),
    }

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_id => 2
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_empty(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES 
        ('apple'),
        ('banana'), 
        ('cherry'), 
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    match r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(),
        stable_sort => true
    );
    "#
    .fetch_result::<()>(&mut conn)
    {
        Err(err) => {
            assert_eq!(err
            .to_string()
            , "error returned from database: more_like_this must be called with either with_document_id or with_document_fields")
        }
        _ => panic!("with_document_id or with_document_fields validation failed"),
    }
}

#[rstest]
fn more_like_this_text(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES 
        ('apple'),
        ('banana'), 
        ('cherry'), 
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_boolean_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id BOOLEAN PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (true, 'apple'),
        (false, 'banana')
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(bool, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn more_like_this_uuid_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id UUID PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'apple'),
        ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'banana'), 
        ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'cherry'), 
        ('eb833eb6-c598-4042-b84a-0045828fceea', 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(uuid::Uuid, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id BIGINT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1, 'apple'),
        (2, 'banana'), 
        (3, 'cherry'), 
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i64, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id INT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1, 'apple'),
        (2, 'banana'), 
        (3, 'cherry'), 
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i16_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SMALLINT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1, 'apple'),
        (2, 'banana'), 
        (3, 'cherry'), 
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i16, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_f32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id FLOAT4 PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1.1, 'apple'),
        (2.2, 'banana'),
        (3.3, 'cherry'),
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(f32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_f64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id FLOAT8 PRIMARY KEY,
    flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1.1, 'apple'),
        (2.2, 'banana'), 
        (3.3, 'cherry'), 
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(f64, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_numeric_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id NUMERIC PRIMARY KEY,
    flavour TEXT
    );
    
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1.1, 'apple'),
        (2.2, 'banana'), 
        (3.3, 'cherry'), 
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(f64, String)> = r#"
    SELECT CAST(id AS FLOAT8), flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_date_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id DATE PRIMARY KEY,
    flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('2023-05-03', 'apple'),
        ('2023-05-04', 'banana'), 
        ('2023-05-05', 'cherry'), 
        ('2023-05-06', 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_time_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id TIME PRIMARY KEY,
    flavour TEXT
    );
    
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('08:09:10', 'apple'),
        ('09:10:11', 'banana'), 
        ('10:11:12', 'cherry'), 
        ('11:12:13', 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timestamp_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id TIMESTAMP PRIMARY KEY,
    flavour TEXT
    );
    
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('2023-05-03 08:09:10', 'apple'),
        ('2023-05-04 09:10:11', 'banana'), 
        ('2023-05-05 10:11:12', 'cherry'), 
        ('2023-05-06 11:12:13', 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timestamptz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id TIMESTAMP WITH TIME ZONE PRIMARY KEY,
    flavour TEXT
    );
    
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('2023-05-03 08:09:10 EST', 'apple'),
        ('2023-05-04 09:10:11 PST', 'banana'), 
        ('2023-05-05 10:11:12 MST', 'cherry'), 
        ('2023-05-06 11:12:13 CST', 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timetz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id TIME WITH TIME ZONE PRIMARY KEY,
        flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('08:09:10 EST',
        'apple'),
        ('09:10:11 PST', 'banana'),
        ('10:11:12 MST', 'cherry'),
        ('11:12:13 CST', 'banana split');
    "#
    .execute(&mut conn);
    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_more_like_this_table',
        index_name => 'test_more_like_this_index',
        key_field => 'id',
        text_fields => '{"flavour": {}}'    
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_index.search(
        query => paradedb.more_like_this(
            with_min_doc_frequency => 0,
            with_min_term_frequency => 0,
            with_document_fields => '{"flavour": "banana"}'
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn fuzzy_phrase(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.fuzzy_phrase(field => 'description', value => 'ruling shoeez'),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3, 4, 5]);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.fuzzy_phrase(field => 'description', value => 'ruling shoeez', match_all_terms => true),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.fuzzy_phrase(field => 'description', value => 'ruling shoeez', distance => 1),
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id.len(), 0);
}

fn lenient_config_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Test lenient configuration: lenient flag enabled, should allow for minor errors like typos
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.fuzzy_term(
            field => 'description',
            value => 'wolo',
            transposition_cost_one => false,
            distance => 1
        ),
        lenient_parsing => true,
        stable_sort => true
    )"#
    .fetch_collect(&mut conn);

    assert_eq!(columns.len(), 4);
}
