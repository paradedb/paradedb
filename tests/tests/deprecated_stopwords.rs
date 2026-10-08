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

// Custom stopword lists do not yet have a working tokenizer cast equivalent.

use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

#[rstest]
fn stopwords_tokenizer_config(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_paradedb_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

    CREATE INDEX bm25_search_idx ON paradedb.bm25_search
        USING paradedb (id, description)
        WITH (text_fields='{"description": {"tokenizer": {"type": "default", "stopwords": ["shoes"]}}}');
    "#
    .execute(&mut conn);

    let count: (i64,) = "
    SELECT COUNT(*) FROM paradedb.bm25_search
    WHERE description ||| 'shoes'"
        .fetch_one(&mut conn);
    assert_eq!(count.0, 0);
}
