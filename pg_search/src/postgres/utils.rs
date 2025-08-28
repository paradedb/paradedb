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

use crate::api::{FieldName, HashMap};
use crate::index::writer::index::IndexError;
use crate::postgres::build::is_bm25_index;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::TantivyValue;
use crate::schema::{CategorizedFieldData, SearchField, SearchFieldType};
use crate::PG_SEARCH_PREFIX;
use anyhow::{anyhow, Result};
use chrono::{NaiveDate, NaiveTime};
use pgrx::itemptr::{item_pointer_get_both, item_pointer_set_all};
use pgrx::*;
use rustc_hash::FxHashMap;
use std::str::FromStr;
use tantivy::schema::OwnedValue;

extern "C-unwind" {
    // SAFETY: `IsTransactionState()` doesn't raise an ERROR.  As such, we can avoid the pgrx
    // sigsetjmp overhead by linking to the function directly.
    pub fn IsTransactionState() -> bool;
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

/// Represents the metadata extracted from an index attribute
#[derive(Debug)]
pub struct ExtractedFieldAttribute {
    /// its ordinal position in the index attribute list
    pub attno: usize,

    /// its original Postgres type OID
    pub pg_type: PgOid,

    /// the type we'll use for indexing in tantivy
    pub tantivy_type: SearchFieldType,
}

/// Extracts the field attributes from the index relation.
/// It returns a vector of tuples containing the field name and its type OID.
pub unsafe fn extract_field_attributes(
    indexrel: pg_sys::Relation,
) -> HashMap<FieldName, ExtractedFieldAttribute> {
    let tupdesc = PgTupleDesc::from_pg_unchecked((*indexrel).rd_att);
    let index_info = pg_sys::BuildIndexInfo(indexrel);
    let expressions = PgList::<pg_sys::Expr>::from_pg((*index_info).ii_Expressions);
    let mut expressions_iter = expressions.iter_ptr();

    let mut field_attributes: FxHashMap<FieldName, ExtractedFieldAttribute> = Default::default();
    for attno in 0..(*index_info).ii_NumIndexAttrs {
        let heap_attno = (*index_info).ii_IndexAttrNumbers[attno as usize];
        let (attname, attribute_type_oid) = if heap_attno == 0 {
            // Is an expression.
            let Some(expression) = expressions_iter.next() else {
                panic!("Expected expression for index attribute {attno}.");
            };
            let node = expression.cast();
            (
                format!("{PG_SEARCH_PREFIX}{attno}").into(),
                pg_sys::exprType(node),
            )
        } else {
            // Is a field.
            let att = tupdesc.get(attno as usize).expect("attribute should exist");
            (att.name().to_owned().into(), att.type_oid().value())
        };

        let pg_type = PgOid::from_untagged(attribute_type_oid);
        let tantivy_type = SearchFieldType::try_from(pg_type).unwrap_or_else(|e| panic!("{e}"));
        field_attributes.insert(
            attname,
            ExtractedFieldAttribute {
                attno: attno as usize,
                pg_type,
                tantivy_type,
            },
        );
    }
    field_attributes
}

pub unsafe fn row_to_search_document(
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    key_field_name: &FieldName,
    categorized_fields: &Vec<(SearchField, CategorizedFieldData)>,
    document: &mut tantivy::TantivyDocument,
) -> Result<(), IndexError> {
    for (
        search_field,
        CategorizedFieldData {
            attno,
            base_oid,
            is_array,
            is_json,
        },
    ) in categorized_fields
    {
        let datum = *values.add(*attno);
        let isnull = *isnull.add(*attno);

        if isnull && key_field_name == search_field.field_name() {
            return Err(IndexError::KeyIdNull(key_field_name.to_string()));
        }

        if isnull {
            continue;
        }

        if *is_array {
            for value in TantivyValue::try_from_datum_array(datum, *base_oid)? {
                document.add_field_value(search_field.field(), &OwnedValue::from(value));
            }
        } else if *is_json {
            for value in TantivyValue::try_from_datum_json(datum, *base_oid)? {
                document.add_field_value(search_field.field(), &OwnedValue::from(value));
            }
        } else {
            document.add_field_value(
                search_field.field(),
                &OwnedValue::from(TantivyValue::try_from_datum(datum, *base_oid)?),
            );
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
            let (seconds, micros, _nanos) = convert_pgrx_seconds_to_chrono(t.second())
                .expect("must not overflow converting pgrx seconds");
            let naive_time = NaiveTime::from_hms_micro_opt(
                t.hour().into(),
                t.minute().into(),
                seconds,
                micros,
            )
            .expect("must be able to parse time");
            let naive_date = NaiveDate::from_ymd_opt(1970, 1, 1).expect("default date");
            let micros = naive_date.and_time(naive_time).and_utc().timestamp_micros();
            tantivy::DateTime::from_timestamp_micros(micros)
        }
        PgOid::BuiltIn(PgBuiltInOids::TIMETZOID) => {
            let twtz = pgrx::datum::TimeWithTimeZone::from_str(date_string)
                .expect("must be a valid postgres time with time zone")
                .to_utc();
            let (seconds, micros, _nanos) = convert_pgrx_seconds_to_chrono(twtz.second())
                .expect("must not overflow converting pgrx seconds");
            let naive_time = NaiveTime::from_hms_micro_opt(
                twtz.hour().into(),
                twtz.minute().into(),
                seconds,
                micros,
            )
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
