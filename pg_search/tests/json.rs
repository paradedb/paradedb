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
use rstest::*;
use sqlx::PgConnection;

// In addition to checking whether all the expected types work for keys, make sure to include tests for anything that
//    is reliant on keys (e.g. stable_sort, alias)

#[rstest]
fn json_datatype(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id serial8,
        value json
    );

    INSERT INTO test_table (value) VALUES ('{"currency_code": "USD", "salary": 120000 }');
    INSERT INTO test_table (value) VALUES ('{"currency_code": "USD", "salary": 75000 }');
    INSERT INTO test_table (value) VALUES ('{"currency_code": "USD", "salary": 140000 }');
    "#
    .execute(&mut conn);

    // if we don't segfault postgres here, we're good
    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_table',
        index_name => 'test_index',
        key_field => 'id',
        json_fields => paradedb.field('value', indexed => true, fast => true)
    );
    "#
    .execute(&mut conn);
}
