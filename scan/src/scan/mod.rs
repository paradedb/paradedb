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

pub use index_read::scan::*;

pub mod batch_scanner;
pub mod deferred_aggregate_rule;
pub mod deferred_lookup;
pub mod deferred_placement_rule;
pub mod execution_plan;
pub mod filter_passthrough_exec;
pub mod filter_pushdown;
pub mod late_materialization;
pub mod lazy_sort;
pub mod plan_walk;
pub mod pre_filter;
pub mod propagate_empty_unnest_rule;
pub mod range_partitioning;
pub mod rank_join_exec;
pub mod rank_join_rule;
pub mod segmented_topk_exec;
pub mod segmented_topk_rule;
pub mod table_provider;
pub mod tantivy_decode_exec;
pub mod tantivy_fetch_exec;
pub mod visibility_ctid_resolver_rule;

pub use batch_scanner::Scanner;
pub use execution_plan::CtidResolver;
pub use info::{
    GlobalPredicateIndex, ScanEstimateBasis, ScanInfo, ScanMode, TagIndex, TaggedQuery,
};
pub use table_provider::{PgSearchTableProvider, VisibilityMode};
