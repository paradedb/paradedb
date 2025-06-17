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
/// it will always generate an `ORDER BY` including one of the given tiebreaker columns (which are
/// assumed to be unique).
pub fn arb_paging_exprs(
    table: impl AsRef<str>,
    columns: Vec<&str>,
    tiebreaker_columns: Vec<&str>,
) -> impl Strategy<Value = String> {
    let columns = columns
        .into_iter()
        .map(|col| format!("{}.{col}", table.as_ref()))
        .collect::<Vec<_>>();
    let columns_len = columns.len();
    let tiebreaker_columns = tiebreaker_columns
        .into_iter()
        .map(|col| format!("{}.{}", table.as_ref(), col))
        .collect::<Vec<_>>();

    let order_by_prefix = if columns_len > 0 {
        proptest::sample::subsequence(columns, 0..columns_len).boxed()
    } else {
        Just(vec![]).boxed()
    };

    // Choose a prefix of columns to `order by`, and a tiebreaker column to ensure determinism.
    (
        order_by_prefix,
        proptest::sample::select(tiebreaker_columns),
    )
        .prop_flat_map(move |(mut order_by_prefix, tiebreaker)| {
            order_by_prefix.push(tiebreaker);
            (
                Just(order_by_prefix),
                proptest::option::of(0..100_usize),
                proptest::option::of(0..100_usize),
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
