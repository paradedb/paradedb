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

use crate::aggregate::agg_spec::AggregationSpec;
use crate::api::OrderByInfo;
use crate::postgres::customscan::aggregatescan::privdat::{
    AggregateType, AggregateValue, GroupingColumn, TargetListEntry,
};
use crate::postgres::customscan::CustomScanState;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;
use tantivy::schema::OwnedValue;

use pgrx::pg_sys;
use tinyvec::TinyVec;

pub type AggregateRow = TinyVec<[AggregateValue; 4]>;

// For GROUP BY results, we need both the group keys and aggregate values
#[derive(Debug, Clone)]
pub struct GroupedAggregateRow {
    pub group_keys: Vec<OwnedValue>, // The values of the grouping columns
    pub aggregate_values: AggregateRow,
}

#[derive(Default)]
pub enum ExecutionState {
    #[default]
    NotStarted,
    Emitting(std::vec::IntoIter<GroupedAggregateRow>),
    Completed,
}

#[derive(Default)]
pub struct AggregateScanState {
    // The state of this scan.
    pub state: ExecutionState,
    // The aggregation specification (agg_types, grouping_columns)
    pub agg_spec: AggregationSpec,
    // The ORDER BY information for sorting
    pub orderby_info: Vec<OrderByInfo>,
    // Maps target list position to data type
    pub target_list_mapping: Vec<TargetListEntry>,
    // The query that will be executed.
    pub query: SearchQueryInput,
    // The index that will be scanned.
    pub indexrelid: pg_sys::Oid,
    // The index relation. Opened during `begin_custom_scan`.
    pub indexrel: Option<(pg_sys::LOCKMODE, PgSearchRelation)>,
    // The execution time RTI (note: potentially different from the planning-time RTI).
    pub execution_rti: pg_sys::Index,
    // The LIMIT, if GROUP BY ... ORDER BY ... LIMIT is present
    pub limit: Option<u32>,
    // The OFFSET, if GROUP BY ... ORDER BY ... LIMIT is present
    pub offset: Option<u32>,
    // Whether a GROUP BY could be lossy (i.e. some buckets truncated)
    pub maybe_truncated: bool,
    // Filter groups for optimization (filter_expr, aggregate_indices)
    pub filter_groups: Vec<super::FilterGroup>,
}

impl AggregateScanState {
    pub fn open_relations(&mut self, lockmode: pg_sys::LOCKMODE) {
        self.indexrel = Some((
            lockmode,
            PgSearchRelation::with_lock(self.indexrelid, lockmode),
        ));
    }

    #[inline(always)]
    pub fn indexrel(&self) -> &PgSearchRelation {
        self.indexrel
            .as_ref()
            .map(|(_, rel)| rel)
            .expect("PdbScanState: indexrel should be initialized")
    }

    // Convenience accessors for agg_spec fields
    #[inline(always)]
    pub fn aggregate_types(&self) -> &[AggregateType] {
        &self.agg_spec.aggs
    }

    #[inline(always)]
    pub fn aggregate_types_mut(&mut self) -> &mut [AggregateType] {
        &mut self.agg_spec.aggs
    }

    #[inline(always)]
    pub fn grouping_columns(&self) -> &[GroupingColumn] {
        &self.agg_spec.groupby
    }
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}
