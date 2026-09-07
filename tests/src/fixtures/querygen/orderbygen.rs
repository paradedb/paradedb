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

use proptest::prelude::*;

#[derive(Clone, Copy, Debug)]
enum JoinScanOrderKind {
    Columns,
    IndexedExpression,
    QuantityIsNull,
    QuantityIsNotNull,
    IndexedExpressionThenQuantityIsNull,
    IndexedExpressionThenQuantityIsNotNull,
}

fn order_parts(kind: JoinScanOrderKind, tables: &[String]) -> Vec<String> {
    let first_table = tables.first().expect("a join must use at least one table");
    let mut parts = match kind {
        JoinScanOrderKind::Columns => Vec::new(),
        JoinScanOrderKind::IndexedExpression => {
            vec![format!("upper({first_table}.category)")]
        }
        JoinScanOrderKind::QuantityIsNull => vec![
            format!("{first_table}.quantity IS NULL"),
            format!("{first_table}.quantity"),
        ],
        JoinScanOrderKind::QuantityIsNotNull => vec![
            format!("{first_table}.quantity IS NOT NULL"),
            format!("{first_table}.quantity"),
        ],
        JoinScanOrderKind::IndexedExpressionThenQuantityIsNull => vec![
            format!("upper({first_table}.category)"),
            format!("{first_table}.quantity IS NULL"),
            format!("{first_table}.quantity"),
        ],
        JoinScanOrderKind::IndexedExpressionThenQuantityIsNotNull => vec![
            format!("upper({first_table}.category)"),
            format!("{first_table}.quantity IS NOT NULL"),
            format!("{first_table}.quantity"),
        ],
    };

    parts.push(format!("{first_table}.id"));
    // Use every table's ID to make LIMIT deterministic across joined rows.
    parts.extend(tables.iter().skip(1).map(|table| format!("{table}.id")));

    parts
}

/// Generate deterministic JoinScan `ORDER BY` parts that are valid for the selected projection.
///
/// When restricted, only projected ID columns are used. Unrestricted cases preserve all six
/// combinations (regular columns, indexed expression, and nullable predicates). When DISTINCT
/// is active, the caller projects any expression present in `ORDER BY` to satisfy PostgreSQL's
/// projection requirements.
pub fn arb_joinscan_order_parts(
    tables: Vec<String>,
    restrict_to_projected_columns: bool,
) -> BoxedStrategy<Vec<String>> {
    if restrict_to_projected_columns {
        Just(order_parts(JoinScanOrderKind::Columns, &tables)).boxed()
    } else {
        prop_oneof![
            Just(JoinScanOrderKind::Columns),
            Just(JoinScanOrderKind::IndexedExpression),
            Just(JoinScanOrderKind::QuantityIsNull),
            Just(JoinScanOrderKind::QuantityIsNotNull),
            Just(JoinScanOrderKind::IndexedExpressionThenQuantityIsNull),
            Just(JoinScanOrderKind::IndexedExpressionThenQuantityIsNotNull),
        ]
        .prop_map(move |kind| order_parts(kind, &tables))
        .boxed()
    }
}
