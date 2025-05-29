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

use std::fmt::{Debug, Display};

use proptest::prelude::*;

#[derive(Clone, Debug)]
pub struct PagingExprs {
    order_by: Vec<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

impl PagingExprs {
    pub fn to_sql(&self) -> String {
        let mut sql = String::new();

        let mut order_bys = self.order_by.iter();
        if let Some(order_by) = order_bys.next() {
            sql.push_str("ORDER BY ");
            sql.push_str(order_by);
        }
        for order_by in order_bys {
            sql.push_str(", ");
            sql.push_str(order_by);
        }

        if let Some(offset) = &self.offset {
            if !sql.is_empty() {
                sql.push(' ');
            }
            sql.push_str("OFFSET ");
            sql.push_str(&offset.to_string());
        }

        if let Some(limit) = &self.limit {
            if !sql.is_empty() {
                sql.push(' ');
            }
            sql.push_str("LIMIT ");
            sql.push_str(&limit.to_string());
        }
        sql
    }
}

/// Generate arbitrary `ORDER BY`, `OFFSET`, and `LIMIT` expressions.
///
/// This strategy limits itself to combinations which allow for deterministic comparison:
/// it will always generate an `ORDER BY` including (at least) the given tiebreaker column.
pub fn arb_paging_exprs(
    table: impl AsRef<str>,
    order_by_tiebreaker: impl AsRef<str>,
    columns: Vec<impl AsRef<str>>,
) -> impl Strategy<Value = String> {
    let columns = columns
        .into_iter()
        .map(|col| format!("{}.{}", table.as_ref(), col.as_ref()))
        .collect::<Vec<_>>();
    let columns_len = columns.len();

    // Choose `order_by`.
    proptest::sample::subsequence(columns, 0..columns_len)
        .prop_flat_map(move |mut order_by| {
            order_by.push(order_by_tiebreaker.as_ref().to_owned());
            (
                Just(order_by),
                prop_oneof![Just(None), Just(Some(0usize)), Just(Some(10usize))].boxed(),
                prop_oneof![Just(None), Just(Some(10usize))].boxed(),
            )
        })
        .prop_map(|(order_by, offset, limit)| {
            PagingExprs {
                order_by,
                offset,
                limit,
            }
            .to_sql()
        })
}
