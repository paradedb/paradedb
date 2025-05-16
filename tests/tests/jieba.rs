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
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

// Helper function to run tokenize and collect results
fn get_tokens(conn: &mut PgConnection, tokenizer_type: &str, text: &str) -> Vec<(String, i32)> {
    let query_str = format!(
        "SELECT token, position FROM paradedb.tokenize(paradedb.tokenizer('{}'), '{}') ORDER BY position;",
        tokenizer_type, text
    );
    query_str.fetch(conn)
}

#[rstest]
fn test_jieba_tokenizer_basic(mut conn: PgConnection) {
    // Test the paradedb.tokenize function directly
    let tokens = get_tokens(&mut conn, "jieba", "我们都有光明的前途");
    assert_eq!(
        tokens,
        vec![
            ("我们".to_string(), 0),
            ("都".to_string(), 2),
            ("有".to_string(), 3),
            ("光明".to_string(), 4),
            ("的".to_string(), 6),
            ("前途".to_string(), 7),
        ],
        "Failed on '我们都有光明的前途'"
    );

    let tokens = get_tokens(&mut conn, "jieba", "李宇");
    assert_eq!(tokens, vec![("李宇".to_string(), 0),], "Failed on '李宇'");

    let tokens = get_tokens(&mut conn, "jieba", "公安");
    assert_eq!(tokens, vec![("公安".to_string(), 0),], "Failed on '公安'");

    let tokens = get_tokens(&mut conn, "jieba", "转移就业");
    assert_eq!(
        tokens,
        vec![("转移".to_string(), 0), ("就业".to_string(), 2),],
        "Failed on '转移就业'"
    );
}

#[rstest]
fn test_jieba_tokenizer_indexing(mut conn: PgConnection) {
    // Create a table and index using the jieba tokenizer
    r#"CREATE TABLE chinese_texts (
            id SERIAL PRIMARY KEY,
            content TEXT
        );"#
    .execute(&mut conn);

    r#"INSERT INTO chinese_texts (content) VALUES
            ('我们都有光明的前途'),
            ('李宇给公安局打了电话'),
            ('这项政策旨在促进劳动力转移就业');"#
        .execute(&mut conn);

    r#"CREATE INDEX chinese_texts_idx ON chinese_texts
        USING bm25 (id, content)
        WITH (
            key_field = 'id',
            text_fields = '{
                "content": { "tokenizer": {"type": "jieba"} }
            }'
        );"#
    .execute(&mut conn);

    // Test searching using fetch/fetch_one extension methods
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM chinese_texts WHERE chinese_texts @@@ 'content:光明' ORDER BY id"#
            .fetch(&mut conn);
    assert_eq!(rows, vec![(1,)], "Failed on 'content:光明'");

    let row: (i32,) =
        r#"SELECT id FROM chinese_texts WHERE chinese_texts @@@ 'content:公安局' ORDER BY id"#
            .fetch_one(&mut conn);
    assert_eq!(row, (2,), "Failed on 'content:公安局'");

    let row: (i32,) =
        r#"SELECT id FROM chinese_texts WHERE chinese_texts @@@ 'content:就业' ORDER BY id"#
            .fetch_one(&mut conn);
    assert_eq!(row, (3,), "Failed on 'content:就业'");
}
