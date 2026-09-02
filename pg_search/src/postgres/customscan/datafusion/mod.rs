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

use datafusion::logical_expr::AggregateUDF;
use std::sync::Arc;

pub mod cardinality_agg;
pub mod explain;
mod expr_translators;
pub mod memory;
pub mod numeric_agg;
pub mod timestamp_to_date;
pub mod translator;

/// Resolve a pg_search aggregate UDAF by name, for the plan codecs. These
/// functions are not in any session registry, so serialized plans (parallel
/// and MPP dispatch) decode them through here.
pub fn udaf_by_name(name: &str) -> Option<Arc<AggregateUDF>> {
    numeric_agg::udaf_by_name(name).or_else(|| cardinality_agg::udaf_by_name(name))
}
