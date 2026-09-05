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

use crate::api::version::Version;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::index::writer::index::SerialIndexWriter;
use crate::postgres::composite::CompositeSlotValues;
use crate::postgres::heap::ExpressionState;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{resolve_field_value, row_to_search_document};
use crate::query::SearchQueryInput;
use crate::schema::{CategorizedFieldData, FieldSource, SearchField};
use pgrx::{IntoDatum, PgBox, PgList, direct_function_call, pg_sys};
use std::sync::OnceLock;
use tantivy::TantivyDocument;
use tantivy::directory::RamDirectory;
use tantivy::index::{SegmentId, SegmentReader};
use tantivy::query::Weight;

/// An expression that materializes a row if the index scan fast path cannot be taken.
///
/// This happens when either:
/// - the row's CTID is invalid
/// - the index predicate is not satisfied
///
/// The expression is `CASE WHEN ctid_is_valid(ctid) AND index_predicate THEN empty_record ELSE table_row END`.
pub(crate) struct MaybeInlineRow(*mut pg_sys::CaseExpr);

impl MaybeInlineRow {
    pub(crate) unsafe fn new(
        root: *mut pg_sys::PlannerInfo,
        base_var: *mut pg_sys::Var,
        ctid: *mut pg_sys::Var,
        indexrel: &PgSearchRelation,
    ) -> Option<Self> {
        // Building a whole-row reference requires a planner query and a Var that names an
        // entry in its range table; outer-query Vars and varno 0 cannot be resolved here.
        if root.is_null()
            || (*root).parse.is_null()
            || (*base_var).varlevelsup != 0
            || (*base_var).varno == 0
        {
            return None;
        }

        let rtable = PgList::<pg_sys::RangeTblEntry>::from_pg((*(*root).parse).rtable);
        let rte = rtable.get_ptr((*base_var).varno as usize - 1)?;

        // The index can answer for an existing heap row that satisfies its predicate.
        let mut valid_args = PgList::<pg_sys::Node>::new();
        valid_args.push(pg_sys::copyObjectImpl(ctid.cast()).cast());
        let valid_ctid = pg_sys::makeFuncExpr(
            ctid_is_valid_procoid(),
            pg_sys::BOOLOID,
            valid_args.into_pg(),
            pg_sys::Oid::INVALID,
            pg_sys::Oid::INVALID,
            pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
        );

        let mut coverage_checks = PgList::<pg_sys::Node>::new();
        coverage_checks.push(valid_ctid.cast());
        let predicate = pg_sys::RelationGetIndexPredicate(indexrel.as_ptr());
        if !predicate.is_null() {
            pg_sys::ChangeVarNodes(predicate.cast(), 1, (*base_var).varno, 0);
            #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
            let predicate = pg_sys::add_nulling_relids(
                predicate.cast(),
                std::ptr::null_mut(),
                (*base_var).varnullingrels,
            )
            .cast();
            coverage_checks.push(pg_sys::make_ands_explicit(predicate).cast());
        }
        let index_covers_row = pg_sys::make_ands_explicit(coverage_checks.into_pg());

        // Build the fallback row reference, preserving outer-join null extension.
        let whole_row = pg_sys::makeWholeRowVar(rte, (*base_var).varno, 0, false);
        #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
        {
            (*whole_row).varnullingrels = pg_sys::bms_copy((*base_var).varnullingrels);
        }

        // A constant empty record keeps the fast path non-NULL, allowing strict search helpers
        // without materializing the table row. Real rows always have at least one indexed column.
        let mut empty_record = PgBox::<pg_sys::RowExpr>::alloc_node(pg_sys::NodeTag::T_RowExpr);
        empty_record.row_typeid = pg_sys::RECORDOID;
        empty_record.row_format = pg_sys::CoercionForm::COERCE_EXPLICIT_CALL;
        empty_record.location = -1;

        // CASE takes the fallback for both FALSE and NULL index predicates.
        let mut when_indexed = PgBox::<pg_sys::CaseWhen>::alloc_node(pg_sys::NodeTag::T_CaseWhen);
        when_indexed.expr = index_covers_row;
        when_indexed.result = pg_sys::evaluate_expr(
            empty_record.into_pg().cast(),
            pg_sys::RECORDOID,
            -1,
            pg_sys::Oid::INVALID,
        );
        when_indexed.location = (*whole_row).location;

        let mut when_list = PgList::<pg_sys::Node>::new();
        when_list.push(when_indexed.into_pg().cast());

        let mut case = PgBox::<pg_sys::CaseExpr>::alloc_node(pg_sys::NodeTag::T_CaseExpr);
        case.casetype = pg_sys::RECORDOID;
        case.casecollid = pg_sys::Oid::INVALID;
        case.args = when_list.into_pg();
        // Share the marker's RECORD type without rebuilding the named table row.
        case.defresult = pg_sys::makeRelabelType(
            whole_row.cast(),
            pg_sys::RECORDOID,
            -1,
            pg_sys::Oid::INVALID,
            pg_sys::CoercionForm::COERCE_IMPLICIT_CAST,
        )
        .cast();
        case.location = (*whole_row).location;
        Some(Self(case.into_pg()))
    }

    pub(crate) fn as_ptr(&self) -> *mut pg_sys::Node {
        self.0.cast()
    }
}

/// Evaluates a row as a one-document search corpus.
pub(super) struct RowMatcher {
    index_relation: PgSearchRelation,
    slot: *mut pg_sys::TupleTableSlot,
    expression_state: ExpressionState,
    categorized_fields: Vec<(SearchField, CategorizedFieldData)>,
    created_by_version: Option<Version>,
    weight: Box<dyn Weight>,
    field_exists_weight: Option<Box<dyn Weight>>,
}

impl RowMatcher {
    pub(super) fn new(index_relation: PgSearchRelation, query: SearchQueryInput) -> Self {
        let heap_relation = index_relation
            .heap_relation()
            .expect("a ParadeDB index must have a heap relation");
        let schema = index_relation
            .schema()
            .expect("a ParadeDB index must have a schema");
        let null_guard = schema.null_guard(&query);
        let reader =
            SearchIndexReader::open(&index_relation, query, false, MvccSatisfies::Snapshot)
                .expect("row matcher should open the ParadeDB index");
        let weight = reader.weight();
        let field_exists_weight = null_guard.map(|guard| {
            reader
                .compile_match_weight(&guard, false)
                .expect("row matcher exists query should be constructable")
        });
        let slot = unsafe {
            pgrx::PgMemoryContexts::TopTransactionContext.switch_to(|_| {
                pg_sys::MakeSingleTupleTableSlot(heap_relation.rd_att, &pg_sys::TTSOpsVirtual)
            })
        };

        Self {
            expression_state: ExpressionState::new(&index_relation),
            categorized_fields: schema.categorized_fields().clone(),
            created_by_version: index_relation.created_by_version(),
            weight,
            field_exists_weight,
            index_relation,
            slot,
        }
    }

    pub(super) unsafe fn matches(&mut self, row: pg_sys::Datum) -> Option<bool> {
        pg_sys::ExecStoreHeapTupleDatum(row, self.slot);
        pg_sys::slot_getallattrs(self.slot);

        let natts = (*self.slot).tts_nvalid as usize;
        let values = std::slice::from_raw_parts((*self.slot).tts_values, natts);
        let isnull = std::slice::from_raw_parts((*self.slot).tts_isnull, natts);
        let expr_results = self.expression_state.evaluate(self.slot);
        let unpacked_composites =
            CompositeSlotValues::from_composites(self.categorized_fields.iter().filter_map(
                |(_, categorized)| {
                    if let FieldSource::CompositeField {
                        expression_idx,
                        composite_type_oid,
                        ..
                    } = categorized.source
                    {
                        let (datum, is_null) = expr_results[expression_idx];
                        Some((expression_idx, datum, is_null, composite_type_oid))
                    } else {
                        None
                    }
                },
            ));

        let mut document = TantivyDocument::new();
        row_to_search_document(
            self.categorized_fields.iter().map(|(field, categorized)| {
                let (datum, is_null) = resolve_field_value(
                    &categorized.source,
                    values,
                    isnull,
                    &expr_results,
                    &unpacked_composites,
                );
                (datum, is_null, field, categorized)
            }),
            &mut document,
            self.created_by_version,
        );
        pg_sys::ExecClearTuple(self.slot);

        // Tantivy queries execute against segment readers, so expose the row as a temporary segment.
        let mut writer = SerialIndexWriter::in_memory(
            &self.index_relation,
            SegmentId::generate_random(),
            RamDirectory::create(),
            0,
        )
        .expect("row matcher should create an in-memory index");
        writer
            .insert(document, 1, || {})
            .expect("row matcher should index one row");
        let segment_meta = writer
            .finalize_nocommit()
            .expect("row matcher should finalize its in-memory index")
            .expect("row matcher always indexes one row");
        let segment_reader = SegmentReader::open(&writer.index.segment(segment_meta))
            .expect("row matcher should open its in-memory segment");

        if self
            .weight
            .count(&segment_reader)
            .expect("inline row query should execute")
            > 0
        {
            Some(true)
        } else if self.field_exists_weight.as_ref().is_some_and(|weight| {
            weight
                .count(&segment_reader)
                .expect("row matcher exists query should execute")
                == 0
        }) {
            None
        } else {
            Some(false)
        }
    }
}

crate::impl_safe_drop!(RowMatcher, |self| {
    unsafe {
        if crate::postgres::utils::IsTransactionState() {
            pg_sys::ExecDropSingleTupleTableSlot(self.slot);
        }
    }
});

fn ctid_is_valid_procoid() -> pg_sys::Oid {
    static CACHE: OnceLock<pg_sys::Oid> = OnceLock::new();
    *CACHE.get_or_init(|| unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.ctid_is_valid(tid)".into_datum()],
        )
        .expect("the `paradedb.ctid_is_valid(tid)` function should exist")
    })
}
