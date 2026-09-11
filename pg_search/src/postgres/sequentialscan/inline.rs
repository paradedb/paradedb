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

use crate::api::operator::search_with_query_input_exec_procoids;
use crate::api::version::Version;
use crate::api::{HashMap, HashSet};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::index::writer::index::SerialIndexWriter;
use crate::postgres::composite::CompositeSlotValues;
use crate::postgres::heap::ExpressionState;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{resolve_field_value, row_to_search_document};
use crate::postgres::var::{find_one_var, find_var_relation, find_vars};
use crate::query::SearchQueryInput;
use crate::schema::{CategorizedFieldData, FieldSource, SearchField};
use pgrx::{IntoDatum, PgBox, PgList, PgTupleDesc, direct_function_call, pg_sys};
use std::ptr::NonNull;
use std::sync::OnceLock;
use tantivy::TantivyDocument;
use tantivy::directory::RamDirectory;
use tantivy::index::{SegmentId, SegmentReader};
use tantivy::query::Weight;

/// An expression that materializes a row if the index scan fast path cannot be taken.
///
/// This happens when either:
/// - the row comes from a subquery or CTE
/// - the row's CTID is invalid
/// - the row was created outside the search snapshot
/// - the index predicate is not satisfied
pub(crate) struct MaybeInlineRow(Option<NonNull<pg_sys::Node>>);

impl MaybeInlineRow {
    pub(crate) unsafe fn new(
        root: *mut pg_sys::PlannerInfo,
        base_var: *mut pg_sys::Var,
        ctid: Option<*mut pg_sys::Var>,
        indexrel: &PgSearchRelation,
    ) -> Self {
        // Building a whole-row reference requires a planner query and a Var that names an
        // entry in its range table; outer-query Vars and varno 0 cannot be resolved here.
        if root.is_null()
            || (*root).parse.is_null()
            || (*base_var).varlevelsup != 0
            || (*base_var).varno == 0
        {
            return Self(None);
        }

        let rtable = PgList::<pg_sys::RangeTblEntry>::from_pg((*(*root).parse).rtable);
        let Some(rte) = rtable.get_ptr((*base_var).varno as usize - 1) else {
            return Self(None);
        };

        let whole_row = pg_sys::makeWholeRowVar(rte, (*base_var).varno, 0, false);
        #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
        {
            (*whole_row).varnullingrels = pg_sys::bms_copy((*base_var).varnullingrels);
        }

        let row = if ctid.is_none() {
            let (heap_oid, _, targetlist) = find_var_relation(base_var, root);
            let targetlist = targetlist.expect("derived row should have a target list");
            let source = targetlist
                .get_ptr((*base_var).varattno as usize - 1)
                .and_then(|entry| find_one_var((*entry).expr.cast()));
            let mut fields = PgList::<pg_sys::Node>::new();
            let mut names = PgList::<pg_sys::Node>::new();
            let mut attributes: HashMap<_, Option<*mut pg_sys::TargetEntry>> = HashMap::default();
            for entry in targetlist.iter_ptr() {
                if (*entry).resjunk
                    || (*entry).resorigtbl != heap_oid
                    || (*entry).resorigcol <= 0
                    || source.is_some_and(|source| {
                        find_one_var((*entry).expr.cast()).is_none_or(|var| {
                            (*var).varno != (*source).varno
                                || (*var).varlevelsup != (*source).varlevelsup
                        })
                    })
                {
                    continue;
                }
                attributes
                    .entry((*entry).resorigcol)
                    .and_modify(|previous| {
                        if (*entry).resno == (*base_var).varattno {
                            *previous = Some(entry);
                        } else if previous.is_some_and(|previous| {
                            (*previous).resno != (*base_var).varattno
                                && !pg_sys::equal((*previous).expr.cast(), (*entry).expr.cast())
                        }) {
                            *previous = None;
                        }
                    })
                    .or_insert(Some(entry));
            }
            for entry in targetlist
                .iter_ptr()
                .filter(|entry| attributes.get(&(**entry).resorigcol) == Some(&Some(*entry)))
            {
                let var = pg_sys::copyObjectImpl(base_var.cast()).cast::<pg_sys::Var>();
                (*var).varattno = (*entry).resno;
                (*var).varattnosyn = (*entry).resno;
                (*var).vartype = pg_sys::exprType((*entry).expr.cast());
                (*var).vartypmod = pg_sys::exprTypmod((*entry).expr.cast());
                (*var).varcollid = pg_sys::exprCollation((*entry).expr.cast());
                fields.push(var.cast());
                names.push(
                    pg_sys::makeString(pg_sys::get_attname(heap_oid, (*entry).resorigcol, false))
                        .cast(),
                );
            }
            let mut row = PgBox::<pg_sys::RowExpr>::alloc_node(pg_sys::NodeTag::T_RowExpr);
            row.args = fields.into_pg();
            row.row_typeid = pg_sys::RECORDOID;
            row.row_format = pg_sys::CoercionForm::COERCE_EXPLICIT_CALL;
            row.colnames = names.into_pg();
            row.location = (*whole_row).location;

            let mut is_null = PgBox::<pg_sys::NullTest>::alloc_node(pg_sys::NodeTag::T_NullTest);
            is_null.arg = whole_row.cast();
            is_null.nulltesttype = pg_sys::NullTestType::IS_NULL;
            is_null.argisrow = false;
            let mut when = PgBox::<pg_sys::CaseWhen>::alloc_node(pg_sys::NodeTag::T_CaseWhen);
            when.expr = is_null.into_pg().cast();
            when.result = pg_sys::makeNullConst(pg_sys::RECORDOID, -1, pg_sys::Oid::INVALID).cast();
            let mut cases = PgList::<pg_sys::Node>::new();
            cases.push(when.into_pg().cast());
            let mut case = PgBox::<pg_sys::CaseExpr>::alloc_node(pg_sys::NodeTag::T_CaseExpr);
            case.casetype = pg_sys::RECORDOID;
            case.args = cases.into_pg();
            case.defresult = row.into_pg().cast();
            case.location = (*whole_row).location;
            case.into_pg().cast::<pg_sys::Node>()
        } else {
            whole_row.cast()
        };
        let row_type = pg_sys::exprType(row);
        let array_type = pg_sys::get_array_type(row_type);
        let mut rows = PgList::<pg_sys::Node>::new();
        rows.push(row);
        let mut inline_row = PgBox::<pg_sys::ArrayExpr>::alloc_node(pg_sys::NodeTag::T_ArrayExpr);
        inline_row.array_typeid = array_type;
        inline_row.element_typeid = row_type;
        inline_row.elements = rows.into_pg();
        inline_row.location = (*whole_row).location;

        // A shard has a different table row type from its coordinator relation.
        let inline_row = pg_sys::makeRelabelType(
            inline_row.into_pg().cast(),
            pg_sys::RECORDARRAYOID,
            -1,
            pg_sys::Oid::INVALID,
            pg_sys::CoercionForm::COERCE_EXPLICIT_CAST,
        );
        let Some(ctid) = ctid else {
            return Self(NonNull::new(inline_row.cast()));
        };

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

        let xmin = pg_sys::copyObjectImpl(ctid.cast()).cast::<pg_sys::Var>();
        (*xmin).varattno = pg_sys::MinTransactionIdAttributeNumber as _;
        (*xmin).varattnosyn = (*xmin).varattno;
        (*xmin).vartype = pg_sys::XIDOID;
        let mut xmin_args = PgList::<pg_sys::Node>::new();
        xmin_args.push(xmin.cast());
        let visible_xmin = pg_sys::makeFuncExpr(
            xmin_is_visible_procoid(),
            pg_sys::BOOLOID,
            xmin_args.into_pg(),
            pg_sys::Oid::INVALID,
            pg_sys::Oid::INVALID,
            pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
        );

        // Requirements are checked in order: prospective RLS rows have no valid CTID,
        // so we must take the fallback before attempting to read their xmin.
        let mut requirements = vec![valid_ctid.cast::<pg_sys::Expr>(), visible_xmin.cast()];
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
            requirements.push(pg_sys::make_ands_explicit(predicate));
        }

        let mut when_list = PgList::<pg_sys::Node>::new();
        for requirement in requirements {
            // Both FALSE and NULL mean the index cannot answer for this row.
            let mut failed =
                PgBox::<pg_sys::BooleanTest>::alloc_node(pg_sys::NodeTag::T_BooleanTest);
            failed.arg = requirement;
            failed.booltesttype = pg_sys::BoolTestType::IS_NOT_TRUE;
            failed.location = (*whole_row).location;

            let mut when = PgBox::<pg_sys::CaseWhen>::alloc_node(pg_sys::NodeTag::T_CaseWhen);
            when.expr = failed.into_pg().cast();
            when.result = pg_sys::copyObjectImpl(inline_row.cast()).cast();
            when.location = (*whole_row).location;
            when_list.push(when.into_pg().cast());
        }

        let mut case = PgBox::<pg_sys::CaseExpr>::alloc_node(pg_sys::NodeTag::T_CaseExpr);
        case.casetype = pg_sys::RECORDARRAYOID;
        case.casecollid = pg_sys::Oid::INVALID;
        case.args = when_list.into_pg();
        // Unlike an anonymous record constant ('()'::record), an empty array can be
        // deparsed and parsed by Citus workers. It stays non-NULL for strict helpers.
        case.defresult = pg_sys::makeConst(
            pg_sys::RECORDARRAYOID,
            -1,
            pg_sys::Oid::INVALID,
            -1,
            pg_sys::Datum::from(pg_sys::construct_empty_array(pg_sys::RECORDOID)),
            false,
            false,
        )
        .cast();
        case.location = (*whole_row).location;
        Self(NonNull::new(case.into_pg().cast()))
    }

    pub(crate) fn as_ptr(&self) -> Option<*mut pg_sys::Node> {
        self.0.map(|case| case.as_ptr().cast())
    }

    /// Returns the `pg_proc` OID of the search execution function to call.
    /// Selects a row-fallback variant when an inline row is available, and a strict
    /// variant when the anchor column is known to be NOT NULL.
    pub(crate) fn procoid(&self, anchor_is_not_null: bool) -> pg_sys::Oid {
        let [
            nullable_procoid,
            strict_procoid,
            row_procoid,
            strict_row_procoid,
        ] = search_with_query_input_exec_procoids();

        // Strictness lets PostgreSQL reduce outer joins. The empty-array marker
        // keeps the ordinary heap path callable even with a strict row argument.
        match (self.0.is_some(), anchor_is_not_null) {
            (true, true) => strict_row_procoid,
            (true, false) => row_procoid,
            (false, true) => strict_procoid,
            (false, false) => nullable_procoid,
        }
    }
}

/// Evaluates a row as a one-document search corpus.
pub(super) struct RowMatcher {
    index_relation: PgSearchRelation,
    slot: *mut pg_sys::TupleTableSlot,
    expression_state: ExpressionState,
    required_expressions: HashSet<usize>,
    categorized_fields: Vec<(SearchField, CategorizedFieldData)>,
    created_by_version: Option<Version>,
    weight: Box<dyn Weight>,
    field_exists_weight: Option<Box<dyn Weight>>,
}

impl RowMatcher {
    /// CurrentMemoryContext must outlive the returned matcher.
    pub(super) unsafe fn new(index_relation: PgSearchRelation, query: SearchQueryInput) -> Self {
        let heap_relation = index_relation
            .heap_relation()
            .expect("a ParadeDB index must have a heap relation");
        let schema = index_relation
            .schema()
            .expect("a ParadeDB index must have a schema");
        let null_guard = schema.null_guard(&query);
        let mut required_fields = HashSet::default();
        let fields_known = query.extract_field_names(&schema, &mut required_fields)
            && null_guard
                .as_ref()
                .is_none_or(|guard| guard.extract_field_names(&schema, &mut required_fields));
        let categorized_fields: Vec<_> = schema
            .categorized_fields()
            .iter()
            .filter(|(field, _)| {
                !fields_known || required_fields.contains(&field.field_name().root())
            })
            .cloned()
            .collect();
        let required_expressions: HashSet<_> = categorized_fields
            .iter()
            .filter_map(|(_, categorized)| match categorized.source {
                FieldSource::Heap { .. } => None,
                FieldSource::Expression { att_idx } => Some(att_idx),
                FieldSource::CompositeField { expression_idx, .. } => Some(expression_idx),
            })
            .collect();
        let reader =
            SearchIndexReader::open(&index_relation, query, false, MvccSatisfies::Snapshot)
                .expect("row matcher should open the ParadeDB index");
        let weight = reader.weight();
        let field_exists_weight = null_guard.map(|guard| {
            reader
                .compile_match_weight(&guard, false)
                .expect("row matcher exists query should be constructable")
        });
        let mut required_attributes: HashSet<_> = categorized_fields
            .iter()
            .filter_map(|(_, field)| match field.source {
                FieldSource::Heap { attno } => Some(attno),
                _ => None,
            })
            .collect();
        let expressions = index_relation.index_expressions();
        for expression in &required_expressions {
            for var in find_vars(expressions.get_ptr(*expression).unwrap().cast()) {
                if (*var).varattno == 0 {
                    required_attributes.extend(0..(*heap_relation.rd_att).natts as usize);
                } else if (*var).varattno > 0 {
                    required_attributes.insert((*var).varattno as usize - 1);
                }
            }
        }
        let tuple_desc = pg_sys::CreateTupleDescCopy(heap_relation.rd_att);
        for attribute in 0..(*tuple_desc).natts as usize {
            if !required_attributes.contains(&attribute) {
                #[cfg(not(feature = "pg18"))]
                {
                    (*tuple_desc)
                        .attrs
                        .as_mut_slice((*tuple_desc).natts as usize)[attribute]
                        .attisdropped = true;
                }
                #[cfg(feature = "pg18")]
                {
                    (*pg_sys::TupleDescAttr(tuple_desc, attribute as _)).attisdropped = true;
                    pg_sys::populate_compact_attribute(tuple_desc, attribute as _);
                }
            }
        }
        let slot = pg_sys::MakeSingleTupleTableSlot(tuple_desc, &pg_sys::TTSOpsVirtual);

        Self {
            expression_state: ExpressionState::new_in_context(
                &index_relation,
                &mut pgrx::PgMemoryContexts::CurrentMemoryContext,
            ),
            required_expressions,
            categorized_fields,
            created_by_version: index_relation.created_by_version(),
            weight,
            field_exists_weight,
            index_relation,
            slot,
        }
    }

    pub(super) unsafe fn matches(&mut self, row: pg_sys::Datum) -> Option<bool> {
        let header = pg_sys::pg_detoast_datum(row.cast_mut_ptr()).cast();
        let row_desc = PgTupleDesc::from_pg(pg_sys::lookup_rowtype_tupdesc(
            pgrx::heap_tuple_header_get_type_id(header),
            pgrx::heap_tuple_header_get_typmod(header),
        ));
        let conversion =
            pg_sys::convert_tuples_by_name(row_desc.as_ptr(), (*self.slot).tts_tupleDescriptor);
        if conversion.is_null() {
            pg_sys::ExecStoreHeapTupleDatum(header.into(), self.slot);
        } else {
            let tuple = pgrx::composite_row_type_make_tuple(header.into());
            let converted = pg_sys::execute_attr_map_tuple(tuple.as_ptr(), conversion);
            pg_sys::ExecForceStoreHeapTuple(converted, self.slot, true);
            pg_sys::free_conversion_map(conversion);
        }
        pg_sys::slot_getallattrs(self.slot);

        let natts = (*self.slot).tts_nvalid as usize;
        let values = std::slice::from_raw_parts((*self.slot).tts_values, natts);
        let isnull = std::slice::from_raw_parts((*self.slot).tts_isnull, natts);
        let expr_results = self.expression_state.evaluate_selected(self.slot, |index| {
            self.required_expressions.contains(&index)
        });
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
        )
        .unwrap_or_else(|error| panic!("failed to index row for inline evaluation: {error}"));
        pg_sys::ExecClearTuple(self.slot);
        if header != row.cast_mut_ptr() {
            pg_sys::pfree(header.cast());
        }

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

fn xmin_is_visible_procoid() -> pg_sys::Oid {
    static CACHE: OnceLock<pg_sys::Oid> = OnceLock::new();
    *CACHE.get_or_init(|| unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.xmin_is_visible(xid)".into_datum()],
        )
        .expect("the `paradedb.xmin_is_visible(xid)` function should exist")
    })
}
