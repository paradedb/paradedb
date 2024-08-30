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
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;
use std::collections::HashSet;

#[rstest]
fn stable_sort_true(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => 'description:book',
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![37, 7]);
}

#[rstest]
fn stable_sort_false(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => 'description:book',
        stable_sort => false
    );
    "#
    .fetch_collect(&mut conn);

    let ids: HashSet<i32> = HashSet::from_iter(columns.id.clone());
    let expected: HashSet<i32> = HashSet::from_iter(vec![37, 7]);
    assert_eq!(ids, expected);
}
