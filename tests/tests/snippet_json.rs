// Copyright (c) 2023-2025 ParadeDB, Inc.
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
use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use rstest::*;
use sqlx::PgConnection;

// Common words that are likely to appear in text and be searched for
const COMMON_WORDS: &[&str] = &[
    "test",
    "search",
    "highlight",
    "text",
    "content",
    "data",
    "json",
    "array",
    "value",
    "field",
    "object",
    "property",
    "snippet",
    "match",
    "result",
    "query",
    "index",
    "database",
    "document",
    "information",
];

fn arb_text_with_common_words() -> impl Strategy<Value = String> {
    Just(COMMON_WORDS.join(" "))
}

fn arb_json_data() -> impl Strategy<Value = (String, String)> {
    let text = arb_text_with_common_words();
    let words_array = COMMON_WORDS.to_vec();

    text.prop_map(move |text| {
        let json = format!(
            r#"{{
                "level1": {{
                    "level2": {{
                        "level3": {{
                            "text": "{}",
                            "array": {:?}
                        }}
                    }}
                }}
            }}"#,
            text, words_array
        );
        (json, text)
    })
}

fn arb_json_operator() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "->".to_string(),
        "->>".to_string(),
        "#>".to_string(),
        "#>>".to_string(),
    ])
}

fn arb_json_path() -> impl Strategy<Value = (String, String)> {
    prop::sample::select(vec![
        (
            "level1->level2->level3->text".to_string(),
            "level1.level2.level3.text".to_string(),
        ),
        (
            "level1->level2->level3->array".to_string(),
            "level1.level2.level3.array".to_string(),
        ),
        (
            "level1,level2,level3,text".to_string(),
            "level1.level2.level3.text".to_string(),
        ),
        (
            "level1,level2,level3,array".to_string(),
            "level1.level2.level3.array".to_string(),
        ),
    ])
}

fn arb_search_term() -> impl Strategy<Value = String> {
    prop::sample::select(COMMON_WORDS.to_vec()).prop_map(|word| word.to_string())
}

fn format_path(operator: &str, path: &str) -> String {
    match operator {
        "#>" | "#>>" => format!("'{{{}}}'", path.replace("->", ",").replace("'", "")),
        _ => format!("'{}'", path),
    }
}

fn format_error(message: &str, insert_sql: &str, query: &str) -> String {
    format!("\n{}\n{}\n{}\n", message, insert_sql, query)
}

fn verify_snippet_results(
    results: &[(Option<String>,)],
    search_term: &str,
    query: &str,
    insert_sql: &str,
) -> Result<(), String> {
    if results.is_empty() {
        return Err(format_error("Query returned no results", insert_sql, query));
    }

    for (snippet,) in results {
        let snippet = match snippet {
            Some(s) => s,
            None => continue,
        };

        if snippet.is_empty() {
            continue;
        }

        if !snippet.to_lowercase().contains(&search_term.to_lowercase()) {
            return Err(format_error(
                &format!(
                    "Snippet '{}' does not contain search term '{}'",
                    snippet, search_term
                ),
                insert_sql,
                query,
            ));
        }
    }
    Ok(())
}

#[rstest]
#[tokio::test]
async fn json_snippet_highlighting(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let setup_sql = r#"
    CREATE EXTENSION IF NOT EXISTS pg_search;

    DROP TABLE IF EXISTS json_snippet_test;
    CREATE TABLE json_snippet_test (
        id SERIAL PRIMARY KEY,
        content_json JSON,
        content_jsonb JSONB
    );

    CREATE INDEX ON json_snippet_test USING bm25 (
        id,
        content_json,
        content_jsonb
    ) WITH (
        key_field = 'id'
    );
    "#;

    setup_sql.execute(&mut pool.pull());

    proptest!(|(
        (json_data, _) in arb_json_data(),
        operator in arb_json_operator(),
        (path, parse_path) in arb_json_path(),
        search_term in arb_search_term(),
    )| {
        let insert_sql = format!(
            "INSERT INTO json_snippet_test (content_json, content_jsonb) VALUES ('{}', '{}')",
            json_data, json_data
        );
        insert_sql.clone().execute(&mut pool.pull());

        let formatted_path = format_path(&operator, &path);

        for field_type in &["content_json", "content_jsonb"] {
            let query = format!(
                "SELECT paradedb.snippet({}{}{}) FROM json_snippet_test WHERE id @@@ paradedb.parse('{}.{}:{}')",
                field_type, operator, formatted_path, field_type, parse_path, search_term
            );
            let results: Vec<(Option<String>,)> = query.clone().fetch(&mut pool.pull());

            if let Err(e) = verify_snippet_results(&results, &search_term, &query, &insert_sql) {
                prop_assert!(false, "{}", e);
            }
        }
    });
}
