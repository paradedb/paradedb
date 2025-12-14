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

use crate::api::tokenizers::{
    type_can_be_tokenized, type_is_alias, type_is_tokenizer, AliasTypmod, UncheckedTypmod,
};
use crate::api::{FieldName, HashMap};
use crate::index::writer::index::IndexError;
use crate::nodecast;
use crate::postgres::build::is_bm25_index;
use crate::postgres::customscan::pdbscan::text_lower_funcoid;
use crate::postgres::deparse::deparse_expr;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::TantivyValue;
use crate::postgres::var::find_vars;
use crate::schema::{CategorizedFieldData, SearchField, SearchFieldType};
use anyhow::{anyhow, Result};
use chrono::{NaiveDate, NaiveTime};
use pgrx::itemptr::{item_pointer_get_both, item_pointer_set_all};
use pgrx::*;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::str::FromStr;
use tantivy::schema::OwnedValue;
use tokenizers::SearchNormalizer;

extern "C-unwind" {
    // SAFETY: `IsTransactionState()` doesn't raise an ERROR.  As such, we can avoid the pgrx
    // sigsetjmp overhead by linking to the function directly.
    pub fn IsTransactionState() -> bool;
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

impl Drop for ExprContextGuard {
    fn drop(&mut self) {
        unsafe {
            // If this is an abort or other unclean shutdown, setting `isCommit = false` will avoid
            // complex cleanup logic.
            let is_commit = pg_sys::IsTransactionState() && !std::thread::panicking();
            pg_sys::FreeExprContext(self.0, is_commit);
        }
    }
}

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

/// Finds and returns the `USING bm25` index on the specified relation with the
/// highest OID, or [`None`] if there aren't any.
pub fn locate_bm25_index(heaprelid: pg_sys::Oid) -> Option<PgSearchRelation> {
    locate_bm25_index_from_heaprel(&PgSearchRelation::open(heaprelid))
}

/// Finds and returns the `USING bm25` index on the specified relation with the
/// highest OID, or [`None`] if there aren't any.
pub fn locate_bm25_index_from_heaprel(heaprel: &PgSearchRelation) -> Option<PgSearchRelation> {
    unsafe {
        let indices = heaprel.indices(pg_sys::AccessShareLock as _);

        // Find all bm25 indexes and keep the one with highest OID
        indices
            .into_iter()
            .filter(|index| pg_sys::get_index_isvalid(index.oid()) && is_bm25_index(index))
            .max_by_key(|index| index.oid().to_u32())
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

#[derive(Copy, Clone, Debug)]
pub enum FieldSource {
    Heap { attno: usize },
    Expression { att_idx: usize },
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

/// Extracts the field attributes from the index relation.
/// It returns a vector of tuples containing the field name and its type OID.
pub unsafe fn extract_field_attributes(
    indexrel: pg_sys::Relation,
) -> HashMap<FieldName, ExtractedFieldAttribute> {
    let heap_relation = PgSearchRelation::from_pg(indexrel).heap_relation().unwrap();
    let heap_tupdesc = heap_relation.tuple_desc();
    let index_info = pg_sys::BuildIndexInfo(indexrel);
    let expressions = PgList::<pg_sys::Expr>::from_pg((*index_info).ii_Expressions);
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

                if type_is_tokenizer(typoid) {
                    typmod = pg_sys::exprTypmod(node);

                    let parsed_typmod =
                        UncheckedTypmod::try_from(typmod).unwrap_or_else(|e| panic!("{e}"));
                    let vars = find_vars(node);

                    normalizer = parsed_typmod.normalizer();
                    attname = parsed_typmod.alias();

                    if attname.is_none() && vars.len() == 1 {
                        let var = vars[0];
                        let heap_attname = heap_relation
                            .tuple_desc()
                            .get((*var).varattno as usize - 1)
                            .unwrap()
                            .name()
                            .to_string();

                        inner_typoid = pg_sys::exprType(var as *mut pg_sys::Node);
                        if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, expression) {
                            if let Some(func_expr) = nodecast!(FuncExpr, T_FuncExpr, (*coerce).arg)
                            {
                                if (*func_expr).funcid == text_lower_funcoid() {
                                    normalizer = Some(SearchNormalizer::Lowercase);
                                }
                            }
                        } else if let Some(relabel) =
                            nodecast!(RelabelType, T_RelabelType, expression)
                        {
                            if is_a((*relabel).arg.cast(), pg_sys::NodeTag::T_CoerceViaIO) {
                                inner_typoid = pg_sys::exprType((*relabel).arg.cast());
                            }
                        }

                        attname = Some(heap_attname);
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
        let tantivy_type = SearchFieldType::try_from((pg_type, att_typmod, inner_typoid))
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
) -> Result<(), IndexError> {
    for (
        datum,
        isnull,
        search_field,
        CategorizedFieldData {
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

        if *is_array {
            for value in TantivyValue::try_from_datum_array(datum, *base_oid).unwrap_or_else(|e| {
                panic!("could not parse field `{}`: {e}", search_field.field_name())
            }) {
                document.add_field_value(search_field.field(), &OwnedValue::from(value));
            }
        } else if *is_json {
            for value in TantivyValue::try_from_datum_json(datum, *base_oid).unwrap_or_else(|e| {
                panic!("could not parse field `{}`: {e}", search_field.field_name())
            }) {
                document.add_field_value(search_field.field(), &OwnedValue::from(value));
            }
        } else {
            let tv = TantivyValue::try_from_datum(datum, *base_oid).unwrap_or_else(|e| {
                panic!("could not parse field `{}`: {e}", search_field.field_name())
            });
            document.add_field_value(search_field.field(), &OwnedValue::from(tv));
        }
    }
    Ok(())
}

/// Utility function for easy `f64` to `u32` conversion
fn f64_to_u32(n: f64) -> Result<u32> {
    let truncated = n.trunc();
    if truncated.is_nan()
        || truncated.is_infinite()
        || truncated < 0.0
        || truncated > u32::MAX.into()
    {
        return Err(anyhow!("overflow in f64 to u32"));
    }

    Ok(truncated as u32)
}

/// Seconds are represented by `f64` in pgrx, with a maximum of microsecond precision
fn convert_pgrx_seconds_to_chrono(orig: f64) -> Result<(u32, u32, u32)> {
    let seconds = f64_to_u32(orig)?;
    let microseconds = f64_to_u32((orig * 1_000_000.0) % 1_000_000.0)?;
    let nanoseconds = f64_to_u32((orig * 1_000_000_000.0) % 1_000_000_000.0)?;
    Ok((seconds, microseconds, nanoseconds))
}

pub fn convert_pg_date_string(typeoid: PgOid, date_string: &str) -> tantivy::DateTime {
    match typeoid {
        PgOid::BuiltIn(PgBuiltInOids::DATEOID | PgBuiltInOids::DATERANGEOID) => {
            let d = pgrx::datum::Date::from_str(date_string)
                .expect("must be valid postgres date format");
            let micros = NaiveDate::from_ymd_opt(d.year(), d.month().into(), d.day().into())
                .expect("must be able to parse date format")
                .and_hms_opt(0, 0, 0)
                .expect("must be able to set date default time")
                .and_utc()
                .timestamp_micros();
            tantivy::DateTime::from_timestamp_micros(micros)
        }
        PgOid::BuiltIn(PgBuiltInOids::TIMESTAMPOID | PgBuiltInOids::TSRANGEOID) => {
            // Since [`pgrx::Timestamp`]s are tied to the Postgres instance's timezone,
            // to figure out *which* timezone it's actually in, we convert to a
            // [`pgrx::TimestampWithTimeZone`].
            // Once the offset is known, we can create and return a [`chrono::NaiveDateTime`]
            // with the appropriate offset.
            let t = pgrx::datum::Timestamp::from_str(date_string)
                .expect("must be a valid postgres timestamp");
            let twtz: datum::TimestampWithTimeZone = t.into();
            let (seconds, micros, _nanos) = convert_pgrx_seconds_to_chrono(twtz.second())
                .expect("must not overflow converting pgrx seconds");
            let micros =
                NaiveDate::from_ymd_opt(twtz.year(), twtz.month().into(), twtz.day().into())
                    .expect("must be able to convert date timestamp")
                    .and_hms_micro_opt(twtz.hour().into(), twtz.minute().into(), seconds, micros)
                    .expect("must be able to parse timestamp format")
                    .and_utc()
                    .timestamp_micros();
            tantivy::DateTime::from_timestamp_micros(micros)
        }
        PgOid::BuiltIn(PgBuiltInOids::TIMESTAMPTZOID | pg_sys::BuiltinOid::TSTZRANGEOID) => {
            let twtz = pgrx::datum::TimestampWithTimeZone::from_str(date_string)
                .expect("must be a valid postgres timestamp with time zone")
                .to_utc();
            let (seconds, micros, _nanos) = convert_pgrx_seconds_to_chrono(twtz.second())
                .expect("must not overflow converting pgrx seconds");
            let micros =
                NaiveDate::from_ymd_opt(twtz.year(), twtz.month().into(), twtz.day().into())
                    .expect("must be able to convert timestamp with timezone")
                    .and_hms_micro_opt(twtz.hour().into(), twtz.minute().into(), seconds, micros)
                    .expect("must be able to parse timestamp with timezone")
                    .and_utc()
                    .timestamp_micros();
            tantivy::DateTime::from_timestamp_micros(micros)
        }
        PgOid::BuiltIn(PgBuiltInOids::TIMEOID) => {
            let t =
                pgrx::datum::Time::from_str(date_string).expect("must be a valid postgres time");
            let (hour, minute, second, micros) = t.to_hms_micro();
            let naive_time =
                NaiveTime::from_hms_micro_opt(hour.into(), minute.into(), second.into(), micros)
                    .expect("must be able to parse time");
            let naive_date = NaiveDate::from_ymd_opt(1970, 1, 1).expect("default date");
            let micros = naive_date.and_time(naive_time).and_utc().timestamp_micros();
            tantivy::DateTime::from_timestamp_micros(micros)
        }
        PgOid::BuiltIn(PgBuiltInOids::TIMETZOID) => {
            let twtz = pgrx::datum::TimeWithTimeZone::from_str(date_string)
                .expect("must be a valid postgres time with time zone")
                .to_utc();
            let (hour, minute, second, micros) = twtz.to_hms_micro();
            let naive_time =
                NaiveTime::from_hms_micro_opt(hour.into(), minute.into(), second.into(), micros)
                    .expect("must be able to parse time with time zone");
            let naive_date = NaiveDate::from_ymd_opt(1970, 1, 1).expect("default date");
            let micros = naive_date.and_time(naive_time).and_utc().timestamp_micros();
            tantivy::DateTime::from_timestamp_micros(micros)
        }
        _ => panic!("Unsupported typeoid: {typeoid:?}"),
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
