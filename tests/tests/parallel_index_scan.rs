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

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn index_scan_under_parallel_path(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    r#"
        set paradedb.enable_custom_scan to off;
        set enable_indexonlyscan to off;
        set debug_parallel_query to on;
    "#
    .execute(&mut conn);

    let count = r#"
        select count(1) from paradedb.bm25_search where description @@@ 'shoes';
    "#
    .fetch::<(i64,)>(&mut conn);
    assert_eq!(count, vec![(3,)]);
}
