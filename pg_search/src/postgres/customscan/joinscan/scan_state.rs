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

//! Execution state and plan building for JoinScan custom scan.
//!
//! This module defines the runtime state of the custom scan, including the DataFusion
//! execution context. It also contains the logic for building the DataFusion logical plan
//! from the serialized planning data.

use std::sync::Arc;

use arrow_array::UInt64Array;
use datafusion::common::{Column, DataFusionError, JoinType, Result};
use datafusion::logical_expr::{col, Expr};
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties};
use datafusion::prelude::SessionContext;
use pgrx::{pg_sys, PgList};

use crate::api::{OrderByFeature, SortDirection};
use crate::index::fast_fields_helper::{FFHelper, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::joinscan::build::{JoinCSClause, JoinLevelSearchPredicate};
use crate::postgres::customscan::joinscan::privdat::{
    OutputColumnInfo, PrivateData, INNER_SCORE_ALIAS, OUTER_SCORE_ALIAS,
};
use crate::postgres::customscan::joinscan::translator::{
    make_col, CombinedMapper, PredicateTranslator,
};
use crate::postgres::customscan::CustomScanState;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::heap::VisibilityChecker as HeapVisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::{PgSearchTableProvider, Scanner};

/// The execution state for the JoinScan.
#[derive(Default)]
pub struct JoinScanState {
    /// The join clause from planning.
    pub join_clause: JoinCSClause,

    // === Driving side state (side with search predicate - we iterate through this) ===
    /// The heap relation for the driving side.
    pub driving_heaprel: Option<PgSearchRelation>,
    /// Visibility checker for the driving side.
    pub driving_visibility_checker: Option<VisibilityChecker>,
    /// Slot for fetching driving side tuples.
    pub driving_fetch_slot: Option<*mut pg_sys::TupleTableSlot>,

    // === Build side state ===
    /// The heap relation for the build side.
    pub build_heaprel: Option<PgSearchRelation>,
    /// Visibility checker for the build side.
    pub build_visibility_checker: Option<VisibilityChecker>,
    /// Slot for fetching build side tuples by ctid.
    pub build_scan_slot: Option<*mut pg_sys::TupleTableSlot>,

    // === Side tracking ===
    /// Whether the driving side is the outer side (true) or inner side (false).
    pub driving_is_outer: bool,

    // === Result state ===
    /// Result tuple slot.
    pub result_slot: Option<*mut pg_sys::TupleTableSlot>,

    // === DataFusion State ===
    pub datafusion_stream: Option<datafusion::execution::SendableRecordBatchStream>,
    pub runtime: Option<tokio::runtime::Runtime>,
    pub current_batch: Option<arrow_array::RecordBatch>,
    pub batch_index: usize,

    // === Result processing state ===
    /// Current driving side ctid from the DataFusion result batch.
    pub current_driving_ctid: Option<u64>,

    // === Output column mapping ===
    /// Mapping of output column positions to their source (outer/inner) and original attribute numbers.
    /// Populated from PrivateData during create_custom_scan_state.
    pub output_columns: Vec<OutputColumnInfo>,

    /// Index of the outer score column in the DataFusion batch, if any.
    pub outer_score_col_idx: Option<usize>,
    /// Index of the inner score column in the DataFusion batch, if any.
    pub inner_score_col_idx: Option<usize>,

    // === Memory tracking ===
    /// Maximum allowed memory for execution (from work_mem, in bytes).
    pub max_memory: usize,
}

impl JoinScanState {
    /// Reset the scan state for a rescan.
    pub fn reset(&mut self) {
        self.datafusion_stream = None;
        self.current_batch = None;
        self.batch_index = 0;
    }

    /// Returns (outer_slot, inner_slot) based on which side is driving.
    ///
    /// This maps the driving/build slots to outer/inner positions:
    /// - If driving_is_outer: driving_slot=outer, build_slot=inner
    /// - If driving_is_inner: driving_slot=inner, build_slot=outer
    pub fn outer_inner_slots(
        &self,
    ) -> (
        Option<*mut pg_sys::TupleTableSlot>,
        Option<*mut pg_sys::TupleTableSlot>,
    ) {
        if self.driving_is_outer {
            (self.driving_fetch_slot, self.build_scan_slot)
        } else {
            (self.build_scan_slot, self.driving_fetch_slot)
        }
    }

    /// Get the appropriate score for an output column.
    ///
    /// This determines whether to use the driving side score or the build side score
    /// based on which side the column references:
    /// - If `col_is_outer == driving_is_outer`: column references driving side → use driving_score
    /// - Otherwise: column references build side → use build_score
    pub fn score_for_column(&self, col_is_outer: bool, row_idx: usize) -> f32 {
        let score_idx = if col_is_outer {
            self.outer_score_col_idx
        } else {
            self.inner_score_col_idx
        };

        if let Some(idx) = score_idx {
            if let Some(batch) = &self.current_batch {
                let score_col = batch.column(idx);
                let score_array = score_col
                    .as_any()
                    .downcast_ref::<arrow_array::Float32Array>()
                    .expect("Score column should be Float32Array");
                return score_array.value(row_idx);
            }
        }

        0.0
    }
}

impl CustomScanState for JoinScanState {
    fn init_exec_method(&mut self, _cstate: *mut pg_sys::CustomScanState) {
        // No special initialization needed for the plain exec method
    }
}

/// Build the DataFusion execution plan for the join.
pub async fn build_joinscan_logical_plan(
    join_clause: &JoinCSClause,
    private_data: &PrivateData,
    custom_exprs: *mut pg_sys::List,
) -> Result<Arc<dyn ExecutionPlan>> {
    let outer_rti = join_clause.outer_side.heap_rti.unwrap_or(0);
    let inner_rti = join_clause.inner_side.heap_rti.unwrap_or(0);

    let outer_alias = join_clause
        .outer_side
        .alias
        .as_deref()
        .expect("outer alias should be set");
    let inner_alias = join_clause
        .inner_side
        .alias
        .as_deref()
        .expect("inner alias should be set");

    // 1. Create SessionContext and register tables
    let config = datafusion::prelude::SessionConfig::new();
    let ctx = SessionContext::new_with_config(config);

    let outer_fields: Vec<WhichFastField> = join_clause
        .outer_side
        .fields
        .iter()
        .map(|f| f.field.clone())
        .collect();
    let inner_fields: Vec<WhichFastField> = join_clause
        .inner_side
        .fields
        .iter()
        .map(|f| f.field.clone())
        .collect();

    // Use current snapshot for execution
    let outer_provider = Arc::new(PgSearchTableProvider::new(
        join_clause.outer_side.clone(),
        outer_fields.clone(),
    ));
    let inner_provider = Arc::new(PgSearchTableProvider::new(
        join_clause.inner_side.clone(),
        inner_fields.clone(),
    ));

    ctx.register_table(outer_alias, outer_provider)?;
    ctx.register_table(inner_alias, inner_provider)?;

    // 2. Build Logical Plan
    let mut outer_df = ctx.table(outer_alias).await?;
    if join_clause.outer_side.score_needed {
        let columns: Vec<Expr> = outer_df
            .schema()
            .fields()
            .iter()
            .zip(outer_fields.iter())
            .map(|(df_field, field_type)| match field_type {
                WhichFastField::Score => {
                    Expr::Column(Column::from_name(df_field.name())).alias(OUTER_SCORE_ALIAS)
                }
                _ => Expr::Column(Column::from_name(df_field.name())),
            })
            .collect();
        outer_df = outer_df.select(columns)?.alias(outer_alias)?;
    }

    let mut inner_df = ctx.table(inner_alias).await?;
    if join_clause.inner_side.score_needed {
        let columns: Vec<Expr> = inner_df
            .schema()
            .fields()
            .iter()
            .zip(inner_fields.iter())
            .map(|(df_field, field_type)| match field_type {
                WhichFastField::Score => {
                    Expr::Column(Column::from_name(df_field.name())).alias(INNER_SCORE_ALIAS)
                }
                _ => Expr::Column(Column::from_name(df_field.name())),
            })
            .collect();
        inner_df = inner_df.select(columns)?.alias(inner_alias)?;
    }

    // Prepare join keys
    let mut on: Vec<Expr> = Vec::new();

    for jk in &join_clause.join_keys {
        let left_name =
            CombinedMapper::get_field_name(&join_clause.outer_side, jk.outer_attno).unwrap();
        let right_name =
            CombinedMapper::get_field_name(&join_clause.inner_side, jk.inner_attno).unwrap();

        let left_col = make_col(outer_alias, &left_name);
        let right_col = make_col(inner_alias, &right_name);

        on.push(left_col.eq(right_col));
    }

    let mut df = outer_df.join_on(inner_df, JoinType::Inner, on)?;

    // 3. Apply Filter if needed
    if let Some(ref join_level_expr) = join_clause.join_level_expr {
        let mapper = CombinedMapper {
            outer: &join_clause.outer_side,
            inner: &join_clause.inner_side,
            output_columns: &private_data.output_columns,
            outer_alias,
            inner_alias,
        };

        let translator = PredicateTranslator::new(
            &join_clause.outer_side,
            &join_clause.inner_side,
            outer_rti,
            inner_rti,
        )
        .with_mapper(Box::new(mapper));

        // Translate all custom_exprs first
        let mut translated_exprs = Vec::new();
        unsafe {
            let expr_list = PgList::<pg_sys::Node>::from_pg(custom_exprs);
            for expr_node in expr_list.iter_ptr() {
                let expr = translator.translate(expr_node).ok_or_else(|| {
                    DataFusionError::Internal("Failed to translate expression".into())
                })?;
                translated_exprs.push(expr);
            }
        }

        // Execute join-level predicates to get matching sets
        let mut join_level_sets = Vec::with_capacity(join_clause.join_level_predicates.len());
        for pred in &join_clause.join_level_predicates {
            let set = unsafe { compute_predicate_matches(pred)? };
            join_level_sets.push(Arc::new(set));
        }

        // Construct ctid column expressions (qualified by table alias)
        let outer_ctid_col = make_col(outer_alias, &WhichFastField::Ctid.name());
        let inner_ctid_col = make_col(inner_alias, &WhichFastField::Ctid.name());

        // Now translate the JoinLevelExpr tree using the translated leaves
        let filter_expr = unsafe {
            PredicateTranslator::translate_join_level_expr(
                join_level_expr,
                &translated_exprs,
                &join_level_sets,
                &outer_ctid_col,
                &inner_ctid_col,
            )
        }
        .ok_or_else(|| {
            DataFusionError::Internal("Failed to translate join level expression tree".into())
        })?;

        df = df.filter(filter_expr)?;
    }

    // 4. Apply Sort
    if !join_clause.order_by.is_empty() {
        let mut sort_exprs = Vec::new();
        for info in &join_clause.order_by {
            let expr = match &info.feature {
                OrderByFeature::Score => {
                    if join_clause.driving_side_is_outer() {
                        Expr::Column(Column::from_name(OUTER_SCORE_ALIAS))
                    } else {
                        Expr::Column(Column::from_name(INNER_SCORE_ALIAS))
                    }
                }
                OrderByFeature::Field(name) => col(name.as_ref()),
            };

            let asc = matches!(
                info.direction,
                SortDirection::AscNullsFirst | SortDirection::AscNullsLast
            );
            let nulls_first = matches!(
                info.direction,
                SortDirection::AscNullsFirst | SortDirection::DescNullsFirst
            );
            sort_exprs.push(expr.sort(asc, nulls_first));
        }
        df = df.sort(sort_exprs)?;
    }

    // 5. Apply Limit
    if let Some(limit) = join_clause.limit {
        df = df.limit(0, Some(limit))?;
    }

    // 6. Create Physical Plan
    let plan = df.create_physical_plan().await?;

    // Ensure we have a single output partition
    if plan.output_partitioning().partition_count() > 1 {
        Ok::<_, DataFusionError>(
            Arc::new(CoalescePartitionsExec::new(plan)) as Arc<dyn ExecutionPlan>
        )
    } else {
        Ok(plan)
    }
}

/// Execute a join-level search predicate (Tantivy query) and return the matching CTIDs.
unsafe fn compute_predicate_matches(pred: &JoinLevelSearchPredicate) -> Result<Vec<u64>> {
    let index_rel = PgSearchRelation::open(pred.indexrelid);
    let heap_rel = PgSearchRelation::open(pred.heaprelid);

    let reader = SearchIndexReader::open_with_context(
        &index_rel,
        pred.query.clone(),
        false,
        MvccSatisfies::Snapshot,
        None,
        None,
    )
    .map_err(|e| DataFusionError::Internal(format!("Failed to open reader: {e}")))?;

    let search_results = reader.search();
    let fields = vec![WhichFastField::Ctid];
    let ffhelper = FFHelper::with_fields(&reader, &fields);
    let snapshot = pg_sys::GetActiveSnapshot();
    let mut visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

    let mut scanner = Scanner::new(search_results, Some(4096), fields, pred.heaprelid.into());

    let mut ctids = Vec::new();
    while let Some(batch) = scanner.next(&ffhelper, &mut visibility) {
        if let Some(Some(col)) = batch.fields.first() {
            let array = col
                .as_any()
                .downcast_ref::<UInt64Array>()
                .expect("Ctid should be UInt64Array");
            ctids.extend(array.values());
        }
    }

    ctids.sort_unstable();
    ctids.dedup();
    Ok(ctids)
}
