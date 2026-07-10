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

use crate::api::operator::row_expr_from_indexed_expr;
use crate::api::tokenizers::definitions::pdb::DatumWithType;
use crate::api::tokenizers::{
    type_can_be_tokenized, type_is_alias, type_is_tokenizer, AliasTypmod, UncheckedTypmod,
};
use crate::api::version::Version;
use crate::api::{FieldName, HashMap};
use crate::index::writer::index::IndexError;
use crate::nodecast;
use crate::postgres::composite::{
    get_composite_fields_for_index, is_composite_type, CompositeSlotValues,
};
use crate::postgres::customscan::orderby::text_lower_funcoid;
use crate::postgres::deparse::deparse_expr;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::TantivyValue;
use crate::postgres::var::find_vars;
use crate::schema::{CategorizedFieldData, SearchField, SearchFieldType};
use anyhow::Result;
use pgrx::itemptr::{item_pointer_get_both, item_pointer_set_all};
use pgrx::*;
use rustc_hash::FxHashMap;
use std::collections::{BTreeMap, HashSet};

use std::ptr::addr_of_mut;
use std::str::FromStr;
use tokenizers::SearchNormalizer;

use super::datetime::PostgresDateTime;

extern "C-unwind" {
    // SAFETY: `IsTransactionState()` doesn't raise an ERROR.  As such, we can avoid the pgrx
    // sigsetjmp overhead by linking to the function directly.
    pub fn IsTransactionState() -> bool;
}

/// Implements Drop that skips cleanup during panic unwinding.
///
/// Because panics are used to propagate PostgreSQL errors via pgrx, it is almost never
/// safe to interact with PostgreSQL APIs during Drop - doing so can cause a double-panic
/// which results in SIGABRT. This macro ensures cleanup is skipped during unwinding.
///
/// PostgreSQL's transaction abort mechanism will clean up resources (buffers, relations, etc.)
/// when the transaction is aborted due to the error.
///
/// # Example
/// ```ignore
/// impl_safe_drop!(MyStruct, |self| {
///     unsafe {
///         if crate::postgres::utils::IsTransactionState() {
///             pg_sys::some_cleanup_function(self.handle);
///         }
///     }
/// });
/// ```
#[macro_export]
macro_rules! impl_safe_drop {
    ($ty:ty, |$self:ident| $body:block) => {
        impl Drop for $ty {
            fn drop(&mut $self) {
                if std::thread::panicking() {
                    return;
                }
                $body
            }
        }
    };
}

/// RAII guard for PostgreSQL standalone expression context
/// Automatically frees the context when dropped
///
/// let context_guard = ExprContextGuard::new();
/// // Use context_guard.as_ptr() to get the raw pointer
/// // Context is automatically freed when context_guard goes out of scope
#[derive(Debug)]
pub struct ExprContextGuard(*mut pg_sys::ExprContext);

// SAFETY: PostgreSQL doesn't execute within threads, despite Tantivy expecting it.
// The ExprContextGuard is used in Tantivy queries that require Send+Sync, but in practice
// these are never actually shared across threads in our PostgreSQL context.
unsafe impl Send for ExprContextGuard {}
unsafe impl Sync for ExprContextGuard {}

impl ExprContextGuard {
    /// Creates a new standalone expression context
    pub fn new() -> Self {
        unsafe { Self(pg_sys::CreateStandaloneExprContext()) }
    }

    /// Returns the raw pointer to the expression context
    pub fn as_ptr(&self) -> *mut pg_sys::ExprContext {
        self.0
    }
}

impl_safe_drop!(ExprContextGuard, |self| {
    unsafe {
        let is_commit = pg_sys::IsTransactionState();
        pg_sys::FreeExprContext(self.0, is_commit);
    }
});

impl Default for ExprContextGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively sort all object keys in JSON for deterministic output
pub fn sort_json_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            // Collect entries, sort by key, and rebuild
            let sorted: BTreeMap<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| {
                    let mut v = v.clone();
                    sort_json_keys(&mut v);
                    (k.clone(), v)
                })
                .collect();
            *map = sorted.into_iter().collect();
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                sort_json_keys(item);
            }
        }
        _ => {}
    }
}

/// TransactionIdPrecedesOrEquals --- is id1 logically <= id2?
///
/// Ported to rust from the Postgres sources to avoid the pgrx FFI overhead
#[allow(non_snake_case)]
#[inline]
pub fn TransactionIdPrecedesOrEquals(
    id1: pg_sys::TransactionId,
    id2: pg_sys::TransactionId,
) -> bool {
    //
    // #define TransactionIdIsNormal(xid)		((xid) >= FirstNormalTransactionId)
    //
    // /*
    //  * TransactionIdPrecedesOrEquals --- is id1 logically <= id2?
    //  */
    // bool
    // TransactionIdPrecedesOrEquals(TransactionId id1, TransactionId id2)
    // {
    //     /*
    //      * If either ID is a permanent XID then we can just do unsigned
    //      * comparison.  If both are normal, do a modulo-2^32 comparison.
    //      */
    //     int32		diff;
    //
    //     if (!TransactionIdIsNormal(id1) || !TransactionIdIsNormal(id2))
    //         return (id1 <= id2);
    //
    //     diff = (int32) (id1 - id2);
    //     return (diff <= 0);
    // }

    #[inline]
    fn is_transaction_id_normal(xid: pg_sys::TransactionId) -> bool {
        xid >= pg_sys::FirstNormalTransactionId
    }

    if !is_transaction_id_normal(id1) || !is_transaction_id_normal(id2) {
        return id1 <= id2;
    }

    // Compare as i32 to handle wraparound
    unsafe {
        // SAFETY: `pg_sysTransactionId` is a #[repr(transparent)] wrapper around a `u32`
        let id1: i32 = std::mem::transmute(id1);
        let id2: i32 = std::mem::transmute(id2);
        let diff = id1.wrapping_sub(id2);
        diff <= 0
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use crate::postgres::utils::TransactionIdPrecedesOrEquals;
    use pgrx::{pg_sys, pg_test};

    #[pg_test]
    fn xid_precedes_or_equals_test() {
        use proptest::prelude::*;

        proptest!(|(xid1 in 0..u32::MAX, xid2 in 0..u32::MAX)| {
            let us = TransactionIdPrecedesOrEquals(pg_sys::TransactionId::from(xid1), pg_sys::TransactionId::from(xid2));
            let pg = unsafe { pg_sys::TransactionIdPrecedesOrEquals(pg_sys::TransactionId::from(xid1), pg_sys::TransactionId::from(xid2)) };
            prop_assert_eq!(us, pg);
        });
    }
}

/// Rather than using pgrx' version of this function, we use our own, which doesn't leave 2
/// empty bytes in the middle of the 64bit representation.  A ctid being only 48bits means
/// if we leave the upper 16 bits (2 bytes) empty, tantivy will have a better chance of
/// bitpacking or compressing these values.
#[allow(dead_code)]
#[inline(always)]
pub fn item_pointer_to_u64(ctid: pg_sys::ItemPointerData) -> u64 {
    let (blockno, offno) = item_pointer_get_both(ctid);
    let blockno = blockno as u64;
    let offno = offno as u64;

    // shift the BlockNumber left 16 bits -- the length of the OffsetNumber we OR onto the end
    // pgrx's version shifts left 32, which is wasteful
    (blockno << 16) | offno
}

/// Rather than using pgrx' version of this function, we use our own, which doesn't leave 2
/// empty bytes in the middle of the 64bit representation.  A ctid being only 48bits means
/// if we leave the upper 16 bits (2 bytes) empty, tantivy will have a better chance of
/// bitpacking or compressing these values.
#[inline(always)]
pub fn u64_to_item_pointer(value: u64, tid: &mut pg_sys::ItemPointerData) {
    // shift right 16 bits to pop off the OffsetNumber, leaving only the BlockNumber
    // pgrx's version must shift right 32 bits to be in parity with `item_pointer_to_u64()`
    let blockno = (value >> 16) as pg_sys::BlockNumber;
    let offno = value as pg_sys::OffsetNumber;
    item_pointer_set_all(tid, blockno, offno);
}

/// Returns `true` if the block referenced by `ctid` (u64-packed form) exists
/// in `rel`. A `false` result means VACUUM has truncated the page.
#[inline(always)]
pub unsafe fn ctid_satisfies_nblocks(
    ctid: u64,
    rel: pg_sys::Relation,
    fork: pg_sys::ForkNumber::Type,
) -> bool {
    let blockno = (ctid >> 16) as pg_sys::BlockNumber;
    blockno < pg_sys::RelationGetNumberOfBlocksInFork(rel, fork)
}

#[derive(Copy, Clone, Debug)]
pub enum FieldSource {
    /// Direct column from heap tuple
    Heap { attno: usize },

    /// Expression index (scalar result)
    Expression { att_idx: usize },

    /// Field extracted from a composite type
    CompositeField {
        /// Index attribute number (slot in values[] array for aminsert/build_callback)
        index_attno: usize,

        /// Expression index (position in expr_results[] array for MVCC build)
        expression_idx: usize,

        /// Field position within the composite (0-indexed)
        field_idx: usize,

        /// OID of the named composite type
        composite_type_oid: pg_sys::Oid,
    },
}

/// Collect composite slot info from categorized fields for upfront unpacking.
///
/// Returns a lazy iterator of (slot_index, datum, is_null, type_oid) for each unique
/// composite slot in the categorized fields.
///
/// # Safety
/// - `values` and `isnull` pointers are dereferenced lazily during iteration,
///   not at call time. Caller must ensure these pointers remain valid until
///   the returned iterator is fully consumed.
/// - The iterator should be consumed immediately (e.g., via `from_composites()`).
///   Storing the iterator and consuming it later risks UB if the underlying
///   slot arrays have been invalidated.
pub unsafe fn collect_composites_for_unpacking<'a>(
    categorized_fields: impl Iterator<Item = &'a CategorizedFieldData> + 'a,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
) -> impl Iterator<Item = (usize, pg_sys::Datum, bool, pg_sys::Oid)> + 'a {
    let mut seen = rustc_hash::FxHashSet::default();

    categorized_fields.filter_map(move |cat| {
        if let FieldSource::CompositeField {
            index_attno,
            composite_type_oid,
            ..
        } = cat.source
        {
            if seen.insert(index_attno) {
                let datum = *values.add(index_attno);
                let is_null = *isnull.add(index_attno);
                return Some((index_attno, datum, is_null, composite_type_oid));
            }
        }
        None
    })
}

/// Helper to extract field value from values[] array, handling composite fields.
///
/// **Works for**: aminsert (INSERT) and build_callback (CREATE INDEX) paths.
/// **Does NOT work for**: MVCC build (which uses expr_results[] instead).
///
/// # Arguments
/// * `source` - The field source (Heap, Expression, or CompositeField)
/// * `index_attno` - Index attribute position (slot in values[] array)
/// * `values` - Array of datums from PostgreSQL
/// * `isnull` - Array of null flags from PostgreSQL
/// * `unpacked_composites` - Pre-unpacked composite values
///
/// # Returns
/// Tuple of (datum, is_null) for the field
///
/// # Safety
/// Caller must ensure values and isnull pointers are valid.
pub unsafe fn get_field_value(
    source: &FieldSource,
    index_attno: usize,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    unpacked_composites: &CompositeSlotValues,
) -> (pg_sys::Datum, bool) {
    match source {
        FieldSource::Heap { .. } | FieldSource::Expression { .. } => {
            (*values.add(index_attno), *isnull.add(index_attno))
        }
        FieldSource::CompositeField {
            index_attno: comp_index_attno,
            field_idx,
            ..
        } => unpacked_composites.get(*comp_index_attno, *field_idx),
    }
}

/// Represents the metadata extracted from an index attribute
#[derive(Debug)]
pub struct ExtractedFieldAttribute {
    /// its ordinal position in the index attribute list
    pub attno: usize,

    /// its source in the heap: either a heap attno, or an expression index
    pub source: FieldSource,

    /// its original Postgres type OID
    pub pg_type: PgOid,

    /// the type we'll use for indexing in tantivy
    pub tantivy_type: SearchFieldType,

    pub inner_typoid: pg_sys::Oid,

    pub normalizer: Option<SearchNormalizer>,
}

/// Recursively strips tokenizer casts (e.g. `pdb.literal`, `pdb.alias`) from an expression.
pub unsafe fn strip_tokenizer_cast(node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    if node.is_null() {
        return node;
    }

    if let Some(func) = nodecast!(FuncExpr, T_FuncExpr, node) {
        if type_is_tokenizer((*func).funcresulttype) {
            let args = PgList::<pg_sys::Node>::from_pg((*func).args);
            if let Some(arg) = args.get_ptr(0) {
                return strip_tokenizer_cast(arg);
            }
        }
    } else if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, node) {
        return strip_tokenizer_cast((*relabel).arg.cast());
    } else if let Some(coerce) = nodecast!(CoerceToDomain, T_CoerceToDomain, node) {
        return strip_tokenizer_cast((*coerce).arg.cast());
    } else if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, node) {
        return strip_tokenizer_cast((*coerce).arg.cast());
    }

    node
}

/// Recursively strips `UNNEST` function calls and basic type coercion wrappers
/// (`RelabelType`, `CoerceToDomain`, `CoerceViaIO`) from an expression.
///
/// Returns a tuple containing the stripped node and a boolean indicating if `UNNEST` was found.
pub unsafe fn strip_unnest_and_relabel(mut node: *mut pg_sys::Node) -> (*mut pg_sys::Node, bool) {
    let mut found_unnest = false;
    loop {
        if node.is_null() {
            return (node, found_unnest);
        }

        if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, node) {
            node = (*relabel).arg.cast();
            continue;
        }
        if let Some(coerce) = nodecast!(CoerceToDomain, T_CoerceToDomain, node) {
            node = (*coerce).arg.cast();
            continue;
        }
        if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, node) {
            node = (*coerce).arg.cast();
            continue;
        }
        if let Some(phv) = nodecast!(PlaceHolderVar, T_PlaceHolderVar, node) {
            node = (*phv).phexpr.cast();
            continue;
        }
        if let Some(func) = nodecast!(FuncExpr, T_FuncExpr, node) {
            if is_unnest_func((*func).funcid) {
                found_unnest = true;
                let args = PgList::<pg_sys::Node>::from_pg((*func).args);
                if let Some(arg) = args.get_ptr(0) {
                    node = arg;
                    continue;
                }
            }
        }
        break;
    }
    (node, found_unnest)
}

/// Identifies if the function identified by `funcid` is `pg_catalog.unnest(anyarray)`.
pub fn is_unnest_func(funcid: pg_sys::Oid) -> bool {
    use std::sync::Once;
    static mut UNNEST_OID: pg_sys::Oid = pg_sys::InvalidOid;
    static UNNEST_OID_ONCE: Once = Once::new();

    unsafe {
        UNNEST_OID_ONCE.call_once(|| {
            if let Some(oid) = direct_function_call::<pg_sys::Oid>(
                pg_sys::regprocedurein,
                &[c"pg_catalog.unnest(anyarray)".into_datum()],
            ) {
                UNNEST_OID = oid;
            }
        });

        if funcid == UNNEST_OID && UNNEST_OID != pg_sys::InvalidOid {
            return true;
        }

        false
    }
}

/// Create a text Const node from a string
///
/// # Safety
/// This function must be called within a PostgreSQL memory context that will persist
/// for the lifetime of the plan tree. The returned Const node will be allocated in the
/// current memory context and should not be freed manually.
pub unsafe fn make_text_const(text: &str) -> *mut pg_sys::Const {
    let text_datum = text
        .into_datum()
        .expect("failed to convert string to datum");

    pg_sys::makeConst(
        pg_sys::TEXTOID,
        -1,
        pg_sys::DEFAULT_COLLATION_OID,
        -1,
        text_datum,
        false, // constisnull
        false, // constbyval (text is not passed by value)
    )
}

/// Extracts the field attributes from the index relation.
/// It returns a vector of tuples containing the field name and its type OID.
pub unsafe fn extract_field_attributes(
    indexrel: pg_sys::Relation,
) -> HashMap<FieldName, ExtractedFieldAttribute> {
    #[inline]
    unsafe fn is_text_lower(expression: *mut pg_sys::Node) -> bool {
        if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, expression) {
            if let Some(func_expr) = nodecast!(FuncExpr, T_FuncExpr, (*coerce).arg) {
                if (*func_expr).funcid == text_lower_funcoid() {
                    return true;
                }
            }
        }

        false
    }

    let heap_relation = PgSearchRelation::from_pg(indexrel).heap_relation().unwrap();
    let heap_tupdesc = heap_relation.tuple_desc();
    let pg_search_indexrel = PgSearchRelation::from_pg(indexrel);
    let created_by_version = pg_search_indexrel.created_by_version();
    let index_info = pg_search_indexrel.index_info();
    let expressions = pg_search_indexrel.index_expressions();
    let mut expressions_iter = expressions.iter_ptr().enumerate();
    let mut field_attributes: FxHashMap<FieldName, ExtractedFieldAttribute> = Default::default();
    for attno in 0..(*index_info).ii_NumIndexAttrs {
        let heap_attno = (*index_info).ii_IndexAttrNumbers[attno as usize];
        let (attname, attribute_type_oid, att_typmod, source, expression, inner_typoid, normalizer) =
            if heap_attno == 0 {
                // Is an expression.
                let Some((expression_idx, expression)) = expressions_iter.next() else {
                    panic!("Expected expression for index attribute {attno}.");
                };
                let source = FieldSource::Expression {
                    att_idx: expression_idx,
                };
                let node = expression.cast();

                let typoid = pg_sys::exprType(node);
                let mut attname = None;
                let mut typmod = -1;
                let mut inner_typoid = typoid;
                let mut normalizer = None;

                // Check if this expression is a composite type
                if is_composite_type(typoid) {
                    // Get composite fields (validates for nested composites, etc.)
                    let composite_fields =
                        get_composite_fields_for_index(typoid).unwrap_or_else(|e| panic!("{e}"));
                    let row_args =
                        row_expr_from_indexed_expr(expression.cast()).map(|row_expr| unsafe {
                            PgList::<pg_sys::Node>::from_pg((*row_expr).args)
                        });

                    // Add each field from the composite to the index
                    for comp_field in composite_fields {
                        if comp_field.is_dropped {
                            continue;
                        }

                        // Check for duplicate field name (reuse existing logic)
                        if field_attributes.contains_key(&FieldName::from(&comp_field.field_name)) {
                            panic!(
                                "indexed attribute {} defined more than once",
                                comp_field.field_name
                            );
                        }

                        let pg_type = PgOid::from_untagged(comp_field.type_oid);
                        let tantivy_type = SearchFieldType::try_from_type_info(
                            pg_type,
                            comp_field.typmod,
                            comp_field.type_oid,
                            created_by_version,
                        )
                        .unwrap_or_else(|e| panic!("{e}"));

                        field_attributes.insert(
                            comp_field.field_name.clone().into(),
                            ExtractedFieldAttribute {
                                attno: attno as usize,
                                source: FieldSource::CompositeField {
                                    index_attno: attno as usize,
                                    expression_idx,
                                    field_idx: comp_field.field_index,
                                    composite_type_oid: typoid,
                                },
                                pg_type,
                                tantivy_type,
                                inner_typoid: row_args
                                    .as_ref()
                                    .and_then(|args| args.get_ptr(comp_field.field_index))
                                    .map(|arg| {
                                        let inner_node = strip_tokenizer_cast(arg.cast());
                                        pg_sys::exprType(inner_node)
                                    })
                                    .unwrap_or(comp_field.type_oid),
                                normalizer: None,
                            },
                        );
                    }

                    // Skip normal expression handling for composite types
                    continue;
                }

                if type_is_tokenizer(typoid) {
                    typmod = pg_sys::exprTypmod(node);

                    let parsed_typmod =
                        UncheckedTypmod::try_from(typmod).unwrap_or_else(|e| panic!("{e}"));
                    let vars = find_vars(node);

                    normalizer = parsed_typmod.normalizer();
                    attname = parsed_typmod.alias();

                    // Attempt to determine inner_typoid by peeling the tokenizer cast/function.
                    // This handles cases like `(a || b)::pdb.literal('alias=...')` where vars.len() > 1.
                    if inner_typoid == typoid {
                        let inner_node = strip_tokenizer_cast(expression.cast());
                        inner_typoid = pg_sys::exprType(inner_node);
                    }

                    if attname.is_none() && vars.len() == 1 {
                        let var = vars[0];
                        inner_typoid = pg_sys::exprType(var as *mut pg_sys::Node);
                        if is_text_lower(expression.cast()) {
                            normalizer = Some(SearchNormalizer::Lowercase);
                        } else if let Some(relabel) =
                            nodecast!(RelabelType, T_RelabelType, expression)
                        {
                            if is_a((*relabel).arg.cast(), pg_sys::NodeTag::T_CoerceViaIO) {
                                inner_typoid = pg_sys::exprType((*relabel).arg.cast());
                            }

                            if is_text_lower((*relabel).arg.cast()) {
                                normalizer = Some(SearchNormalizer::Lowercase);
                            }
                        }
                        if attname.is_none() {
                            let heap_attname = heap_relation
                                .tuple_desc()
                                .get((*var).varattno as usize - 1)
                                .unwrap()
                                .name()
                                .to_string();
                            attname = Some(heap_attname);
                        }
                    }

                    if type_is_alias(typoid) {
                        if type_can_be_tokenized(inner_typoid) {
                            panic!("To alias a text or JSON type, cast it to a tokenizer with an `alias` argument instead of `pdb.alias`");
                        } else {
                            // if we have a non text field text to `pdb.alias`, unwrap it to get the inner typoid
                            if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, expression)
                            {
                                if let Some(func) = nodecast!(FuncExpr, T_FuncExpr, (*relabel).arg)
                                {
                                    let args = PgList::<pg_sys::Node>::from_pg((*func).args);
                                    if args.len() == 1 {
                                        if let Some(arg) = args.get_ptr(0) {
                                            if let Some(inner_func) =
                                                nodecast!(FuncExpr, T_FuncExpr, arg)
                                            {
                                                inner_typoid = (*inner_func).funcresulttype;
                                            } else if let Some(op_expr) =
                                                nodecast!(OpExpr, T_OpExpr, arg)
                                            {
                                                inner_typoid = (*op_expr).opresulttype;
                                            }
                                        }
                                    }
                                }
                            }

                            // use the alias name as the field name instead of the heap attribute name
                            let alias_typmod =
                                AliasTypmod::try_from(typmod).unwrap_or_else(|e| panic!("{e}"));
                            attname = alias_typmod.alias();
                        }
                    }
                }

                let Some(attname) = attname else {
                    let expr_str = deparse_expr(None, &heap_relation, expression.cast());
                    panic!(
                        "indexed expression requires a tokenizer cast with an alias: {expr_str}"
                    );
                };

                (
                    attname,
                    typoid,
                    typmod,
                    source,
                    Some(expression),
                    inner_typoid,
                    normalizer,
                )
            } else {
                // Is a field -- get the field name from the heap relation.
                let attno = (heap_attno - 1) as usize;
                let att = heap_tupdesc.get(attno).expect("attribute should exist");
                (
                    att.name().to_owned(),
                    att.type_oid().value(),
                    att.type_mod(),
                    FieldSource::Heap { attno },
                    None,
                    att.type_oid().value(),
                    None,
                )
            };

        if field_attributes.contains_key(&FieldName::from(&attname)) {
            panic!("indexed attribute {attname} defined more than once");
        }

        let pg_type = PgOid::from_untagged(attribute_type_oid);
        let tantivy_type = SearchFieldType::try_from_type_info(
            pg_type,
            att_typmod,
            inner_typoid,
            created_by_version,
        )
        .unwrap_or_else(|e| panic!("{e}"));

        // non-plain-attribute expressions that aren't cast to a tokenizer type are forced to use our `pdb.literal` tokenizer
        let missing_tokenizer_cast = expression.is_some()
            && att_typmod == -1
            && matches!(tantivy_type, SearchFieldType::Text(..));
        if missing_tokenizer_cast {
            let expr_str =
                unsafe { deparse_expr(None, &heap_relation, expression.unwrap().cast()) };
            panic!("indexed expression must be cast to a tokenizer: {expr_str}");
        }

        field_attributes.insert(
            attname.into(),
            ExtractedFieldAttribute {
                attno: attno as usize,
                source,
                pg_type,
                tantivy_type,
                inner_typoid,
                normalizer,
            },
        );
    }
    field_attributes
}

pub unsafe fn row_to_search_document<'a>(
    categorized_fields: impl Iterator<
        Item = (
            pg_sys::Datum,
            bool,
            &'a SearchField,
            &'a CategorizedFieldData,
        ),
    >,
    document: &mut tantivy::TantivyDocument,
    created_by_version: Option<Version>,
) -> Result<(), IndexError> {
    for (
        datum,
        isnull,
        search_field,
        CategorizedFieldData {
            pg_type,
            base_oid,
            is_key_field,
            is_array,
            is_json,
            ..
        },
    ) in categorized_fields
    {
        if isnull && *is_key_field {
            return Err(IndexError::KeyIdNull(search_field.field_name().to_string()));
        }

        if isnull {
            continue;
        }

        // For pdb.alias/tokenizer types, get the underlying type if it's not a text type.
        let actual_datum = if type_is_alias(pg_type.value()) || type_is_tokenizer(pg_type.value()) {
            unsafe { DatumWithType::get_underlying_type(datum).0 }
        } else {
            datum
        };

        if *is_array {
            let converted_array = match search_field.field_type() {
                SearchFieldType::Numeric64(_, scale) => {
                    TantivyValue::try_from_numeric_array_i64(actual_datum, scale)
                }
                SearchFieldType::NumericBytes(..) => {
                    TantivyValue::try_from_numeric_array_bytes(actual_datum)
                }
                // Legacy pre-v0.22.0 indexes stored NUMERIC arrays as F64 in the tantivy schema.
                SearchFieldType::F64(oid) if oid == pg_sys::NUMERICOID => {
                    TantivyValue::try_from_numeric_array_f64(actual_datum)
                }
                _ => TantivyValue::try_from_datum_array(actual_datum, *base_oid),
            }
            .unwrap_or_else(|e| {
                panic!("could not parse field `{}`: {e}", search_field.field_name())
            });
            for value in converted_array {
                document.add_field_value(
                    search_field.field(),
                    &value.into_tantivy_value(created_by_version),
                );
            }
        } else if *is_json {
            for value in
                TantivyValue::try_from_datum_json(actual_datum, *base_oid).unwrap_or_else(|e| {
                    panic!("could not parse field `{}`: {e}", search_field.field_name())
                })
            {
                document.add_field_value(
                    search_field.field(),
                    &value.into_tantivy_value(created_by_version),
                );
            }
        } else {
            // Check for NUMERIC field types that need special handling
            let tv = match search_field.field_type() {
                SearchFieldType::Numeric64(_, scale) => {
                    TantivyValue::try_from_numeric_i64(actual_datum, scale)
                }
                SearchFieldType::NumericBytes(..) => {
                    TantivyValue::try_from_numeric_bytes(actual_datum)
                }
                // Legacy pre-v0.22.0 indexes stored NUMERIC as F64 in the tantivy schema.
                SearchFieldType::F64(oid) if oid == pg_sys::NUMERICOID => {
                    TantivyValue::try_from_numeric_f64(actual_datum)
                }
                _ => TantivyValue::try_from_datum(actual_datum, *base_oid),
            }
            .unwrap_or_else(|e| {
                panic!("could not parse field `{}`: {e}", search_field.field_name())
            });
            document.add_field_value(
                search_field.field(),
                &tv.into_tantivy_value(created_by_version),
            );
        }
    }
    Ok(())
}

pub fn convert_pg_date_string(typeoid: PgOid, date_string: &str) -> PostgresDateTime {
    match typeoid {
        PgOid::BuiltIn(PgBuiltInOids::DATEOID | PgBuiltInOids::DATERANGEOID) => {
            let d = pgrx::datum::Date::from_str(date_string)
                .expect("must be valid postgres date format");
            PostgresDateTime::from(d)
        }
        // For TIMESTAMPOID, Used only by legacy indexes as of v0.24.0.
        PgOid::BuiltIn(PgBuiltInOids::TIMESTAMPOID | PgBuiltInOids::TSRANGEOID) => {
            let t = pgrx::datum::Timestamp::from_str(date_string)
                .expect("must be a valid postgres timestamp");
            PostgresDateTime::from(t)
        }
        // For TIMESTAMPTZOID, Used only by legacy indexes as of v0.24.0.
        PgOid::BuiltIn(PgBuiltInOids::TIMESTAMPTZOID | pg_sys::BuiltinOid::TSTZRANGEOID) => {
            let twtz = pgrx::datum::TimestampWithTimeZone::from_str(date_string)
                .expect("must be a valid postgres timestamp with time zone");
            PostgresDateTime::from(twtz)
        }
        PgOid::BuiltIn(PgBuiltInOids::TIMEOID) => {
            let t =
                pgrx::datum::Time::from_str(date_string).expect("must be a valid postgres time");
            PostgresDateTime::from(t)
        }
        PgOid::BuiltIn(PgBuiltInOids::TIMETZOID) => {
            let twtz = pgrx::datum::TimeWithTimeZone::from_str(date_string)
                .expect("must be a valid postgres time with time zone");
            PostgresDateTime::from(twtz)
        }
        _ => panic!("Unsupported typeoid: {typeoid:?}"),
    }
}

/// Extract precision and scale from PostgreSQL NUMERIC typmod.
///
/// PostgreSQL encodes NUMERIC precision and scale in the typmod as:
/// `typmod = ((precision << 16) | (scale & 0x7ff)) + VARHDRSZ`
///
/// The scale is stored in 11 bits (0x7ff = 2047), supporting the range [-1000, 1000].
/// Negative scales are stored in two's complement within those 11 bits.
///
/// # Returns
/// - `(precision, Some(scale))` if typmod specifies precision/scale
/// - `(0, None)` if typmod is -1 (unlimited/unspecified precision)
///
/// # Note
/// - For NUMERIC columns declared without precision (e.g., `NUMERIC` instead of `NUMERIC(10,2)`),
///   PostgreSQL uses typmod = -1, indicating arbitrary precision.
/// - PostgreSQL 15+ supports negative scales (e.g., NUMERIC(5,-3) rounds to nearest 1000).
///
/// # Reference
/// See PostgreSQL's `numeric_typmod_scale()` in src/backend/utils/adt/numeric.c:
/// ```c
/// return (((typmod - VARHDRSZ) & 0x7ff) ^ 1024) - 1024;
/// ```
pub fn extract_numeric_precision_scale(typmod: i32) -> (u16, Option<i16>) {
    // PostgreSQL uses typmod = -1 for unlimited precision NUMERIC
    if typmod < 0 {
        return (0, None);
    }

    // Extract precision and scale from typmod
    // See PostgreSQL's make_numeric_typmod() and numeric_typmod_scale()
    let typmod_val = (typmod - pg_sys::VARHDRSZ as i32) as u32;

    // Precision is in upper 16 bits
    let precision = ((typmod_val >> 16) & 0xFFFF) as u16;

    // Scale is stored in lower 11 bits (0x7ff), using two's complement for negatives.
    // The formula (x ^ 1024) - 1024 sign-extends an 11-bit value to a full integer.
    // This works because 1024 = 2^10, the midpoint of the 11-bit range.
    let stored_scale = (typmod_val & 0x7ff) as i16;
    let scale = (stored_scale ^ 1024) - 1024;

    (precision, Some(scale))
}

// Backport of `store_att_byval()` that works on pg15
// Writes a datum `arg_newdatum` into memory pointed to by `arg_t`
// SAFETY: `arg_t` must point at writable memory of sufficient size to store the
//         Postgres value represented by `arg_newdatum`.
//         `arg_newdatum` must be a valid postgres value with the specified attlen.
pub unsafe fn store_att_byval(
    arg_t: *mut std::ffi::c_void,
    arg_newdatum: pg_sys::Datum,
    arg_attlen: std::ffi::c_int,
) {
    #[cfg(not(feature = "pg15"))]
    unsafe {
        pg_sys::store_att_byval(arg_t, arg_newdatum, arg_attlen)
    }

    #[cfg(feature = "pg15")]
    unsafe {
        match arg_attlen {
            1 => arg_t.cast::<i8>().write(arg_newdatum.value() as i8),
            2 => arg_t.cast::<i16>().write(arg_newdatum.value() as i16),
            4 => arg_t.cast::<i32>().write(arg_newdatum.value() as i32),
            8 => arg_t.cast::<i64>().write(arg_newdatum.value() as i64),
            _ => unreachable!(),
        }
    }
}

// Backport of `fetch_att_byval()` that works on pg15
// Returns a Datum representation of the Postgres value stored in `arg_t`
// SAFETY: The value stored in `arg_t` must be a valid Postgres value with
//         the specified by-val and att-len.
//         If the type is not by-val the provided pointer must live as long as
//         the Datum.
pub unsafe fn fetch_att(
    arg_t: *const ::core::ffi::c_void,
    arg_attbyval: bool,
    arg_attlen: ::core::ffi::c_int,
) -> pg_sys::Datum {
    #[cfg(not(feature = "pg15"))]
    unsafe {
        pg_sys::fetch_att(arg_t, arg_attbyval, arg_attlen)
    }

    #[cfg(feature = "pg15")]
    if arg_attbyval {
        unsafe {
            match arg_attlen {
                1 => pg_sys::Datum::from(arg_t.cast::<i8>().read()),
                2 => pg_sys::Datum::from(arg_t.cast::<i16>().read()),
                4 => pg_sys::Datum::from(arg_t.cast::<i32>().read()),
                8 => pg_sys::Datum::from(arg_t.cast::<i64>().read()),
                _ => unreachable!(),
            }
        }
    } else {
        pg_sys::Datum::from(arg_t)
    }
}

type IsArray = bool;
/// Returns the base type of the given `oid`, and a boolean indicating if the
/// type is an array.
pub fn resolve_base_type(oid: PgOid) -> Option<(PgOid, IsArray)> {
    fn is_domain_type(oid: pg_sys::Oid) -> bool {
        unsafe { pg_sys::get_typtype(oid) as u8 == pg_sys::TYPTYPE_DOMAIN }
    }

    if matches!(oid, PgOid::Invalid) {
        return None;
    }

    // resolve domain type to its base
    let base_oid = if is_domain_type(oid.value()) {
        let resolved_type = unsafe { pg_sys::getBaseType(oid.value()) };
        if resolved_type == pg_sys::InvalidOid {
            return None;
        }
        resolved_type
    } else {
        oid.value()
    };

    // check if it's an array type
    let array_type = PgOid::from(unsafe { pg_sys::get_element_type(base_oid) });

    match array_type {
        // not an array
        PgOid::Invalid => Some((base_oid.into(), false)),

        // built-in array type or custom array type
        PgOid::BuiltIn(_) | PgOid::Custom(_) => {
            let resolved_array_type = if is_domain_type(array_type.value()) {
                let resolved_type = unsafe { pg_sys::getBaseType(array_type.value()) };
                if resolved_type == pg_sys::InvalidOid {
                    return None;
                }
                resolved_type
            } else {
                array_type.value()
            };

            Some((resolved_array_type.into(), true))
        }
    }
}

pub trait ToPalloc: Sized {
    fn palloc(self) -> *mut Self {
        self.palloc_in(PgMemoryContexts::CurrentMemoryContext)
    }

    fn palloc_in(self, mcxt: PgMemoryContexts) -> *mut Self;
}

impl<T> ToPalloc for T {
    fn palloc_in(mut self, mut mcxt: PgMemoryContexts) -> *mut Self {
        unsafe { mcxt.copy_ptr_into((&mut self as *mut T).cast(), size_of::<T>()) }
    }
}

/// Recursively check if an expression tree contains any of the specified operators
/// Uses PostgreSQL's expression_tree_walker for robust traversal
///
/// NOTE: This logic is duplicated with `extract_quals` in qual_inspect.rs.
/// Both need to traverse expression trees looking for operators, so changes to one
/// should be reflected in the other.
/// TODO: Consider unifying this logic to avoid duplication (see GitHub issue #3455)
pub unsafe fn expr_contains_any_operator(
    node: *mut pg_sys::Node,
    target_opnos: &[pg_sys::Oid],
) -> bool {
    use pgrx::pg_guard;
    use std::ptr::addr_of_mut;

    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        let context = &*(data as *const Context);
        let node_type = (*node).type_;

        // Check if this node is an OpExpr with one of our target operators
        if node_type == pg_sys::NodeTag::T_OpExpr {
            let opexpr = node as *mut pg_sys::OpExpr;
            if context.target_opnos.contains(&(*opexpr).opno) {
                // Found a match! Set the flag and stop walking
                (*(data as *mut Context)).found = true;
                return true; // Stop walking
            }
        }
        pg_sys::expression_tree_walker(node, Some(walker), data)
    }

    struct Context {
        target_opnos: Vec<pg_sys::Oid>,
        found: bool,
    }

    let mut context = Context {
        target_opnos: target_opnos.to_vec(),
        found: false,
    };

    walker(node, addr_of_mut!(context).cast());
    context.found
}

/// Collects all unique RTIs (range table indices) from Var nodes in an expression tree.
/// Returns a HashSet of RTIs referenced by the expression.
pub unsafe fn expr_collect_rtis(
    node: *mut pg_sys::Node,
) -> std::collections::HashSet<pg_sys::Index> {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        let rtis = &mut *(data as *mut HashSet<pg_sys::Index>);

        if (*node).type_ == pg_sys::NodeTag::T_Var {
            let var = node as *mut pg_sys::Var;
            let varno = (*var).varno as pg_sys::Index;
            // Skip special RTIs like INNER_VAR/OUTER_VAR
            if varno > 0 && varno < pg_sys::INNER_VAR as pg_sys::Index {
                rtis.insert(varno);
            }
        }

        pg_sys::expression_tree_walker(node, Some(walker), data)
    }

    let mut rtis = HashSet::new();
    walker(node, addr_of_mut!(rtis).cast());
    rtis
}

/// A Var reference with its range table index and attribute number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarRef {
    /// Range table index (varno)
    pub rti: pg_sys::Index,
    /// Attribute number (varattno), 1-indexed
    pub attno: pg_sys::AttrNumber,
}

/// Collects all unique Var references (RTI + attribute number) from an expression tree.
/// Returns a Vec of VarRef structs for each column referenced by the expression.
///
/// If `include_special_vars` is true, variables with special varnos (like INDEX_VAR) are included.
/// If false, only variables referencing base relations (varno > 0 and < INNER_VAR) are included.
pub unsafe fn expr_collect_vars(
    node: *mut pg_sys::Node,
    include_special_vars: bool,
) -> Vec<VarRef> {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        let (vars, include_special_vars) = &mut *(data as *mut (Vec<VarRef>, bool));

        if (*node).type_ == pg_sys::NodeTag::T_Var {
            let var = node as *mut pg_sys::Var;
            let varno = (*var).varno as pg_sys::Index;
            let varattno = (*var).varattno;

            // Standard check for base relation var:
            let is_base_rel_var = varno > 0 && varno < pg_sys::INNER_VAR as pg_sys::Index;

            if *include_special_vars {
                // Include if valid attno
                if varattno > 0 {
                    vars.push(VarRef {
                        rti: varno,
                        attno: varattno,
                    });
                }
            } else {
                // Only include base relation vars
                if is_base_rel_var && varattno > 0 {
                    vars.push(VarRef {
                        rti: varno,
                        attno: varattno,
                    });
                }
            }
        }

        pg_sys::expression_tree_walker(node, Some(walker), data)
    }

    let mut context = (Vec::new(), include_special_vars);
    walker(node, addr_of_mut!(context).cast());
    context.0
}

/// Look up a function in the pdb schema by name and argument types.
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation).
pub fn lookup_pdb_function(func_name: &str, arg_types: &[pg_sys::Oid]) -> pg_sys::Oid {
    unsafe {
        // Look up the pdb schema
        let pdb_schema = pg_sys::get_namespace_oid(c"pdb".as_ptr(), true);
        if pdb_schema == pg_sys::InvalidOid {
            return pg_sys::InvalidOid;
        }

        // Build the qualified function name list: pdb.<func_name>
        let mut func_name_list = PgList::<pg_sys::Node>::new();
        func_name_list.push(pg_sys::makeString(c"pdb".as_ptr() as *mut std::ffi::c_char) as *mut _);
        // Convert func_name to CString for makeString
        let func_name_cstr = std::ffi::CString::new(func_name).unwrap();
        func_name_list
            .push(pg_sys::makeString(func_name_cstr.as_ptr() as *mut std::ffi::c_char) as *mut _);

        // LookupFuncName returns InvalidOid if function doesn't exist (with missing_ok = true)
        pg_sys::LookupFuncName(
            func_name_list.as_ptr(),
            arg_types.len() as i32,
            arg_types.as_ptr(),
            true, // missing_ok = true, don't error if not found
        )
    }
}

/// Returns true if the pg_search extension is installed in the current database.
///
/// This is used to guard shared_preload_libraries hooks from touching schemas/types/functions
/// that only exist after `CREATE EXTENSION pg_search`.
pub fn pg_search_extension_installed() -> bool {
    unsafe { pg_sys::get_extension_oid(c"pg_search".as_ptr(), true) != pg_sys::InvalidOid }
}

/// RAII wrapper for `pg_sys::List` that automatically frees the list on drop.
///
/// This is useful when you need to create a temporary PostgreSQL list for use with
/// PostgreSQL functions and want to ensure it's properly freed even if the code
/// returns early or panics.
///
/// # Example
/// ```ignore
/// let temp_list = TempPgList::new();
/// temp_list.push(some_node as *mut std::ffi::c_void);
/// let result = pg_sys::some_function(temp_list.as_ptr());
/// // temp_list is automatically freed when it goes out of scope
/// ```
#[derive(Default)]
pub struct TempPgList(*mut pg_sys::List);

impl TempPgList {
    /// Create a new empty temporary list.
    pub fn new() -> Self {
        Self(std::ptr::null_mut())
    }

    /// Append a cell to the list.
    ///
    /// # Safety
    /// The caller must ensure that `datum` is a valid pointer that can be
    /// stored in a PostgreSQL list.
    pub unsafe fn push(&mut self, datum: *mut std::ffi::c_void) {
        self.0 = pg_sys::lappend(self.0, datum);
    }

    /// Get the raw pointer to the list.
    pub fn as_ptr(&self) -> *mut pg_sys::List {
        self.0
    }
}

impl Drop for TempPgList {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                pg_sys::list_free(self.0);
            }
        }
    }
}

/// Returns `true` if `index_predicate` belongs to a partial index whose predicate
/// is NOT implied by the query's restriction clauses -- i.e. the query is missing
/// the predicate, so the index is missing rows the query needs and cannot safely
/// answer it. Returns `false` for a non-partial index, or a partial index whose
/// predicate the query implies.
pub unsafe fn missing_partial_index_predicate(
    index_predicate: *mut pg_sys::List,
    restrict_info: &PgList<pg_sys::RestrictInfo>,
) -> bool {
    // Not a partial index: nothing can be missing.
    if index_predicate.is_null() {
        return false;
    }

    let mut clause_list: *mut pg_sys::List = std::ptr::null_mut();
    for ri in restrict_info.iter_ptr() {
        clause_list = pg_sys::lappend(clause_list, (*ri).clause as *mut std::ffi::c_void);
    }
    !pg_sys::predicate_implied_by(index_predicate, clause_list, false)
}

/// Filter out RestrictInfo entries whose clauses are implied by a partial index predicate.
///
/// When using a partial index (e.g., `CREATE INDEX ... WHERE deleted_at IS NULL`),
/// the index only contains rows that satisfy the predicate. If the query's WHERE clause
/// includes the same predicate, we don't need to create a heap filter for it since the
/// partial index already guarantees it.
///
/// This function uses PostgreSQL's `predicate_implied_by` to check if the index predicate
/// implies each query clause. If so, that clause is filtered out.
///
/// This is only the redundant-clause optimization; it does NOT verify the query is
/// compatible with the partial index. Callers must separately gate on
/// [`missing_partial_index_predicate`].
pub unsafe fn filter_implied_predicates(
    index_predicate: *mut pg_sys::List,
    restrict_info: &PgList<pg_sys::RestrictInfo>,
) -> PgList<pg_sys::RestrictInfo> {
    // If there's no partial index predicate, return the original list unchanged
    if index_predicate.is_null() {
        return PgList::from_pg(restrict_info.as_ptr());
    }

    // Build a new list with only the predicates that are NOT implied by the index predicate
    let mut filtered_list: *mut pg_sys::List = std::ptr::null_mut();

    for ri in restrict_info.iter_ptr() {
        let clause = (*ri).clause;
        let mut clause_list = TempPgList::new();
        clause_list.push(clause as *mut std::ffi::c_void);

        // Check if the index predicate implies this clause
        // predicate_implied_by(A, B, false) returns true if B => A
        let is_implied = pg_sys::predicate_implied_by(clause_list.as_ptr(), index_predicate, false);

        if !is_implied {
            filtered_list = pg_sys::lappend(filtered_list, ri as *mut std::ffi::c_void);
        }
    }

    PgList::from_pg(filtered_list)
}

#[macro_export]
macro_rules! debug1 {
    ($($arg:tt)*) => {{
        if unsafe { pg_sys::message_level_is_interesting(pg_sys::DEBUG1 as _) } {
            pg_sys::debug1!($($arg)*);
        }
    }};
}

#[macro_export]
macro_rules! debug2 {
    ($($arg:tt)*) => {{
        if unsafe { pg_sys::message_level_is_interesting(pg_sys::DEBUG2 as _) } {
            pg_sys::debug2!($($arg)*);
        }
    }};
}

/// Ensure all Var nodes referenced by `expr` are present in `tlist`.
///
/// For each base-relation Var in the expression, if no matching
/// `(varno, varattno)` entry exists yet, a new resjunk `TargetEntry` is
/// appended. This is used to populate `custom_scan_tlist` so that
/// `set_customscan_references` can resolve Var references in
/// `custom_exprs`.
pub unsafe fn add_vars_to_tlist(expr: *mut pg_sys::Node, tlist: &mut PgList<pg_sys::TargetEntry>) {
    if expr.is_null() {
        return;
    }

    for var_ptr in crate::postgres::var::find_vars(expr) {
        let varno = (*var_ptr).varno as pg_sys::Index;
        let varattno = (*var_ptr).varattno;

        if varno == 0 || varattno <= 0 {
            continue;
        }

        let already_present = tlist.iter_ptr().any(|te| {
            if (*(*te).expr).type_ == pg_sys::NodeTag::T_Var {
                let existing = (*te).expr as *mut pg_sys::Var;
                (*existing).varno as pg_sys::Index == varno && (*existing).varattno == varattno
            } else {
                false
            }
        });

        if !already_present {
            let resno = tlist.len() as pg_sys::AttrNumber + 1;
            let new_var = pg_sys::copyObjectImpl(var_ptr.cast()).cast::<pg_sys::Var>();
            let te = pg_sys::makeTargetEntry(new_var.cast(), resno, std::ptr::null_mut(), true);
            tlist.push(te);
        }
    }
}

/// Recursively peels `RelabelType` and `PlaceHolderVar` wrappers to get the underlying node.
pub unsafe fn strip_wrappers(mut node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    loop {
        if node.is_null() {
            return node;
        }
        match (*node).type_ {
            pg_sys::NodeTag::T_RelabelType => {
                node = (*(node as *mut pg_sys::RelabelType)).arg.cast();
            }
            pg_sys::NodeTag::T_PlaceHolderVar => {
                node = (*(node as *mut pg_sys::PlaceHolderVar)).phexpr.cast();
            }
            _ => break,
        }
    }
    node
}

/// Unwraps `PlaceHolderVar` nodes (if any) and checks if the underlying node is a `FuncExpr`
/// whose OID is present in `funcoids`. Returns true if it matches.
pub unsafe fn is_search_operator(node: *mut pg_sys::Node, funcoids: &[pg_sys::Oid]) -> bool {
    if node.is_null() {
        return false;
    }

    let check_node = strip_wrappers(node);

    if (*check_node).type_ == pg_sys::NodeTag::T_FuncExpr {
        let funcexpr = check_node as *mut pg_sys::FuncExpr;
        if funcoids.contains(&(*funcexpr).funcid) {
            return true;
        }
    }
    false
}

/// Helper function to inspect the parent plan's requirements (`processed_tlist`
/// and `pathkeys`) and add any missing search operator function calls (like `pdb.score(...)`)
/// into the CustomScan's targetlist.
///
/// This ensures that if the CustomScan is expected to output a score (or similar),
/// it is actually present in its `targetlist` so the `Result` node can simply
/// project it, rather than Postgres natively trying to execute the dummy function.
pub unsafe fn add_missing_search_operators_to_tlist(
    root: *mut pg_sys::PlannerInfo,
    best_path: *mut pg_sys::Path,
    tlist: &mut PgList<pg_sys::TargetEntry>,
    search_operator_funcoids: &[pg_sys::Oid],
) {
    let mut add_missing_func = |expr: *mut pg_sys::Node| {
        if !is_search_operator(expr, search_operator_funcoids) {
            return;
        }

        let unwrapped_expr = strip_wrappers(expr.cast());
        let already_present = tlist.iter_ptr().any(|te| {
            pg_sys::equal(
                strip_wrappers((*te).expr.cast()).cast(),
                unwrapped_expr.cast(),
            )
        });

        if !already_present {
            let resno = tlist.len() as pg_sys::AttrNumber + 1;
            let te = pg_sys::makeTargetEntry(
                pg_sys::copyObjectImpl(expr.cast()).cast(),
                resno,
                std::ptr::null_mut(),
                true, // resjunk
            );
            tlist.push(te);
        }
    };

    // Look in processed_tlist
    if !(*root).processed_tlist.is_null() {
        let p_tlist = PgList::<pg_sys::TargetEntry>::from_pg((*root).processed_tlist);
        for te in p_tlist.iter_ptr() {
            add_missing_func((*te).expr.cast());
        }
    }

    // Look in pathkeys
    if !best_path.is_null() && !(*best_path).pathkeys.is_null() {
        let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*best_path).pathkeys);
        for pk in pathkeys.iter_ptr() {
            let eclass = (*pk).pk_eclass;
            if !eclass.is_null() {
                let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*eclass).ec_members);
                for em in members.iter_ptr() {
                    add_missing_func((*em).em_expr.cast());
                }
            }
        }
    }
}
