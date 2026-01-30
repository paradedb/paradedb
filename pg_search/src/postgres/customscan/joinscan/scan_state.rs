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
use datafusion::common::{DataFusionError, JoinType, Result};
use datafusion::logical_expr::{col, Expr};
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties};
use datafusion::prelude::{DataFrame, SessionContext};
use futures::future::{FutureExt, LocalBoxFuture};
use pgrx::pg_sys;

use crate::api::{OrderByFeature, SortDirection};
use crate::index::fast_fields_helper::{FFHelper, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::joinscan::build::{
    JoinCSClause, JoinLevelSearchPredicate, JoinSource,
};

use crate::postgres::customscan::joinscan::privdat::{
    OutputColumnInfo, PrivateData, INNER_SCORE_ALIAS, OUTER_SCORE_ALIAS,
};
use crate::postgres::customscan::joinscan::translator::{make_col, CombinedMapper};
use crate::postgres::customscan::CustomScanState;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::heap::VisibilityChecker as HeapVisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::{PgSearchTableProvider, Scanner};

/// Execution state for a single base relation in a join.
pub struct RelationState {
    pub _heaprel: PgSearchRelation,
    pub visibility_checker: VisibilityChecker,
    pub fetch_slot: *mut pg_sys::TupleTableSlot,
    /// Index of the CTID column for this relation in the result RecordBatch.
    pub ctid_col_idx: Option<usize>,
}

/// The execution state for the JoinScan.
#[derive(Default)]
pub struct JoinScanState {
    /// The join clause from planning.
    pub join_clause: JoinCSClause,

    /// Map of range table index (RTI) to relation execution state.
    pub relations: crate::api::HashMap<pg_sys::Index, RelationState>,

    // === Result state ===
    /// Result tuple slot.
    pub result_slot: Option<*mut pg_sys::TupleTableSlot>,

    // === DataFusion State ===
    pub datafusion_stream: Option<datafusion::execution::SendableRecordBatchStream>,
    pub runtime: Option<tokio::runtime::Runtime>,
    pub current_batch: Option<arrow_array::RecordBatch>,
    pub batch_index: usize,

    // === Output column mapping ===
    /// Mapping of output column positions to their source (outer/inner) and original attribute numbers.
    /// Populated from PrivateData during create_custom_scan_state.
    pub output_columns: Vec<OutputColumnInfo>,

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
    let ctx = SessionContext::new();
    let df = build_clause_df(&ctx, join_clause, private_data, custom_exprs).await?;

    let plan = df.create_physical_plan().await?;

    if plan.output_partitioning().partition_count() > 1 {
        Ok::<_, DataFusionError>(
            Arc::new(CoalescePartitionsExec::new(plan)) as Arc<dyn ExecutionPlan>
        )
    } else {
        Ok(plan)
    }
}

/// Recursively builds a DataFusion `DataFrame` for a given join clause.
///
/// This function constructs the logical plan for a join by:
/// 1. Building DataFrames for the left (outer) and right (inner) sources.
/// 2. Performing an inner join on the specified equi-join keys.
/// 3. Applying join-level filters (both search predicates and heap conditions).
/// 4. Applying sorting and limits if specified.
/// 5. Projecting the final output columns as defined by the join's output projection.
fn build_clause_df<'a>(
    ctx: &'a SessionContext,
    join_clause: &'a JoinCSClause,
    private_data: &'a PrivateData,
    custom_exprs: *mut pg_sys::List,
) -> LocalBoxFuture<'a, Result<DataFrame>> {
    let f = async move {
        if join_clause.sources.len() < 2 {
            return Err(DataFusionError::Internal(
                "JoinScan requires at least 2 sources".into(),
            ));
        }

        let outer_source = &join_clause.sources[0];
        let inner_source = &join_clause.sources[1];

        let outer_df = build_source_df(ctx, outer_source, 0).await?;
        let inner_df = build_source_df(ctx, inner_source, 1).await?;

        // Prepare join keys
        let mut on: Vec<Expr> = Vec::new();
        let outer_alias_owned = outer_source.alias().unwrap_or_else(|| "outer".to_string());
        let inner_alias_owned = inner_source.alias().unwrap_or_else(|| "inner".to_string());
        let outer_alias = outer_alias_owned.as_str();
        let inner_alias = inner_alias_owned.as_str();

        let outer_df = outer_df.alias(outer_alias)?;
        let inner_df = inner_df.alias(inner_alias)?;

        for jk in &join_clause.join_keys {
            let left_name = match outer_source {
                JoinSource::Base(info) => {
                    let res = outer_source.column_name(jk.outer_attno);
                    if res.is_none() {
                        pgrx::warning!("Failed to resolve outer field name for attno {} in base {:?}. Available fields: {:?}", jk.outer_attno, info.alias, info.fields.iter().map(|f| f.attno).collect::<Vec<_>>());
                    }
                    res.ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "Failed to resolve outer join key field name for attno {} in source with alias '{}' and info {:?}",
                            jk.outer_attno,
                            outer_alias,
                            info
                        ))
                    })?
                }
                JoinSource::Join(..) => format!("col_{}", jk.outer_attno),
            };
            let right_name = match inner_source {
                JoinSource::Base(info) => {
                    let res = inner_source.column_name(jk.inner_attno);
                    if res.is_none() {
                        pgrx::warning!("Failed to resolve inner field name for attno {} in base {:?}. Available fields: {:?}", jk.inner_attno, info.alias, info.fields.iter().map(|f| f.attno).collect::<Vec<_>>());
                    }
                    res.ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "Failed to resolve inner join key field name for attno {} in source with alias '{}' and info {:?}",
                            jk.inner_attno,
                            inner_alias,
                            info
                        ))
                    })?
                }
                JoinSource::Join(..) => format!("col_{}", jk.inner_attno),
            };

            let left_col = make_col(outer_alias, &left_name);
            let right_col = make_col(inner_alias, &right_name);

            on.push(left_col.eq(right_col));
        }

        let mut df = outer_df.join_on(inner_df, JoinType::Inner, on)?;

        // 3. Apply Filter
        if let Some(ref join_level_expr) = join_clause.join_level_expr {
            let mapper = CombinedMapper {
                sources: &join_clause.sources,
                output_columns: &private_data.output_columns,
            };

            let translator =
                crate::postgres::customscan::joinscan::translator::PredicateTranslator::new(
                    &join_clause.sources,
                )
                .with_mapper(Box::new(mapper));

            // Translate all custom_exprs first
            let mut translated_exprs = Vec::new();
            unsafe {
                use pgrx::PgList;
                let expr_list = PgList::<pg_sys::Node>::from_pg(custom_exprs);
                for (i, expr_node) in expr_list.iter_ptr().enumerate() {
                    let expr = translator.translate(expr_node).ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "Failed to translate custom expression at index {}",
                            i
                        ))
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

            // Find the CTID column name for each source.
            let mut source_ctid_cols = Vec::new();
            for (i, source) in join_clause.sources.iter().enumerate() {
                let alias = source.alias().unwrap_or_else(|| format!("source_{}", i));
                let ctid_name = if let JoinSource::Base(info) = source {
                    format!("ctid_{}", info.heap_rti.unwrap_or(0))
                } else {
                    WhichFastField::Ctid.name().to_string()
                };
                source_ctid_cols.push(make_col(&alias, &ctid_name));
            }

            let filter_expr = unsafe {
                crate::postgres::customscan::joinscan::translator::PredicateTranslator::translate_join_level_expr(
                    join_level_expr,
                    &translated_exprs,
                    &join_level_sets,
                    &source_ctid_cols,
                )
            }
            .ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "Failed to translate join level expression tree: {:?}",
                    join_level_expr
                ))
            })?;

            df = df.filter(filter_expr)?;
        }

        // 4. Apply Sort
        if !join_clause.order_by.is_empty() {
            let mut sort_exprs = Vec::new();
            for info in &join_clause.order_by {
                let expr = match &info.feature {
                    OrderByFeature::Score => {
                        let ordering_is_outer = join_clause.ordering_side_is_outer();
                        let source_idx = if ordering_is_outer { 0 } else { 1 };
                        let source = &join_clause.sources[source_idx];
                        let alias = source.alias().unwrap_or_else(|| {
                            if source_idx == 0 {
                                "outer".to_string()
                            } else {
                                "inner".to_string()
                            }
                        });

                        let ordering_rti = source.ordering_rti().unwrap_or(0);
                        if let Some(col_idx) = source.map_var(ordering_rti, 0) {
                            if let Some(field_name) = source.column_name(col_idx) {
                                make_col(&alias, &field_name)
                            } else {
                                let score_alias = if ordering_is_outer {
                                    OUTER_SCORE_ALIAS
                                } else {
                                    INNER_SCORE_ALIAS
                                };
                                make_col(&alias, score_alias)
                            }
                        } else {
                            let score_alias = if ordering_is_outer {
                                OUTER_SCORE_ALIAS
                            } else {
                                INNER_SCORE_ALIAS
                            };
                            make_col(&alias, score_alias)
                        }
                    }
                    OrderByFeature::Field(name) => col(name.as_ref()),
                    OrderByFeature::Var {
                        rti,
                        attno,
                        name: _,
                    } => {
                        // Resolve RTI/Attno to column expression
                        let mut resolved_expr = None;
                        for (i, source) in join_clause.sources.iter().enumerate() {
                            if let Some(mapped_attno) = source.map_var(*rti, *attno) {
                                if let Some(field_name) = source.column_name(mapped_attno) {
                                    let alias = source.alias().unwrap_or_else(|| {
                                        if i == 0 {
                                            "outer".to_string()
                                        } else {
                                            "inner".to_string()
                                        }
                                    });
                                    resolved_expr = Some(make_col(&alias, &field_name));
                                    break;
                                }
                            }
                        }
                        resolved_expr.unwrap_or_else(|| col("unknown_col"))
                    }
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

        // 6. Apply Output Projection
        let mut final_cols = Vec::new();

        if let Some(projection) = &join_clause.output_projection {
            for (i, proj) in projection.iter().enumerate() {
                let col_alias = format!("col_{}", i + 1);
                let expr = build_projection_expr(proj, join_clause);
                final_cols.push(expr.alias(col_alias));
            }

            // ALWAYS carry forward all CTID columns from both sides
            let mut base_relations = Vec::new();
            join_clause.collect_base_relations(&mut base_relations);
            for base in base_relations {
                if let Some(rti) = base.heap_rti {
                    let ctid_name = format!("ctid_{}", rti);
                    // Check if it already exists in df schema (it should)
                    if df.schema().field_with_unqualified_name(&ctid_name).is_ok() {
                        // Carry it.
                        final_cols.push(col(&ctid_name));
                    }
                }
            }
        } else {
            for field in df.schema().fields() {
                final_cols.push(col(field.name()));
            }
        }

        df = df.select(final_cols)?;

        Ok(df)
    };
    f.boxed_local()
}

/// Builds a DataFusion projection expression for a given child projection info.
///
/// This maps a `ChildProjection` (referencing an RTI and attribute number) to a DataFusion
/// column expression, taking into account aliases and special columns like scores.
fn build_projection_expr(
    proj: &crate::postgres::customscan::joinscan::build::ChildProjection,
    join_clause: &JoinCSClause,
) -> Expr {
    for (i, source) in join_clause.sources.iter().enumerate() {
        let alias = source.alias().unwrap_or_else(|| {
            if i == 0 {
                "outer".to_string()
            } else {
                "inner".to_string()
            }
        });

        if proj.is_score {
            if let Some(attno) = source.map_var(proj.rti, 0) {
                if let Some(name) = source.column_name(attno) {
                    return make_col(&alias, &name);
                } else {
                    let score_alias = if i == 0 {
                        OUTER_SCORE_ALIAS
                    } else {
                        INNER_SCORE_ALIAS
                    };
                    return make_col(&alias, score_alias);
                }
            } else if source.contains_rti(proj.rti) {
                let score_alias = if i == 0 {
                    OUTER_SCORE_ALIAS
                } else {
                    INNER_SCORE_ALIAS
                };
                return make_col(&alias, score_alias);
            }
        } else if let Some(attno) = source.map_var(proj.rti, proj.attno) {
            if let Some(field_name) = source.column_name(attno) {
                return make_col(&alias, &field_name);
            }
        }
    }
    datafusion::logical_expr::lit(datafusion::common::ScalarValue::Null)
}

/// Builds a DataFusion `DataFrame` for a given join source.
///
/// If the source is a base relation, it registers a `PgSearchTableProvider` and
/// selects the required fields, aliasing CTID and Score columns as needed.
/// If the source is another join, it recursively calls `build_clause_df`.
fn build_source_df<'a>(
    ctx: &'a SessionContext,
    source: &'a JoinSource,
    source_idx: usize,
) -> LocalBoxFuture<'a, Result<DataFrame>> {
    async move {
        match source {
            JoinSource::Base(scan_info) => {
                let alias = scan_info.alias.as_deref().unwrap_or("base");
                let fields: Vec<WhichFastField> =
                    scan_info.fields.iter().map(|f| f.field.clone()).collect();
                let provider = Arc::new(PgSearchTableProvider::new(
                    scan_info.clone(),
                    fields.clone(),
                ));
                ctx.register_table(alias, provider)?;

                let mut df = ctx.table(alias).await?;

                // Select fields AND ensure CTID is aliased uniquely
                let mut exprs = Vec::new();
                for (df_field, field_type) in df.schema().fields().iter().zip(fields.iter()) {
                    let expr = match field_type {
                        WhichFastField::Ctid => {
                            let rti = scan_info.heap_rti.unwrap_or(0);
                            make_col(alias, df_field.name()).alias(format!("ctid_{}", rti))
                        }
                        WhichFastField::Score => {
                            let score_alias = if source_idx == 0 {
                                OUTER_SCORE_ALIAS
                            } else {
                                INNER_SCORE_ALIAS
                            };
                            make_col(alias, df_field.name()).alias(score_alias)
                        }
                        _ => make_col(alias, df_field.name()),
                    };
                    exprs.push(expr);
                }
                df = df.select(exprs)?;

                Ok(df)
            }
            JoinSource::Join(clause, _, _) => {
                build_clause_df(
                    ctx,
                    clause,
                    &PrivateData::new(clause.clone()),
                    std::ptr::null_mut(),
                )
                .await
            }
        }
    }
    .boxed_local()
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
    let mut ffhelper = FFHelper::with_fields(&reader, &fields);
    let snapshot = pg_sys::GetActiveSnapshot();
    let mut visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

    let mut scanner = Scanner::new(search_results, None, fields, pred.heaprelid.into());

    let mut ctids = Vec::new();
    while let Some(batch) = scanner.next(&mut ffhelper, &mut visibility) {
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
