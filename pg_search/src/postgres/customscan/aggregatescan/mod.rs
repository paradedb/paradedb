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
pub mod query;
pub mod scan_state;
pub mod targetlist;

use crate::gucs;
use crate::nodecast;

use crate::aggregate::{execute_aggregate, AggregateRequest};
use crate::api::HashMap;
use crate::customscan::aggregatescan::aggregations::{
    AggregateCSClause, AggregationKey, FilterSentinelKey, GroupedKey,
};
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
        explainer.add_query(state.custom_state().aggregate_clause.query());
        state
            .custom_state()
            .aggregate_clause
            .add_to_explainer(explainer);
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

            let mut aggregates = row.aggregates.clone().into_iter();
            let mut natts_processed = 0;

            // Fill in values according to the target list mapping
            for (i, entry) in state.custom_state().aggregate_clause.entries().enumerate() {
                let attr = tupdesc.get(i).expect("missing attribute");
                let expected_typoid = attr.type_oid().value();

                let datum = match (entry, row.is_empty()) {
                    (TargetListEntry::GroupingColumn(gc_idx), false) => row.group_keys[*gc_idx]
                        .clone()
                        .try_into_datum(pgrx::PgOid::from(expected_typoid))
                        .expect("should be able to convert to datum"),
                    (TargetListEntry::GroupingColumn(_), true) => None,
                    (TargetListEntry::Aggregate(agg_type), false) => {
                        if agg_type.can_use_doc_count()
                            && !state.custom_state().aggregate_clause.has_filter()
                            && state.custom_state().aggregate_clause.has_groupby()
                        {
                            row.doc_count()
                                .try_into_datum(pgrx::PgOid::from(expected_typoid))
                                .expect("should be able to convert to datum")
                        } else {
                            SingleMetricResult::new(
                                expected_typoid,
                                aggregates
                                    .next()
                                    .and_then(|v| v)
                                    .unwrap_or_else(|| agg_type.nullish()),
                            )
                            .into_datum()
                        }
                    }
                    (TargetListEntry::Aggregate(agg_type), true) => {
                        SingleMetricResult::new(expected_typoid, agg_type.nullish()).into_datum()
                    }
                };

                if let Some(datum) = datum {
                    datums[i] = datum;
                    isnull[i] = false;
                } else {
                    datums[i] = pg_sys::Datum::null();
                    isnull[i] = true;
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

    fn explain_output(&self) -> Box<dyn Iterator<Item = (String, String)>> {
        Box::new(std::iter::empty())
    }

    fn explain_needs_indent(&self) -> bool {
        true
    }

    fn add_to_explainer(&self, explainer: &mut Explainer) {
        for (key, value) in self.explain_output() {
            explainer.add_text(&format!("  {}", key), &value);
        }
    }

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

    if result.is_empty() {
        if state.custom_state().aggregate_clause.has_groupby() {
            vec![].into_iter()
        } else {
            vec![AggregationResultsRow::default()].into_iter()
        }
    } else {
        result.into_iter()
    }
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
        let mut result = Vec::new();
        match self.style() {
            AggregationStyle::Ungrouped => self.flatten_ungrouped(&mut result),
            AggregationStyle::Grouped => self.flatten_grouped(&mut result),
            AggregationStyle::GroupedWithFilter => self.flatten_grouped_with_filter(&mut result),
        }

        result.into_iter()
    }
}

#[derive(Debug, Default)]
struct AggregationResultsRow {
    group_keys: Vec<TantivyValue>,
    aggregates: Vec<Option<TantivySingleMetricResult>>,
    doc_count: Option<u64>,
}

impl AggregationResultsRow {
    fn doc_count(&self) -> TantivyValue {
        match self.doc_count {
            Some(doc_count) => TantivyValue(OwnedValue::U64(doc_count)),
            None => TantivyValue(OwnedValue::Null),
        }
    }

    fn is_empty(&self) -> bool {
        self.group_keys.is_empty() && self.aggregates.is_empty() && self.doc_count.is_none()
    }
}

enum AggregationStyle {
    Ungrouped,
    Grouped,
    GroupedWithFilter,
}

impl AggregationResults {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn style(&self) -> AggregationStyle {
        if self.0.contains_key(FilterSentinelKey::NAME) {
            AggregationStyle::GroupedWithFilter
        } else if self.0.contains_key(GroupedKey::NAME) {
            AggregationStyle::Grouped
        } else {
            AggregationStyle::Ungrouped
        }
    }

    fn collect_group_keys(
        &self,
        key_accumulator: Vec<TantivyValue>,
        out: &mut Vec<AggregationResultsRow>,
    ) {
        // look only at the "grouped" terms bucket at this level
        if let Some(AggregationResult::BucketResult(BucketResult::Terms { buckets, .. })) =
            self.0.get(GroupedKey::NAME)
        {
            for bucket_entry in buckets {
                // extend the key path with this bucket's key
                let mut new_keys = key_accumulator.clone();
                let key_val = match &bucket_entry.key {
                    Key::Str(s) => TantivyValue(OwnedValue::Str(s.clone())),
                    Key::I64(i) => TantivyValue(OwnedValue::I64(*i)),
                    Key::U64(u) => TantivyValue(OwnedValue::U64(*u)),
                    Key::F64(f) => TantivyValue(OwnedValue::F64(*f)),
                };
                new_keys.push(key_val);

                // check if this bucket has a child "grouped" terms bucket
                let sub = AggregationResults(bucket_entry.sub_aggregation.0.clone());
                let has_child_grouped = match sub.0.get(GroupedKey::NAME) {
                    Some(AggregationResult::BucketResult(BucketResult::Terms {
                        buckets, ..
                    })) => !buckets.is_empty(),
                    _ => false,
                };

                if has_child_grouped {
                    // not a leaf yet; keep descending
                    sub.collect_group_keys(new_keys, out);
                } else {
                    // leaf: emit ONLY the deepest group path
                    out.push(AggregationResultsRow {
                        group_keys: new_keys,
                        aggregates: Vec::new(),
                        doc_count: Some(bucket_entry.doc_count),
                    });
                }
            }
        }
    }

    fn flatten_grouped(self, out: &mut Vec<AggregationResultsRow>) {
        if self.0.is_empty() {
            return;
        }

        // 1. collect all group key paths first
        self.collect_group_keys(Vec::new(), out);

        // 2. for each row, chase down aggregate values matching its group keys
        for row in out.iter_mut() {
            let mut current = self.0.clone();

            // traverse down into nested "grouped" buckets following group_keys
            for key in &row.group_keys {
                if let Some(AggregationResult::BucketResult(BucketResult::Terms {
                    buckets, ..
                })) = current.get(GroupedKey::NAME)
                {
                    // find the bucket whose key matches this level
                    let maybe_bucket = buckets.iter().find(|b| match (&b.key, &key.0) {
                        (Key::Str(s), OwnedValue::Str(v)) => s == v,
                        (Key::I64(i), OwnedValue::I64(v)) => i == v,
                        (Key::U64(i), OwnedValue::U64(v)) => i == v,
                        (Key::F64(i), OwnedValue::F64(v)) => i == v,
                        _ => false,
                    });

                    if let Some(bucket) = maybe_bucket {
                        // descend into this bucket’s sub-aggregations
                        current = bucket.sub_aggregation.0.clone();
                    } else {
                        // no matching bucket found — bail out early
                        current.clear();
                        break;
                    }
                }
            }

            // 3. collect any metric results at this nested level
            let mut entries: Vec<_> = current.into_iter().collect();
            entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(usize::MAX));

            for (_name, result) in entries {
                if let AggregationResult::MetricResult(metric) = result {
                    let single = match metric {
                        MetricResult::Average(r)
                        | MetricResult::Count(r)
                        | MetricResult::Sum(r)
                        | MetricResult::Min(r)
                        | MetricResult::Max(r) => r,
                        other => {
                            pgrx::warning!("unsupported metric type in flatten_into: {:?}", other);
                            continue;
                        }
                    };
                    row.aggregates.push(Some(single));
                }
            }
        }
    }

    fn flatten_ungrouped(self, out: &mut Vec<AggregationResultsRow>) {
        if self.0.is_empty() {
            return;
        }

        let mut aggregates = Vec::new();
        let mut entries: Vec<_> = self.0.into_iter().collect();
        entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(usize::MAX));

        for (_name, result) in entries {
            match result {
                AggregationResult::MetricResult(metric) => {
                    let single = match metric {
                        MetricResult::Average(r)
                        | MetricResult::Count(r)
                        | MetricResult::Sum(r)
                        | MetricResult::Min(r)
                        | MetricResult::Max(r) => r,
                        other => {
                            pgrx::warning!(
                                "unsupported metric type in flatten_ungrouped: {:?}",
                                other
                            );
                            continue;
                        }
                    };
                    aggregates.push(Some(single));
                }
                AggregationResult::BucketResult(BucketResult::Filter(filter_bucket)) => {
                    let mut sub_rows = Vec::new();
                    let sub = AggregationResults(filter_bucket.sub_aggregations.0);
                    sub.flatten_ungrouped(&mut sub_rows);
                    for sub_row in sub_rows {
                        aggregates.extend(sub_row.aggregates);
                    }
                }
                unsupported => todo!("unsupported bucket type: {:?}", unsupported),
            }
        }

        out.push(AggregationResultsRow {
            group_keys: Vec::new(),
            aggregates,
            doc_count: None,
        });
    }

    fn flatten_grouped_with_filter(self, out: &mut Vec<AggregationResultsRow>) {
        if self.0.is_empty() {
            return;
        }

        // Ensure stable sorting of results
        let mut filter_entries: Vec<_> = self
            .0
            .iter()
            .filter(|(k, _)| *k != FilterSentinelKey::NAME)
            .collect();
        filter_entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(usize::MAX));

        // Extract the sentinel filter bucket, used to get all the group keys
        let sentinel = match self.0.get(FilterSentinelKey::NAME) {
            Some(AggregationResult::BucketResult(BucketResult::Filter(filter_bucket))) => {
                filter_bucket
            }
            _ => {
                pgrx::warning!("missing filter_sentinel in flatten_grouped_with_filter");
                return;
            }
        };

        // Collect all group keys from the sentinel
        let sentinel_sub = AggregationResults(sentinel.sub_aggregations.0.clone());
        let mut rows = Vec::new();
        sentinel_sub.collect_group_keys(Vec::new(), &mut rows);

        // For each row of group keys, collect aggregates
        let num_filters = filter_entries.len();
        for row in &mut rows {
            let mut aggregates = Vec::new();

            for (_filter_name, filter_result) in &filter_entries {
                let mut found_metrics: Vec<Option<TantivySingleMetricResult>> = Vec::new();

                if let AggregationResult::BucketResult(BucketResult::Filter(filter_bucket)) =
                    filter_result
                {
                    let sub = AggregationResults(filter_bucket.sub_aggregations.0.clone());
                    let mut current = sub.0;

                    for key in &row.group_keys {
                        if let Some(AggregationResult::BucketResult(BucketResult::Terms {
                            buckets,
                            ..
                        })) = current.get(GroupedKey::NAME)
                        {
                            if let Some(bucket) = buckets.iter().find(|b| match (&b.key, &key.0) {
                                (Key::Str(s), OwnedValue::Str(v)) => s == v,
                                (Key::I64(i), OwnedValue::I64(v)) => i == v,
                                (Key::U64(i), OwnedValue::U64(v)) => i == v,
                                (Key::F64(i), OwnedValue::F64(v)) => i == v,
                                _ => false,
                            }) {
                                current = bucket.sub_aggregation.0.clone();
                            } else {
                                current.clear();
                                break;
                            }
                        }
                    }

                    for (_n, res) in current {
                        if let AggregationResult::MetricResult(metric) = res {
                            let single = match metric {
                                MetricResult::Average(r)
                                | MetricResult::Count(r)
                                | MetricResult::Sum(r)
                                | MetricResult::Min(r)
                                | MetricResult::Max(r) => r,
                                _ => continue,
                            };
                            found_metrics.push(Some(single));
                        }
                    }
                }

                // pad: if no metrics found for this filter, insert an empty placeholder
                if found_metrics.is_empty() {
                    aggregates.push(None);
                } else {
                    aggregates.extend(found_metrics);
                }
            }

            row.aggregates = aggregates;
        }

        out.extend(rows);
    }
}
