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

//! Shared DataFusion infrastructure for the Postgres custom scan providers.
//!
//! `JoinScan` and `AggregateScan` both lower their work into Apache DataFusion.
//! This module collects the pieces they share — the memory pool, predicate /
//! expression translators, and EXPLAIN-output formatters — into one neutral
//! namespace so neither scan has to reach into the other for non-scan-specific
//! code.
//!
//! Future phases of the dedup work will move the shared session-builder helpers
//! and the `RelNode` family of relation-tree types into this module as well.

use datafusion::common::ScalarValue;
use datafusion::error::{DataFusionError, Result};
use datafusion::logical_expr::function::AccumulatorArgs;
use datafusion::logical_expr::{AggregateUDF, ScalarUDF};
use datafusion::physical_plan::expressions::Literal;
use std::sync::Arc;

pub mod cardinality_agg;
pub mod explain;
mod expr_translators;
pub mod memory;
pub mod numeric_agg;
pub mod spill;
pub mod timestamp_to_date;
pub mod translator;

/// All pg_search aggregate UDAFs, registered into SessionState so plans referencing
/// them can resolve them by name across plan serialization and dispatch.
pub fn all_pg_search_udafs() -> Vec<Arc<AggregateUDF>> {
    vec![
        numeric_agg::numeric64_sum_udaf(),
        numeric_agg::numeric64_avg_udaf(),
        numeric_agg::numeric_bytes_sum_udaf(),
        numeric_agg::numeric_bytes_avg_udaf(),
        cardinality_agg::tantivy_cardinality_udaf(),
    ]
}

/// All stateless pg_search scalar UDFs registered into SessionState.
pub fn all_pg_search_udfs() -> Vec<Arc<ScalarUDF>> {
    vec![timestamp_to_date::timestamp_to_date_udf()]
}

/// Resolve a pg_search aggregate UDAF by name, for the plan codecs.
pub fn udaf_by_name(name: &str) -> Option<Arc<AggregateUDF>> {
    all_pg_search_udafs()
        .into_iter()
        .find(|udaf| udaf.name() == name)
}

/// The literal argument at `index` of a UDAF call. A per-call setting travels
/// as a plan literal so it survives plan serialization for parallel and MPP
/// execution.
pub(crate) fn literal_arg<'a>(
    args: &'a AccumulatorArgs,
    index: usize,
    name: &str,
    what: &str,
) -> Result<&'a ScalarValue> {
    let expr = args
        .exprs
        .get(index)
        .ok_or_else(|| DataFusionError::Internal(format!("{name} requires a {what} argument")))?;
    let literal = expr
        .as_ref()
        .downcast_ref::<Literal>()
        .ok_or_else(|| DataFusionError::Internal(format!("{name} {what} must be a literal")))?;
    Ok(literal.value())
}

pub(crate) fn reject_distinct(args: &AccumulatorArgs, name: &str) -> Result<()> {
    if args.is_distinct {
        return Err(DataFusionError::NotImplemented(format!(
            "{name} does not support DISTINCT"
        )));
    }
    Ok(())
}
