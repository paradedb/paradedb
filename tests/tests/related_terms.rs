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

fn setup_related_terms_table(conn: &mut PgConnection) {
    r#"
    DROP TABLE IF EXISTS related_terms_test CASCADE;
    CREATE TABLE related_terms_test (
        id SERIAL PRIMARY KEY,
        description TEXT,
        category TEXT,
        price INTEGER
    );

    INSERT INTO related_terms_test (description, category, price) VALUES
        ('running shoes for athletes', 'footwear', 100),
        ('comfortable running sneakers', 'footwear', 120),
        ('hiking boots for outdoor adventures', 'footwear', 150),
        ('casual shoes for everyday wear', 'footwear', 80),
        ('athletic running gear', 'apparel', 50),
        ('running shorts and tops', 'apparel', 40);

    CREATE INDEX ON related_terms_test USING bm25 (id, description, category, price) WITH (key_field = 'id');
    "#
    .execute(conn);
}

fn teardown_related_terms_table(conn: &mut PgConnection) {
    "DROP TABLE IF EXISTS related_terms_test CASCADE;".execute(conn);
}

#[rstest]
fn basic_related_terms(mut conn: PgConnection) {
    setup_related_terms_table(&mut conn);

    let results: Vec<(String, String, f32)> = r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'shoes',
        relation => 'related_terms_test'::regclass
    ) ORDER BY weight DESC
    "#
    .fetch_collect(&mut conn);

    assert!(!results.is_empty(), "Should return related terms");

    let terms: Vec<&str> = results.iter().map(|(_, t, _)| t.as_str()).collect();
    assert!(
        !terms.contains(&"shoes"),
        "Query term 'shoes' should be excluded from results"
    );

    // Verify field names are returned
    let fields: Vec<&str> = results.iter().map(|(f, _, _)| f.as_str()).collect();
    assert!(
        fields
            .iter()
            .all(|f| *f == "description" || *f == "category" || *f == "price"),
        "Field names should be valid indexed fields"
    );

    teardown_related_terms_table(&mut conn);
}

#[rstest]
fn related_terms_with_specific_fields(mut conn: PgConnection) {
    setup_related_terms_table(&mut conn);

    let results: Vec<(String, String, f32)> = r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'shoes',
        relation => 'related_terms_test'::regclass,
        fields => ARRAY['description']
    ) ORDER BY weight DESC
    "#
    .fetch_collect(&mut conn);

    assert!(!results.is_empty(), "Should return related terms");

    // All results should be from the description field
    for (field, _, _) in &results {
        assert_eq!(
            field, "description",
            "All terms should be from 'description' field"
        );
    }

    teardown_related_terms_table(&mut conn);
}

#[rstest]
fn related_terms_max_query_terms(mut conn: PgConnection) {
    setup_related_terms_table(&mut conn);

    let results: Vec<(String, String, f32)> = r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'running',
        relation => 'related_terms_test'::regclass,
        fields => ARRAY['description'],
        max_query_terms => 3
    ) ORDER BY weight DESC
    "#
    .fetch_collect(&mut conn);

    assert!(
        results.len() <= 3,
        "Should return at most 3 terms, got {}",
        results.len()
    );

    teardown_related_terms_table(&mut conn);
}

#[rstest]
fn related_terms_excludes_query_term(mut conn: PgConnection) {
    setup_related_terms_table(&mut conn);

    let results: Vec<(String, String, f32)> = r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'running',
        relation => 'related_terms_test'::regclass,
        fields => ARRAY['description']
    ) WHERE term = 'running'
    "#
    .fetch_collect(&mut conn);

    assert!(
        results.is_empty(),
        "Query term 'running' should be excluded from results"
    );

    teardown_related_terms_table(&mut conn);
}

#[rstest]
fn related_terms_no_matching_documents(mut conn: PgConnection) {
    setup_related_terms_table(&mut conn);

    let results: Vec<(String, String, f32)> = r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'nonexistentterm',
        relation => 'related_terms_test'::regclass
    ) ORDER BY weight DESC
    "#
    .fetch_collect(&mut conn);

    assert!(
        results.is_empty(),
        "Should return empty result for non-matching query term"
    );

    teardown_related_terms_table(&mut conn);
}

#[rstest]
fn related_terms_min_word_length(mut conn: PgConnection) {
    setup_related_terms_table(&mut conn);

    let results: Vec<(String, String, f32)> = r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'shoes',
        relation => 'related_terms_test'::regclass,
        fields => ARRAY['description'],
        min_word_length => 6
    ) ORDER BY weight DESC
    "#
    .fetch_collect(&mut conn);

    for (_, term, _) in &results {
        assert!(
            term.len() >= 6,
            "Term '{}' should be at least 6 characters",
            term
        );
    }

    teardown_related_terms_table(&mut conn);
}

#[rstest]
fn related_terms_max_word_length(mut conn: PgConnection) {
    setup_related_terms_table(&mut conn);

    let results: Vec<(String, String, f32)> = r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'shoes',
        relation => 'related_terms_test'::regclass,
        fields => ARRAY['description'],
        max_word_length => 5
    ) ORDER BY weight DESC
    "#
    .fetch_collect(&mut conn);

    for (_, term, _) in &results {
        assert!(
            term.len() <= 5,
            "Term '{}' should be at most 5 characters",
            term
        );
    }

    teardown_related_terms_table(&mut conn);
}

#[rstest]
fn related_terms_null_fields_uses_all(mut conn: PgConnection) {
    setup_related_terms_table(&mut conn);

    let results: Vec<(String, String, f32)> = r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'footwear',
        relation => 'related_terms_test'::regclass,
        fields => NULL
    ) ORDER BY weight DESC LIMIT 10
    "#
    .fetch_collect(&mut conn);

    assert!(
        !results.is_empty(),
        "Should return related terms when fields is NULL"
    );

    teardown_related_terms_table(&mut conn);
}

#[rstest]
fn related_terms_error_no_index(mut conn: PgConnection) {
    "DROP TABLE IF EXISTS no_index_table CASCADE;".execute(&mut conn);
    "CREATE TABLE no_index_table (id SERIAL PRIMARY KEY, name TEXT);".execute(&mut conn);

    match r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'test',
        relation => 'no_index_table'::regclass
    )
    "#
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail when no BM25 index exists"),
        Err(err) => assert!(
            err.to_string().contains("no BM25 index found"),
            "Expected 'no BM25 index found' error, got: {}",
            err
        ),
    };

    "DROP TABLE no_index_table;".execute(&mut conn);
}

#[rstest]
fn related_terms_weights_are_positive(mut conn: PgConnection) {
    setup_related_terms_table(&mut conn);

    let results: Vec<(String, String, f32)> = r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'running',
        relation => 'related_terms_test'::regclass
    ) ORDER BY weight DESC
    "#
    .fetch_collect(&mut conn);

    for (_, term, weight) in &results {
        assert!(
            *weight > 0.0,
            "Weight for term '{}' should be positive, got {}",
            term,
            weight
        );
    }

    teardown_related_terms_table(&mut conn);
}

#[rstest]
fn related_terms_weights_ordered_descending(mut conn: PgConnection) {
    setup_related_terms_table(&mut conn);

    let results: Vec<(String, String, f32)> = r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'running',
        relation => 'related_terms_test'::regclass
    ) ORDER BY weight DESC
    "#
    .fetch_collect(&mut conn);

    if results.len() > 1 {
        for i in 0..results.len() - 1 {
            assert!(
                results[i].2 >= results[i + 1].2,
                "Weights should be in descending order"
            );
        }
    }

    teardown_related_terms_table(&mut conn);
}

#[rstest]
fn related_terms_per_field_df_calculation(mut conn: PgConnection) {
    // This test verifies that DF is calculated per-field, matching MLT behavior.
    // Terms that appear in multiple fields should have separate entries for each field.
    setup_related_terms_table(&mut conn);

    let results: Vec<(String, String, f32)> = r#"
    SELECT field, term, weight FROM pdb.related_terms(
        query_term => 'footwear',
        relation => 'related_terms_test'::regclass,
        fields => ARRAY['description', 'category']
    ) ORDER BY field, term
    "#
    .fetch_collect(&mut conn);

    // Check that we get per-field results
    // The same term could appear with different weights in different fields
    // because DF is calculated per-field
    let field_term_pairs: std::collections::HashSet<(&str, &str)> = results
        .iter()
        .map(|(f, t, _)| (f.as_str(), t.as_str()))
        .collect();

    // Each (field, term) pair should be unique
    assert_eq!(
        field_term_pairs.len(),
        results.len(),
        "Each (field, term) pair should be unique in results"
    );

    teardown_related_terms_table(&mut conn);
}
