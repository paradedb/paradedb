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

//! Utilities for "pulling up" values from the index (i.e. using fast fields).
//!
//! This module provides shared logic for determining if and how a PostgreSQL
//! column can be resolved using Tantivy fast fields.

use crate::api::operator::row_expr_from_indexed_expr;
use crate::api::{CTID_FIELD_NAME, FieldName};
use crate::index::fast_fields_helper::{FieldCardinality, FieldDelivery, WhichFastField};
use crate::postgres::composite::get_composite_type_fields;
#[cfg(not(feature = "pg15"))]
use crate::postgres::node::NodeExt;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::strip_tokenizer_cast;
use crate::schema::{FieldSource, SearchField, SearchFieldType, SearchIndexSchema};
use pgrx::{PgList, PgTupleDesc, pg_sys};

/// what the caller wants from a single source
pub enum ResTarget<'a> {
    /// A heap column by attno
    Attno(pg_sys::AttrNumber),
    /// A Tantivy field by name: an alias, a JSON sub-field, an array, a pdb.agg field.
    Field(&'a str),
    /// An expression to match against the index expressions (planning time only).
    Expr(*mut pg_sys::Node),
    Ctid,
    Score,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedField {
    pub attno: Option<pg_sys::AttrNumber>, // none for expression-only fields,
    pub field: WhichFastField,
    pub is_json: bool,
}
impl ResolvedField {
    pub fn new(attno: Option<pg_sys::AttrNumber>, field: WhichFastField, is_json: bool) -> Self {
        Self {
            attno,
            field,
            is_json,
        }
    }
}

/// How the caller will use consume the value
#[derive(Copy, Clone)]
pub enum Access {
    Scalar,
    List,
    Any,
}
impl Access {
    fn is_unallowed_array(&self, is_array: bool) -> bool {
        match self {
            Self::List | Self::Any => false,
            Self::Scalar => is_array,
        }
    }
}

#[derive(Debug)]
pub struct SourceContext {
    pub heap_rti: pg_sys::Index,
    pub heaprel: PgSearchRelation,
    pub indexrel: Option<PgSearchRelation>,
    pub alias: Option<String>,
}
impl SourceContext {
    pub fn new(
        heap_rti: pg_sys::Index,
        heaprel: PgSearchRelation,
        indexrel: Option<PgSearchRelation>,
        alias: Option<String>,
    ) -> Self {
        Self {
            heap_rti,
            heaprel,
            indexrel,
            alias,
        }
    }

    pub fn tupdesc(&self) -> PgTupleDesc<'_> {
        self.heaprel.tuple_desc()
    }
    pub fn index_expressions(&self) -> Option<PgList<pg_sys::Expr>> {
        self.indexrel().map(|i| i.index_expressions())
    }

    pub fn indexrel(&self) -> Option<&PgSearchRelation> {
        self.indexrel.as_ref()
    }
    pub fn schema(&self) -> Option<SearchIndexSchema> {
        self.indexrel.as_ref().map(|i| {
            i.schema()
                .expect("a valid index should always have a schema")
        })
    }

    pub fn resolve_heap_field(&self, target: ResTarget) -> Option<(FieldName, SearchField)> {
        match target {
            ResTarget::Attno(attno) => {
                let name = self.get_attname(attno)?;
                let field = self.schema().and_then(|s| s.search_field(&name))?;
                Some((name, field))
            }
            ResTarget::Field(name) => {
                let field = self.schema().and_then(|s| s.search_field(name))?;
                Some((name.into(), field))
            }
            _ => unreachable!("only attno and field supported"),
        }
    }

    /// Resolves a ColumnRef to a Tantivy fast field, from this source
    pub fn resolve_fast_field(&self, col: ResTarget, access: Access) -> Option<ResolvedField> {
        //TODO:  This will swtich on columnref and just call resovlers. (once that works,
        //consolidate their code here
        match col {
            ResTarget::Attno(attno) => {
                let resolved = unsafe { resolve_fast_field(attno, self, access)? };
                if !field_type_supported_for_pullup(
                    resolved.field.field_type(),
                    access.is_unallowed_array(resolved.field.is_array()),
                ) {
                    return None;
                }
                Some(resolved)
            }
            ResTarget::Field(name) => {
                let resolved = resolve_fast_field_by_name(name, self.indexrel()?)?;
                let is_subfield = FieldName::from(name).root() != name;
                if !is_subfield
                    && !field_type_supported_for_pullup(
                        resolved.field.field_type(),
                        access.is_unallowed_array(resolved.field.is_array()),
                    )
                {
                    return None;
                }
                Some(resolved)
            }
            ResTarget::Expr(node) => {
                let (ff, is_json) = find_matching_fast_field(node, self)?;
                if !field_type_supported_for_pullup(
                    ff.field_type(),
                    access.is_unallowed_array(ff.is_array()),
                ) {
                    return None;
                }
                // TODO: Should be able to find an attno sometimes
                Some(ResolvedField::new(None, ff, is_json))
            }
            ResTarget::Ctid => Some(ResolvedField::new(
                Some(pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber),
                WhichFastField::Ctid,
                false,
            )),
            ResTarget::Score => Some(ResolvedField::new(None, WhichFastField::score(), false)),
        }
    }

    /// Look up a 1-based attribute number by column name in a tuple descriptor.
    pub fn get_attno_by_name(&self, name: &str) -> Option<pg_sys::AttrNumber> {
        for (i, att) in self.tupdesc().iter().enumerate() {
            if att.name() == name {
                return Some((i + 1) as pg_sys::AttrNumber);
            }
        }
        None
    }

    pub fn get_attname(&self, attno: pg_sys::AttrNumber) -> Option<FieldName> {
        if attno == 0 {
            return None;
        }
        if attno == pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber {
            Some(CTID_FIELD_NAME.into())
        } else {
            self.tupdesc()
                .get(attno as usize - 1)
                .map(|attribute| attribute.name().into())
        }
    }

    /// Get the column name for an attribute, with fallback to "relname.attno" if lookup fails.
    pub fn get_attname_safe(&self, attno: pg_sys::AttrNumber, rel_name: &str) -> String {
        self.get_attname(attno)
            .map(|f| format!("{rel_name}.{f}"))
            .unwrap_or(format!("{}.{}", rel_name, attno))
    }
}

fn fix_varno_in_place(node: *mut pg_sys::Node, old_varno: i32, new_varno: i32) {
    if node.is_null() {
        return;
    }
    for var in node.collect_nodes::<pg_sys::Var>() {
        let var = unsafe { &mut *var };
        if var.varno == old_varno {
            var.varno = new_varno as _;
        }
        if var.varnosyn as i32 == old_varno {
            var.varnosyn = new_varno as _;
        }
    }
}

fn find_matching_fast_field(
    node: *mut pg_sys::Node,
    context: &SourceContext,
) -> Option<(WhichFastField, bool)> {
    if node.is_null() {
        return None;
    }
    let Some(index) = &context.indexrel else {
        return None;
    };

    // In PostgreSQL 17+, `Var` nodes on the nullable side of an outer join carry
    // `varnullingrels` tracking outer-join relids. Base table index expressions
    // always have `varnullingrels == NULL`. Clone the candidate query expression
    // and clear `varnullingrels` on all its `Var`s so `pg_sys::equal` can match.
    let node_copy = unsafe { pg_sys::copyObjectImpl(node.cast()) }.cast::<pg_sys::Node>();
    #[cfg(not(feature = "pg15"))]
    for var in node_copy.collect_nodes::<pg_sys::Var>() {
        unsafe { (*var).varnullingrels = std::ptr::null_mut() };
    }

    let schema = index.schema().expect("should always have a valid schema");
    let categorized_fields = schema.categorized_fields();

    let matches_node = |candidate: *mut pg_sys::Node| {
        let unwrapped = strip_tokenizer_cast(candidate);
        let candidate_copy =
            unsafe { pg_sys::copyObjectImpl(unwrapped.cast()) }.cast::<pg_sys::Node>();
        fix_varno_in_place(candidate_copy, 1, context.heap_rti as i32);
        unsafe {
            pg_sys::equal(
                node_copy as *const core::ffi::c_void,
                candidate_copy as *const core::ffi::c_void,
            )
        }
    };

    let to_fast_field = |search_field: &SearchField, is_array: bool| -> Option<WhichFastField> {
        if search_field.is_fast() {
            return Some(WhichFastField::named(
                search_field.field_name().to_string(),
                search_field.field_type(),
                FieldCardinality::from_arrayness(is_array),
                FieldDelivery::Eager,
            ));
        }
        None
    };

    for (i, expr) in index.index_expressions().iter_ptr().enumerate() {
        if let Some(row_expr) = unsafe { row_expr_from_indexed_expr(expr) } {
            // ROW(...) composite: match each arg as if it were an independent expression.
            let composite_oid = unsafe { pg_sys::exprType(expr.cast()) };
            let Ok(fields) = (unsafe { get_composite_type_fields(composite_oid) }) else {
                continue;
            };

            let row_args = unsafe { PgList::<pg_sys::Node>::from_pg((*row_expr).args) };
            for (position, arg) in row_args.iter_ptr().enumerate() {
                if position >= fields.len() || fields[position].is_dropped {
                    continue;
                }

                let field_idx = fields[position].field_index;
                if let Some((search_field, data)) = categorized_fields.iter().find(|(_, data)| {
                    matches!(
                        data.source,
                        FieldSource::CompositeField {
                            expression_idx,
                            field_idx: idx,
                            ..
                        } if expression_idx == i && idx == field_idx
                    )
                }) && matches_node(arg)
                    && let Some(ff) = to_fast_field(search_field, data.is_array)
                {
                    return Some((ff, data.is_json));
                }
            }
        } else if let Some((search_field, data)) = categorized_fields.iter().find(
            |(_, data)| matches!(data.source, FieldSource::Expression { att_idx } if att_idx == i),
        ) && matches_node(expr as *mut pg_sys::Node)
            && let Some(ff) = to_fast_field(search_field, data.is_array)
        {
            return Some((ff, data.is_json));
        }
    }

    None
}

/// Resolves a PostgreSQL attribute number to a Tantivy fast field, if available.
///
/// This checks if the column corresponding to `attno` in the heap relation is available
/// as a fast field in the ParadeDB index. It handles:
///
/// - System columns (ctid, tableoid)
/// - Regular columns (mapped via name)
/// - Expression-indexed columns (e.g. typmod casts like `(col)::pdb.literal_normalized`)
/// - Type compatibility checks
/// - Field source verification (ensuring index field comes from the expected heap column)
///
/// Returns `Some(WhichFastField)` if the column can be pulled up from the index,
/// or `None` if it cannot (e.g. not indexed, not a fast field, incompatible type).
unsafe fn resolve_fast_field(
    attno: pg_sys::AttrNumber,
    context: &SourceContext,
    access: Access,
) -> Option<ResolvedField> {
    let Some(index) = &context.indexrel else {
        return None;
    };
    let tupdesc = context.tupdesc();
    match attno as i32 {
        // any of these mean we can't use fast fields
        pg_sys::MinTransactionIdAttributeNumber
        | pg_sys::MaxTransactionIdAttributeNumber
        | pg_sys::MinCommandIdAttributeNumber
        | pg_sys::MaxCommandIdAttributeNumber => None,

        // these aren't _exactly_ fast fields, but we do have the information
        // readily available during the scan, so we'll pretend
        pg_sys::SelfItemPointerAttributeNumber => {
            Some(ResolvedField::new(Some(attno), WhichFastField::Ctid, false))
        }

        pg_sys::TableOidAttributeNumber => Some(ResolvedField::new(
            Some(attno),
            WhichFastField::TableOid,
            false,
        )),

        _ => {
            if attno <= 0 {
                return None;
            }

            let att = tupdesc.get((attno - 1) as usize)?;

            let schema = index.schema().expect("should always have a valid schema");
            if let Some((search_field, data)) = schema.search_field_with_data(att.name()) {
                // For direct heap columns, the source attno must match.
                // Expression-indexed columns (FieldSource::Expression) are NOT
                // handled here — pulling up the raw column value from a transformed
                // expression (e.g. lower(name)) would return wrong data.
                // See: https://github.com/paradedb/paradedb/issues/3978
                //
                // Expression-indexed columns that are simple tokenizer casts
                // (e.g. (col)::pdb.literal_normalized) are handled by the
                // find_matching_fast_field fallback below instead.
                if matches!(data.source, FieldSource::Heap { attno: source_attno } if source_attno == (attno - 1) as usize)
                    && search_field.is_fast()
                {
                    return Some(ResolvedField::new(
                        Some(attno),
                        WhichFastField::named(
                            att.name(),
                            search_field.field_type(),
                            FieldCardinality::from_arrayness(data.is_array),
                            FieldDelivery::Eager,
                        ),
                        data.is_json,
                    ));
                }
            }

            // Fallback for expression-indexed columns (e.g. typmod casts like
            // `(col)::pdb.literal_normalized`). These have FieldSource::Expression
            // which the check above skips, but find_matching_fast_field handles
            // by stripping the tokenizer cast and matching the underlying Var.
            //
            // We use context.heap_rti as varno because find_matching_fast_field
            // compares via pg_sys::equal(), so both sides must use the same varno
            // to match.
            let dummy_var = pg_sys::makeVar(
                context.heap_rti.try_into().expect("should always fit"),
                attno as pg_sys::AttrNumber,
                att.atttypid,
                att.atttypmod,
                att.attcollation,
                0,
            );
            context.resolve_fast_field(ResTarget::Expr(dummy_var as *mut pg_sys::Node), access)
        }
    }
}

/// Resolve a fast field by Tantivy field name (e.g., `metadata.category`).
///
/// Used for JSON sub-fields where [`resolve_fast_field`] fails because the attno
/// maps to the parent JSON column rather than the sub-field. Tantivy stores JSON
/// sub-fields as separate fast fields with dotted names.
fn resolve_fast_field_by_name(field_name: &str, index: &PgSearchRelation) -> Option<ResolvedField> {
    let schema = index.schema().ok()?;
    let (search_field, data) = schema.search_field_with_data(field_name)?;
    if search_field.is_fast() {
        let heap_attno = data
            .source
            .heap_attno(index)
            .map(|n| n as pg_sys::AttrNumber + 1); // convert index-based to pg-based attno
        Some(ResolvedField::new(
            heap_attno,
            WhichFastField::named(
                field_name,
                search_field.field_type(),
                FieldCardinality::from_arrayness(data.is_array),
                FieldDelivery::Eager,
            ),
            data.is_json,
        ))
    } else {
        None
    }
}

/// Returns the `SearchFieldType` if it's supported for fast field pullup execution.
///
/// Returns `Some(SearchFieldType)` if the type is supported for fast field execution,
/// `None` otherwise (e.g. arrays, JSON which require special handling).
fn field_type_supported_for_pullup(field_type: Option<&SearchFieldType>, is_array: bool) -> bool {
    if is_array {
        return false;
    }
    match field_type {
        // The field did not have a field type (ctid, score, etc.), which is allowed for pullup
        None => true,
        // JSON/JSONB are excluded because fast fields do not contain the
        // full content of the JSON in a way that we can easily render:
        // rather, the individual fields are exploded out into dynamic columns.
        Some(SearchFieldType::Json(_))
        | Some(SearchFieldType::Tokenized(_, _, pg_sys::JSONOID | pg_sys::JSONBOID)) => false,
        // Range types are not yet supported for pullup
        Some(SearchFieldType::Range(_)) => false,
        // All other types can be pulled up directly
        _ => true,
    }
}
