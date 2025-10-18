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

pub mod aggregations;
pub mod groupby;
pub mod limit_offset;
pub mod orderby;
pub mod privdat;
pub mod quals;
pub mod scan_state;
pub mod targetlist;

use crate::gucs;
use crate::nodecast;

use crate::aggregate::{execute_aggregate, AggregateRequest};
use crate::api::HashMap;
use crate::customscan::aggregatescan::aggregations::AggregateCSClause;
use crate::postgres::customscan::aggregatescan::groupby::{GroupByClause, GroupingColumn};
use crate::postgres::customscan::aggregatescan::privdat::{AggregateType, PrivateData};
use crate::postgres::customscan::aggregatescan::scan_state::{AggregateScanState, ExecutionState};
use crate::postgres::customscan::aggregatescan::targetlist::TargetListEntry;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::{
    range_table, CreateUpperPathsHookArgs, CustomScan, ExecMethod, PlainExecCapable,
};
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::TantivyValue;
use crate::postgres::PgSearchRelation;

use pgrx::{pg_sys, IntoDatum, PgList, PgTupleDesc};
use std::ffi::CStr;
use tantivy::aggregation::agg_result::{
    AggregationResult, AggregationResults as TantivyAggregationResults, BucketResult, MetricResult,
};
use tantivy::aggregation::metric::SingleMetricResult as TantivySingleMetricResult;
use tantivy::aggregation::{Key, DEFAULT_BUCKET_LIMIT};
use tantivy::schema::OwnedValue;

#[derive(Default)]
pub struct AggregateScan;

impl CustomScan for AggregateScan {
    const NAME: &'static CStr = c"ParadeDB Aggregate Scan";
    type Args = CreateUpperPathsHookArgs;
    type State = AggregateScanState;
    type PrivateData = PrivateData;

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        // We can only handle single base relations as input
        if builder.args().input_rel().reloptkind != pg_sys::RelOptKind::RELOPT_BASEREL {
            return None;
        }

        let parent_relids = builder.args().input_rel().relids;
        let heap_rti = unsafe { range_table::bms_exactly_one_member(parent_relids)? };
        let heap_rte = unsafe {
            // NOTE: The docs indicate that `simple_rte_array` is always the same length
            // as `simple_rel_array`.
            range_table::get_rte(
                builder.args().root().simple_rel_array_size as usize,
                builder.args().root().simple_rte_array,
                heap_rti,
            )?
        };
        let (table, index) = rel_get_bm25_index(unsafe { (*heap_rte).relid })?;
        let (builder, aggregate_clause) = AggregateCSClause::build(builder, heap_rti, &index)?;

        Some(builder.build(PrivateData {
            heap_rti,
            indexrelid: index.oid(),
            aggregate_clause,
        }))
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        builder.set_scanrelid(builder.custom_private().heap_rti);

        if builder
            .custom_private()
            .aggregate_clause
            .planner_should_replace_aggrefs()
        {
            unsafe {
                let mut cscan = builder.build();
                let plan = &mut cscan.scan.plan;
                replace_aggrefs_in_target_list(plan);
                cscan
            }
        } else {
            builder.build()
        }
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        // EXECUTION-TIME REPLACEMENT: Replace T_Aggref if we have GROUP BY or ORDER BY
        // For simple aggregations without GROUP BY or ORDER BY, replacement should have happened at planning time
        // Now we have the complete reverse logic: replace at execution time if we have any of these conditions
        if !builder
            .custom_private()
            .aggregate_clause
            .planner_should_replace_aggrefs()
        {
            unsafe {
                let cscan = builder.args().cscan;
                let plan = &mut (*cscan).scan.plan;
                replace_aggrefs_in_target_list(plan);
            }
        }

        builder.custom_state().indexrelid = builder.custom_private().indexrelid;
        builder.custom_state().execution_rti =
            unsafe { (*builder.args().cscan).scan.scanrelid as pg_sys::Index };
        builder.custom_state().aggregate_clause = builder.custom_private().aggregate_clause.clone();
        builder.build()
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        explainer.add_text("Index", state.custom_state().indexrel().name());

        // Use pre-computed filter groups from the scan state
        // let filter_groups = &state.custom_state().filter_groups;
        todo!()
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        unsafe {
            let rte = pg_sys::exec_rt_fetch(state.custom_state().execution_rti, estate);
            assert!(!rte.is_null());
            let lockmode = (*rte).rellockmode as pg_sys::LOCKMODE;
            let planstate = state.planstate();
            // TODO: Opening of the index could be deduped between custom scans: see
            // `PdbScanState::open_relations`.
            state.custom_state_mut().open_relations(lockmode);

            state
                .custom_state_mut()
                .init_expr_context(estate, planstate);
            state.runtime_context = state.csstate.ss.ps.ps_ExprContext;
        }
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        state.custom_state_mut().state = ExecutionState::NotStarted;
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        let next = match &mut state.custom_state_mut().state {
            ExecutionState::Completed => {
                return std::ptr::null_mut();
            }
            ExecutionState::NotStarted => {
                // Execute the aggregate, and change the state to Emitting.
                let mut row_iter = execute(state);
                let next = row_iter.next();
                state.custom_state_mut().state = ExecutionState::Emitting(row_iter);
                next
            }
            ExecutionState::Emitting(row_iter) => {
                // Emit the next row.
                row_iter.next()
            }
        };

        let Some(row) = next else {
            state.custom_state_mut().state = ExecutionState::Completed;
            return std::ptr::null_mut();
        };

        unsafe {
            let tupdesc = PgTupleDesc::from_pg_unchecked((*state.planstate()).ps_ResultTupleDesc);
            let slot = pg_sys::MakeTupleTableSlot(
                (*state.planstate()).ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            pg_sys::ExecClearTuple(slot);

            let natts = (*(*slot).tts_tupleDescriptor).natts as usize;
            let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
            let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);
            let mut agg_idx = 0;
            let mut natts_processed = 0;

            // Fill in values according to the target list mapping
            for (i, entry) in state.custom_state().aggregate_clause.entries().enumerate() {
                match entry {
                    &TargetListEntry::GroupingColumn(gc_idx) => {
                        todo!()
                    }
                    TargetListEntry::Aggregate(_) => {
                        let attr = tupdesc.get(i).expect("missing attribute");
                        let expected_typoid = attr.type_oid().value();
                        let datum = SingleMetricResult::new(expected_typoid, row.aggregates[agg_idx].clone())
                            .into_datum();
                        if let Some(datum) = datum {
                            datums[i] = datum;
                            isnull[i] = false;
                        } else {
                            datums[i] = pg_sys::Datum::null();
                            isnull[i] = true;
                        }
                        agg_idx += 1;
                    }
                }
                natts_processed += 1;
            }

            assert_eq!(natts, natts_processed, "target list length mismatch",);

            // Simple finalization - just set the flags and return the slot (no ExecStoreVirtualTuple needed)
            (*slot).tts_flags &= !(pg_sys::TTS_FLAG_EMPTY as u16);
            (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
            (*slot).tts_nvalid = natts as i16;
            slot
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}
}

/// Replace any T_Aggref expressions in the target list with T_FuncExpr placeholders
/// This is called at execution time to avoid "Aggref found in non-Agg plan node" errors
unsafe fn replace_aggrefs_in_target_list(plan: *mut pg_sys::Plan) {
    if (*plan).targetlist.is_null() {
        return;
    }

    let targetlist = (*plan).targetlist;
    let original_tlist = PgList::<pg_sys::TargetEntry>::from_pg((*plan).targetlist);
    let mut new_targetlist = PgList::<pg_sys::TargetEntry>::new();

    for (te_idx, te) in original_tlist.iter_ptr().enumerate() {
        if let Some(aggref) = nodecast!(Aggref, T_Aggref, (*te).expr) {
            // Create a flat copy of the target entry
            let new_te = pg_sys::flatCopyTargetEntry(te);
            // Replace the T_Aggref with a T_FuncExpr placeholder
            let funcexpr = make_placeholder_func_expr(aggref);
            (*new_te).expr = funcexpr as *mut pg_sys::Expr;
            new_targetlist.push(new_te);
        } else {
            // For non-Aggref entries, just make a flat copy
            let copied_te = pg_sys::flatCopyTargetEntry(te);
            new_targetlist.push(copied_te);
        }
    }

    (*plan).targetlist = new_targetlist.into_pg();
}

unsafe fn make_placeholder_func_expr(aggref: *mut pg_sys::Aggref) -> *mut pg_sys::FuncExpr {
    let paradedb_funcexpr: *mut pg_sys::FuncExpr =
        pg_sys::palloc0(size_of::<pg_sys::FuncExpr>()).cast();
    (*paradedb_funcexpr).xpr.type_ = pg_sys::NodeTag::T_FuncExpr;
    (*paradedb_funcexpr).funcid = placeholder_procid();
    (*paradedb_funcexpr).funcresulttype = (*aggref).aggtype;
    (*paradedb_funcexpr).funcretset = false;
    (*paradedb_funcexpr).funcvariadic = false;
    (*paradedb_funcexpr).funcformat = pg_sys::CoercionForm::COERCE_EXPLICIT_CALL;
    (*paradedb_funcexpr).funccollid = pg_sys::InvalidOid;
    (*paradedb_funcexpr).inputcollid = (*aggref).inputcollid;
    (*paradedb_funcexpr).location = (*aggref).location;
    (*paradedb_funcexpr).args = PgList::<pg_sys::Node>::new().into_pg();

    paradedb_funcexpr
}

/// Get the Oid of a placeholder function to use in the target list of aggregate custom scans.
unsafe fn placeholder_procid() -> pg_sys::Oid {
    pgrx::direct_function_call::<pg_sys::Oid>(pg_sys::regprocedurein, &[c"now()".into_datum()])
        .expect("the `now()` function should exist")
}

impl ExecMethod for AggregateScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <AggregateScan as PlainExecCapable>::exec_methods()
    }
}

impl PlainExecCapable for AggregateScan {}

impl SolvePostgresExpressions for AggregateScanState {
    fn has_heap_filters(&mut self) -> bool {
        self.aggregate_clause.query_mut().has_heap_filters()
            || self
                .aggregate_clause
                .aggregates_mut()
                .any(|agg| agg.has_heap_filters())
    }

    fn has_postgres_expressions(&mut self) -> bool {
        self.aggregate_clause.query_mut().has_postgres_expressions()
            || self
                .aggregate_clause
                .aggregates_mut()
                .any(|agg| agg.has_postgres_expressions())
    }

    fn init_postgres_expressions(&mut self, planstate: *mut pg_sys::PlanState) {
        self.aggregate_clause
            .query_mut()
            .init_postgres_expressions(planstate);
        self.aggregate_clause
            .aggregates_mut()
            .for_each(|agg| agg.init_postgres_expressions(planstate));
    }

    fn solve_postgres_expressions(&mut self, expr_context: *mut pg_sys::ExprContext) {
        self.aggregate_clause
            .query_mut()
            .solve_postgres_expressions(expr_context);
        self.aggregate_clause
            .aggregates_mut()
            .for_each(|agg| agg.solve_postgres_expressions(expr_context));
    }
}

pub trait CustomScanClause<CS: CustomScan> {
    type Args;

    fn from_pg(args: &CS::Args, heap_rti: pg_sys::Index, index: &PgSearchRelation) -> Option<Self>
    where
        Self: Sized;

    fn add_to_custom_path(&self, builder: CustomPathBuilder<CS>) -> CustomPathBuilder<CS>;

    fn build(
        builder: CustomPathBuilder<CS>,
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Option<(CustomPathBuilder<CS>, Self)>
    where
        Self: Sized,
    {
        let clause = Self::from_pg(builder.args(), heap_rti, index)?;
        let builder = clause.add_to_custom_path(builder);
        Some((builder, clause))
    }
}

fn execute(
    state: &mut CustomScanStateWrapper<AggregateScan>,
) -> std::vec::IntoIter<AggregationResultsRow> {
    let planstate = state.planstate();
    let expr_context = state.runtime_context;

    state
        .custom_state_mut()
        .prepare_query_for_execution(planstate, expr_context);

    let aggregate_clause = state.custom_state().aggregate_clause.clone();
    let query = aggregate_clause.query().clone();

    let result: AggregationResults = execute_aggregate(
        state.custom_state().indexrel(),
        query,
        AggregateRequest::Sql(aggregate_clause),
        true,                                              // solve_mvcc
        gucs::adjust_work_mem().get().try_into().unwrap(), // memory_limit
        DEFAULT_BUCKET_LIMIT,                              // bucket_limit
    )
    .unwrap_or_else(|e| pgrx::error!("Failed to execute filter aggregation: {}", e))
    .into();

    pgrx::info!("result: {:?}", result);

    result.into_iter()
}

#[derive(Debug)]
struct SingleMetricResult {
    oid: pg_sys::Oid,
    inner: TantivySingleMetricResult,
}

impl SingleMetricResult {
    fn new(oid: pg_sys::Oid, inner: TantivySingleMetricResult) -> Self {
        Self { oid, inner }
    }
}

impl IntoDatum for SingleMetricResult {
    fn into_datum(self) -> Option<pg_sys::Datum> {
        unsafe {
            self.inner.value.and_then(|value| {
                TantivyValue(OwnedValue::F64(value))
                    .try_into_datum(self.oid.into())
                    .unwrap()
            })
        }
    }

    fn type_oid() -> pg_sys::Oid {
        pg_sys::FLOAT8OID
    }
}

#[derive(Debug)]
struct AggregationResults(HashMap<String, AggregationResult>);

impl From<TantivyAggregationResults> for AggregationResults {
    fn from(results: TantivyAggregationResults) -> Self {
        Self(results.0)
    }
}

impl IntoIterator for AggregationResults {
    type Item = AggregationResultsRow;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut rows = Vec::new();
        let key_accumulator = Vec::new();
        self.flatten_into(&mut rows, key_accumulator, None);
        rows.into_iter()
    }
}

#[derive(Debug)]
struct AggregationResultsRow {
    group_keys: Vec<OwnedValue>,
    aggregates: Vec<TantivySingleMetricResult>,
    doc_count: Option<u64>,
}

impl AggregationResults {
    fn flatten_into(
        self,
        rows: &mut Vec<AggregationResultsRow>,
        key_accumulator: Vec<OwnedValue>,
        doc_count: Option<u64>,
    ) {
        for (_name, result) in self.0 {
            match result {
                AggregationResult::BucketResult(bucket) => match bucket {
                    BucketResult::Terms { buckets, .. } => {
                        for bucket_entry in buckets {
                            let mut new_keys = key_accumulator.clone();
                            let doc_count = bucket_entry.doc_count;
                            let key_value = match bucket_entry.key {
                                Key::Str(s) => OwnedValue::Str(s),
                                Key::I64(v) => OwnedValue::I64(v),
                                Key::U64(v) => OwnedValue::U64(v),
                                Key::F64(v) => OwnedValue::F64(v),
                            };

                            new_keys.push(key_value);

                            if !bucket_entry.sub_aggregation.0.is_empty() {
                                AggregationResults(bucket_entry.sub_aggregation.0).flatten_into(
                                    rows,
                                    new_keys,
                                    Some(doc_count),
                                );
                            } else {
                                rows.push(AggregationResultsRow {
                                    group_keys: new_keys,
                                    aggregates: Vec::new(),
                                    doc_count: Some(doc_count),
                                });
                            }
                        }
                    }
                    _ => {
                        todo!("support other bucket results");
                    }
                },

                AggregationResult::MetricResult(metric) => {
                    let single = match metric {
                        MetricResult::Average(result)
                        | MetricResult::Count(result)
                        | MetricResult::Sum(result)
                        | MetricResult::Min(result)
                        | MetricResult::Max(result) => result,
                        unknown => todo!("support other metric results: {:?}", unknown),
                    };

                    pgrx::info!("single: {:?}", single);

                    if let Some(existing_row) = rows.last_mut() {
                        existing_row.aggregates.push(single);
                    } else {
                        rows.push(AggregationResultsRow {
                            group_keys: key_accumulator.clone(),
                            aggregates: vec![single],
                            doc_count,
                        });
                    }
                }
            }
        }
    }
}
