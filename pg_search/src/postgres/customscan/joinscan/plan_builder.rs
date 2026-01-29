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

use std::sync::Arc;

use arrow_array::UInt64Array;
use datafusion_common::NullEquality;
use datafusion_common::{DataFusionError, JoinType, Result};
use datafusion_physical_expr::expressions::Column;
use datafusion_physical_expr::PhysicalExpr;
use datafusion_physical_plan::filter::FilterExec;
use datafusion_physical_plan::joins::{HashJoinExec, PartitionMode};
use datafusion_physical_plan::ExecutionPlan;
use pgrx::{pg_sys, PgList};

use crate::index::fast_fields_helper::{FFHelper, FastFieldType, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{Bm25Params, SearchIndexReader};
use crate::postgres::customscan::joinscan::build::{
    JoinCSClause, JoinLevelSearchPredicate, JoinSideInfo,
};
use crate::postgres::customscan::joinscan::privdat::{OutputColumnInfo, PrivateData};
use crate::postgres::customscan::joinscan::translator::{ColumnMapper, PredicateTranslator};
use crate::postgres::heap::VisibilityChecker as HeapVisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::expr_collect_vars;
use crate::query::SearchQueryInput;
use crate::scan::datafusion_plan::ScanPlan;
use crate::scan::Scanner;

pub struct JoinScanPlanBuilder;

/// Helper struct to track which fields need to be extracted from one side of the join
/// for use in the DataFusion plan (join keys, filters, scores).
struct SideSchema {
    rti: pg_sys::Index,
    fields: Vec<WhichFastField>,
    // Mapping from varattno to index in fields
    attno_to_index: std::collections::HashMap<pg_sys::AttrNumber, usize>,
}

impl SideSchema {
    fn new(rti: pg_sys::Index) -> Self {
        Self {
            rti,
            fields: vec![WhichFastField::Ctid],
            attno_to_index: std::collections::HashMap::new(),
        }
    }

    fn add_field(&mut self, attno: pg_sys::AttrNumber, side: &JoinSideInfo) {
        if !self.attno_to_index.contains_key(&attno) {
            let field = unsafe { get_fast_field(side, attno) };
            self.attno_to_index.insert(attno, self.fields.len());
            self.fields.push(field);
        }
    }

    fn add_score(&mut self) {
        if !self.fields.contains(&WhichFastField::Score) {
            self.fields.push(WhichFastField::Score);
        }
    }
}

/// Helper to map PostgreSQL variables to DataFusion column indices across both sides
/// of the join. Used during predicate translation.
///
/// `output_columns` is used to resolve `INDEX_VAR` references. These variables
/// point to the output columns of the custom scan itself, and must be mapped
/// back to the original source relation (outer or inner) and its attribute
/// to find the correct column in the DataFusion plan.
struct CombinedMapper<'a> {
    outer: SideSchema,
    inner: SideSchema,
    outer_len: usize,
    output_columns: &'a [OutputColumnInfo],
}

impl<'a> ColumnMapper for CombinedMapper<'a> {
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<usize> {
        // Handle INDEX_VAR references which point to the custom scan output columns
        if varno == pg_sys::INDEX_VAR as pg_sys::Index {
            let idx = (varattno - 1) as usize;
            if idx >= self.output_columns.len() {
                return None;
            }
            let info = &self.output_columns[idx];

            if info.is_outer {
                if info.is_score {
                    // Find score field in outer schema
                    return self
                        .outer
                        .fields
                        .iter()
                        .position(|f| matches!(f, WhichFastField::Score));
                } else {
                    return self.outer.attno_to_index.get(&info.original_attno).copied();
                }
            } else {
                // Inner side
                let inner_idx = if info.is_score {
                    self.inner
                        .fields
                        .iter()
                        .position(|f| matches!(f, WhichFastField::Score))
                } else {
                    self.inner.attno_to_index.get(&info.original_attno).copied()
                };
                return inner_idx.map(|i| i + self.outer_len);
            }
        }

        if varno == self.outer.rti {
            self.outer.attno_to_index.get(&varattno).copied()
        } else if varno == self.inner.rti {
            self.inner
                .attno_to_index
                .get(&varattno)
                .map(|i| i + self.outer_len)
        } else {
            None
        }
    }
}

impl JoinScanPlanBuilder {
    /// Build the DataFusion execution plan for the join.
    ///
    /// The resulting plan is a `HashJoinExec` (optionally wrapped in `FilterExec`).
    /// The output schema of the plan preserves the structure expected by `JoinScan`:
    /// - Outer side columns, starting with `ctid` (u64)
    /// - Inner side columns, starting with `ctid` (u64)
    pub unsafe fn build(
        join_clause: &JoinCSClause,
        private_data: &PrivateData,
        custom_exprs: *mut pg_sys::List,
        snapshot: pg_sys::Snapshot,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let outer_rti = join_clause.outer_side.heap_rti.unwrap_or(0);
        let inner_rti = join_clause.inner_side.heap_rti.unwrap_or(0);

        let mut outer_schema = SideSchema::new(outer_rti);
        let mut inner_schema = SideSchema::new(inner_rti);

        // 1. Collect fields for join keys
        for jk in &join_clause.join_keys {
            outer_schema.add_field(jk.outer_attno, &join_clause.outer_side);
            inner_schema.add_field(jk.inner_attno, &join_clause.inner_side);
        }

        // 2. Collect fields for filters
        let expr_list = PgList::<pg_sys::Node>::from_pg(custom_exprs);
        for expr_node in expr_list.iter_ptr() {
            let vars = expr_collect_vars(expr_node, true);
            for var in vars {
                if var.rti == pg_sys::INDEX_VAR as pg_sys::Index {
                    // Resolve INDEX_VAR to original table column
                    let idx = (var.attno - 1) as usize;
                    if let Some(info) = private_data.output_columns.get(idx) {
                        if info.original_attno > 0 {
                            if info.is_outer {
                                outer_schema
                                    .add_field(info.original_attno, &join_clause.outer_side);
                            } else {
                                inner_schema
                                    .add_field(info.original_attno, &join_clause.inner_side);
                            }
                        }
                    }
                } else if var.rti == outer_rti {
                    outer_schema.add_field(var.attno, &join_clause.outer_side);
                } else if var.rti == inner_rti {
                    inner_schema.add_field(var.attno, &join_clause.inner_side);
                }
            }
        }

        // 3. Add score fields if needed
        if join_clause.outer_side.score_needed {
            outer_schema.add_score();
        }
        if join_clause.inner_side.score_needed {
            inner_schema.add_score();
        }

        // 4. Build ScanPlans
        let outer_plan = build_side_plan(&join_clause.outer_side, &outer_schema.fields, snapshot)?;
        let inner_plan = build_side_plan(&join_clause.inner_side, &inner_schema.fields, snapshot)?;

        // 5. Construct HashJoinExec
        let mut on: Vec<(Arc<dyn PhysicalExpr>, Arc<dyn PhysicalExpr>)> = Vec::new();
        for jk in &join_clause.join_keys {
            let left_idx = *outer_schema.attno_to_index.get(&jk.outer_attno).unwrap();
            let right_idx = *inner_schema.attno_to_index.get(&jk.inner_attno).unwrap();

            let outer_schema_ref = outer_plan.schema();
            let left_field = outer_schema_ref.field(left_idx);
            let inner_schema_ref = inner_plan.schema();
            let right_field = inner_schema_ref.field(right_idx);

            let left_name = left_field.name().clone();
            let right_name = right_field.name().clone();

            on.push((
                Arc::new(Column::new(&left_name, left_idx)),
                Arc::new(Column::new(&right_name, right_idx)),
            ));
        }

        let hash_join = HashJoinExec::try_new(
            outer_plan,
            inner_plan,
            on,
            None,
            &JoinType::Inner,
            None, // projection
            PartitionMode::CollectLeft,
            NullEquality::NullEqualsNull,
        )?;

        let mut plan: Arc<dyn ExecutionPlan> = Arc::new(hash_join);

        // 6. Apply FilterExec if needed
        if let Some(ref join_level_expr) = join_clause.join_level_expr {
            let outer_len = plan.children()[0].schema().fields().len();
            let mapper = CombinedMapper {
                outer: outer_schema,
                inner: inner_schema,
                outer_len,
                output_columns: &private_data.output_columns,
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
            for expr_node in expr_list.iter_ptr() {
                let physical_expr = translator.translate(expr_node).ok_or_else(|| {
                    DataFusionError::Internal("Failed to translate expression".into())
                })?;
                translated_exprs.push(physical_expr);
            }

            // Execute join-level predicates to get matching sets
            let mut join_level_sets = Vec::with_capacity(join_clause.join_level_predicates.len());
            for pred in &join_clause.join_level_predicates {
                let set = compute_predicate_matches(pred, snapshot)?;
                join_level_sets.push(Arc::new(set));
            }

            // Construct ctid column expressions
            let outer_ctid_col: Arc<dyn PhysicalExpr> = Arc::new(Column::new("ctid", 0));
            // Inner ctid is the first column of the inner plan, which is appended after outer columns
            let inner_ctid_col: Arc<dyn PhysicalExpr> = Arc::new(Column::new("ctid", outer_len));

            // Now translate the JoinLevelExpr tree using the translated leaves
            let filter_expr = PredicateTranslator::translate_join_level_expr(
                join_level_expr,
                &translated_exprs,
                &join_level_sets,
                &outer_ctid_col,
                &inner_ctid_col,
            )
            .ok_or_else(|| {
                DataFusionError::Internal("Failed to translate join level expression tree".into())
            })?;

            plan = Arc::new(FilterExec::try_new(filter_expr, plan)?);
        }

        Ok(plan)
    }
}

/// Execute a join-level search predicate (Tantivy query) and return the matching CTIDs.
///
/// These CTIDs are used to implement the bitmap-scan-like filtering in `JoinScan`,
/// where we intersect the search results with the join results.
unsafe fn compute_predicate_matches(
    pred: &JoinLevelSearchPredicate,
    snapshot: pg_sys::Snapshot,
) -> Result<Vec<u64>> {
    let index_rel = PgSearchRelation::open(pred.indexrelid);
    let heap_rel = PgSearchRelation::open(pred.heaprelid);

    let reader = SearchIndexReader::open_with_context(
        &index_rel,
        pred.query.clone(),
        MvccSatisfies::Snapshot,
        None,
        None,
        None, // No scoring needed
    )
    .map_err(|e| DataFusionError::Internal(format!("Failed to open reader: {e}")))?;

    let search_results = reader.search();
    let fields = vec![WhichFastField::Ctid];
    let mut ffhelper = FFHelper::with_fields(&reader, &fields);
    let mut visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

    let mut scanner = Scanner::new(search_results, Some(4096), fields, pred.heaprelid.into());

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

/// Create a DataFusion `ScanPlan` for one side of the join.
///
/// This plan scans the BM25 index and extracts the requested fast fields (including CTIDs).
unsafe fn build_side_plan(
    side: &JoinSideInfo,
    fields: &[WhichFastField],
    snapshot: pg_sys::Snapshot,
) -> Result<Arc<dyn ExecutionPlan>> {
    let heap_relid = side
        .heaprelid
        .ok_or_else(|| DataFusionError::Internal("Missing heaprelid".into()))?;
    let index_relid = side
        .indexrelid
        .ok_or_else(|| DataFusionError::Internal("Missing indexrelid".into()))?;

    let heap_rel = PgSearchRelation::open(heap_relid);
    let index_rel = PgSearchRelation::open(index_relid);

    let query = side.query.clone().unwrap_or(SearchQueryInput::All);

    let reader = SearchIndexReader::open_with_context(
        &index_rel,
        query,
        MvccSatisfies::Snapshot,
        None,
        None,
        // Use default BM25 params if scoring needed, None otherwise
        if side.score_needed {
            Some(Bm25Params::default())
        } else {
            None
        },
    )
    .map_err(|e| DataFusionError::Internal(format!("Failed to open reader: {e}")))?;

    let search_results = reader.search();
    let ffhelper = FFHelper::with_fields(&reader, fields);

    let visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

    let scanner = Scanner::new(
        search_results,
        Some(100), // batch size
        fields.to_vec(),
        heap_relid.into(),
    );

    Ok(Arc::new(ScanPlan::new(
        scanner,
        ffhelper,
        Box::new(visibility),
    )))
}

unsafe fn get_fast_field(side: &JoinSideInfo, attno: pg_sys::AttrNumber) -> WhichFastField {
    let heaprelid = side.heaprelid.unwrap();
    let heaprel = PgSearchRelation::open(heaprelid);
    let tupdesc = heaprel.tuple_desc();
    let att = tupdesc.get((attno - 1) as usize).unwrap();
    let att_name = att.name();

    let indexrelid = side.indexrelid.unwrap();
    let indexrel = PgSearchRelation::open(indexrelid);
    let schema = indexrel.schema().unwrap();

    if let Some(search_field) = schema.search_field(att_name) {
        WhichFastField::Named(
            att_name.to_string(),
            FastFieldType::from(search_field.field_type()),
        )
    } else {
        WhichFastField::Named(att_name.to_string(), FastFieldType::Int64)
    }
}
