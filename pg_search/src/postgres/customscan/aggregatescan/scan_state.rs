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

use crate::postgres::customscan::aggregatescan::privdat::AggregateType;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;

use pgrx::pg_sys;
use tinyvec::TinyVec;

// TODO: This should match the output types of the extracted aggregate functions. For now we only
// support COUNT.
pub type AggregateRow = TinyVec<[i64; 4]>;

#[derive(Default)]
pub enum ExecutionState {
    #[default]
    NotStarted,
    Emitting(std::vec::IntoIter<AggregateRow>),
    Completed,
}

#[derive(Default)]
pub struct AggregateScanState {
    // The state of this scan.
    pub state: ExecutionState,
    // The aggregate types that we are executing for.
    pub aggregate_types: Vec<AggregateType>,
    // The query that will be executed.
    pub query: SearchQueryInput,
    // The index that will be scanned.
    pub indexrelid: pg_sys::Oid,
    // The index relation. Opened during `begin_custom_scan`.
    pub indexrel: Option<(pg_sys::LOCKMODE, PgSearchRelation)>,
    // The execution time RTI (note: potentially different from the planning-time RTI).
    pub execution_rti: pg_sys::Index,
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

    pub fn aggregates_to_json(&self) -> serde_json::Value {
        serde_json::Value::Object(
            self.aggregate_types
                .iter()
                .enumerate()
                .map(|(idx, aggregate)| (idx.to_string(), aggregate.to_json()))
                .collect(),
        )
    }

    pub fn json_to_aggregate_results(&self, result: serde_json::Value) -> Vec<AggregateRow> {
        let result_map = result
            .as_object()
            .expect("unexpected aggregate result collection type");

        let row = self
            .aggregate_types
            .iter()
            .enumerate()
            .map(move |(idx, aggregate)| {
                let aggregate_val = result_map
                    .get(&idx.to_string())
                    .expect("missing aggregate result")
                    .as_object()
                    .expect("unexpected aggregate structure")
                    .get("value")
                    .expect("missing aggregate result value")
                    .as_number()
                    .expect("unexpected aggregate result type");

                aggregate.result_from_json(aggregate_val)
            })
            .collect::<AggregateRow>();

        vec![row]
    }
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}
