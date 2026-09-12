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

//! Generators for global window aggregates (empty `OVER ()`) in join target
//! lists, covering the shapes JoinScan absorbs (#5637): bare aggregates,
//! cast/function wrappers evaluated through `PgExprUdf`, source columns mixed
//! with window values, and NUMERIC (storage-encoded) argument columns — plus
//! shapes JoinScan declines, so the PostgreSQL-fallback results are compared
//! too.

use proptest::prelude::*;

/// Integer-typed argument columns (`quantity` is nullable, exercising
/// null-skipping aggregation on both sides).
const INT_ARG_COLUMNS: &[&str] = &["age", "quantity"];

/// NUMERIC-typed argument columns, spanning Numeric64 storages and scales.
/// `big_numeric` (unbounded NUMERIC) is deliberately excluded: non-COUNT
/// window aggregates over it raise a planning error inside the scan
/// ("declare a precision and scale") rather than declining.
const NUMERIC_ARG_COLUMNS: &[&str] = &["price", "small_numeric", "int_numeric", "high_scale"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowAggKind {
    CountStar,
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

impl WindowAggKind {
    fn func(&self) -> &'static str {
        match self {
            WindowAggKind::CountStar | WindowAggKind::Count => "COUNT",
            WindowAggKind::Sum => "SUM",
            WindowAggKind::Avg => "AVG",
            WindowAggKind::Min => "MIN",
            WindowAggKind::Max => "MAX",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowWrapper {
    /// `AGG(col) OVER ()` standing alone.
    Bare,
    /// `(AGG(col) OVER ())::float8`
    CastFloat8,
    /// `AGG(col) OVER () + 1`
    PlusOne,
    /// `<qualified int column> + AGG(col) OVER ()`
    PlusColumn(String),
    /// `round(AGG(col) OVER (), 2)::float8`
    RoundCastFloat8,
}

#[derive(Debug, Clone)]
pub struct WindowTarget {
    pub kind: WindowAggKind,
    /// Qualified argument column; `None` for `COUNT(*)`.
    pub arg: Option<String>,
    arg_is_numeric: bool,
    pub wrapper: WindowWrapper,
}

impl WindowTarget {
    fn window_sql(&self) -> String {
        match &self.arg {
            None => "COUNT(*) OVER ()".to_string(),
            Some(col) => format!("{}({col}) OVER ()", self.kind.func()),
        }
    }

    pub fn to_sql(&self) -> String {
        let w = self.window_sql();
        match &self.wrapper {
            WindowWrapper::Bare => w,
            WindowWrapper::CastFloat8 => format!("({w})::float8"),
            WindowWrapper::PlusOne => format!("{w} + 1"),
            WindowWrapper::PlusColumn(col) => format!("{col} + {w}"),
            WindowWrapper::RoundCastFloat8 => format!("round({w}, 2)::float8"),
        }
    }

    /// PG result type of the bare window function is NUMERIC for AVG and for
    /// any non-COUNT aggregate over a NUMERIC column.
    fn numeric_result(&self) -> bool {
        match self.kind {
            WindowAggKind::CountStar | WindowAggKind::Count => false,
            WindowAggKind::Avg => true,
            WindowAggKind::Sum | WindowAggKind::Min | WindowAggKind::Max => self.arg_is_numeric,
        }
    }

    /// Whether JoinScan can absorb this target. Bare aggregates always can
    /// (the direct window-output path handles NUMERIC results), and casts
    /// yield float8; arithmetic wrappers around a NUMERIC-result aggregate
    /// produce a NUMERIC expression, which is not Arrow-convertible and
    /// declines the whole path (falling back to PostgreSQL's WindowAgg).
    pub fn is_absorbable(&self) -> bool {
        match &self.wrapper {
            WindowWrapper::Bare | WindowWrapper::CastFloat8 | WindowWrapper::RoundCastFloat8 => {
                true
            }
            WindowWrapper::PlusOne | WindowWrapper::PlusColumn(_) => !self.numeric_result(),
        }
    }
}

fn arb_window_target(tables: Vec<String>) -> BoxedStrategy<WindowTarget> {
    let arb_kind = prop_oneof![
        Just(WindowAggKind::CountStar),
        Just(WindowAggKind::Count),
        Just(WindowAggKind::Sum),
        Just(WindowAggKind::Avg),
        Just(WindowAggKind::Min),
        Just(WindowAggKind::Max),
    ];
    (
        arb_kind,
        any::<prop::sample::Index>(),
        any::<prop::sample::Index>(),
        0..5usize,
        any::<bool>(),
    )
        .prop_map(
            move |(kind, table_sel, column_sel, wrapper_sel, numeric_arg)| {
                let table = table_sel.get(&tables);
                let (arg, arg_is_numeric) = if kind == WindowAggKind::CountStar {
                    (None, false)
                } else if numeric_arg {
                    (
                        Some(format!("{table}.{}", column_sel.get(NUMERIC_ARG_COLUMNS))),
                        true,
                    )
                } else {
                    (
                        Some(format!("{table}.{}", column_sel.get(INT_ARG_COLUMNS))),
                        false,
                    )
                };
                let wrapper = match wrapper_sel {
                    0 => WindowWrapper::Bare,
                    1 => WindowWrapper::CastFloat8,
                    2 => WindowWrapper::PlusOne,
                    3 => WindowWrapper::PlusColumn(format!("{}.age", tables[0])),
                    _ => WindowWrapper::RoundCastFloat8,
                };
                WindowTarget {
                    kind,
                    arg,
                    arg_is_numeric,
                    wrapper,
                }
            },
        )
        .boxed()
}

/// Up to three window aggregate targets over the joined tables (zero keeps
/// the plain non-window case in the mix). Duplicate aggregates across
/// targets arise naturally, exercising the shared window column
/// deduplication.
pub fn arb_window_targets(tables: Vec<String>) -> BoxedStrategy<Vec<WindowTarget>> {
    prop::collection::vec(arb_window_target(tables), 0..=3).boxed()
}
