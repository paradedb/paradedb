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
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn only_one_index_allowed(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    )
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX index_one ON public.mock_items
    USING bm25 (id, description)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    match r#"
    CREATE INDEX index_two ON public.mock_items
    USING bm25 (id, description)
    WITH (key_field = 'id');
    "#
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("created a second `USING bm25` index"),
        Err(e) if format!("{e}").contains("a relation may only have one `USING bm25` index") => (), // all good
        Err(e) => panic!("{}", e),
    }
}
