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
use crate::postgres::types::{TantivyValue, TantivyValueError};
use crate::query::numeric::{bytes_to_hex, decimal_to_index_bytes};
use crate::schema::range::{TantivyRange, TantivyRangeBuilder};
use decimal_bytes::{Decimal, DecimalError};
use pgrx::datum::{Date, RangeBound, Timestamp, TimestampWithTimeZone};
use pgrx::pg_sys::Datum;
use pgrx::{AnyNumeric, FromDatum};
use std::str::FromStr;

use super::pdb_owned_value::PdbOwnedValue;

/// A NUMERIC range bound as lexicographically sortable bytes, which serialize to hex so that
/// string comparison in Tantivy's JSON fields gives correct numeric ordering.
///
/// Used for `numrange` to preserve full NUMERIC precision in range queries.
#[derive(Clone, Debug)]
pub(crate) struct SortableDecimal(Vec<u8>);

impl SortableDecimal {
    /// Encodes `val` in the byte layout of the index identified by `index_created_by_version`.
    pub(crate) fn for_index(
        val: AnyNumeric,
        index_created_by_version: Option<Version>,
    ) -> Result<Self, DecimalError> {
        let decimal = Decimal::from_str(val.normalize())?;
        Ok(SortableDecimal(decimal_to_index_bytes(
            decimal,
            index_created_by_version,
        )))
    }
}

impl TryFrom<AnyNumeric> for SortableDecimal {
    type Error = DecimalError;

    fn try_from(val: AnyNumeric) -> Result<Self, Self::Error> {
        let decimal = Decimal::from_str(val.normalize())?;
        Ok(SortableDecimal(decimal.into_bytes()))
    }
}

impl TryFrom<SortableDecimal> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(value: SortableDecimal) -> Result<Self, Self::Error> {
        // "Serialize" as hex-encoded sortable bytes.
        // Hex encoding preserves lexicographic ordering since:
        // - Each byte maps to exactly 2 hex chars
        // - Hex chars compare in the same order as byte values
        let hex_str = bytes_to_hex(&value.0);
        Ok(TantivyValue(PdbOwnedValue::Str(hex_str)))
    }
}

impl TantivyValue {
    /// Convert a NUMRANGE datum for writing into the index identified by
    /// `index_created_by_version`, so its bounds use that index's byte layout.
    pub unsafe fn try_from_numrange(
        datum: Datum,
        index_created_by_version: Option<Version>,
    ) -> Result<Self, TantivyValueError> {
        let range = unsafe { pgrx::Range::<AnyNumeric>::from_datum(datum, false) }
            .ok_or(TantivyValueError::DatumDeref)?;
        Self::from_range_with(range, |n| {
            SortableDecimal::for_index(n, index_created_by_version).map_err(|e| {
                TantivyValueError::NumericConversion(format!(
                    "Failed to convert NUMRANGE bound to bytes: {e:?}"
                ))
            })
        })
    }
}

pub(crate) trait RangeToTantivyValue<T, S>
where
    T: pgrx::datum::RangeSubType,
    S: TryFrom<T> + Clone,
    <S as TryFrom<T>>::Error: std::fmt::Debug,
    TantivyValue: TryFrom<S, Error = TantivyValueError>,
{
    fn from_range(val: pgrx::Range<T>) -> Result<TantivyValue, TantivyValueError> {
        Self::from_range_with(val, |v| Ok(S::try_from(v).unwrap()))
    }

    fn from_range_with(
        val: pgrx::Range<T>,
        convert: impl Fn(T) -> Result<S, TantivyValueError>,
    ) -> Result<TantivyValue, TantivyValueError> {
        match val.is_empty() {
            true => Ok(<TantivyValue as TryFrom<TantivyRange<S>>>::try_from(
                TantivyRangeBuilder::<S>::new().empty(true).build(),
            )?),
            false => {
                let lower = match val.lower() {
                    Some(RangeBound::Inclusive(val)) => Some(convert(val.clone())?),
                    Some(RangeBound::Exclusive(val)) => Some(convert(val.clone())?),
                    Some(RangeBound::Infinite) | None => None,
                };
                let upper = match val.upper() {
                    Some(RangeBound::Inclusive(val)) => Some(convert(val.clone())?),
                    Some(RangeBound::Exclusive(val)) => Some(convert(val.clone())?),
                    Some(RangeBound::Infinite) | None => None,
                };

                let lower_inclusive = matches!(val.lower(), Some(RangeBound::Inclusive(_)));
                let upper_inclusive = matches!(val.upper(), Some(RangeBound::Inclusive(_)));
                let lower_unbounded = matches!(val.lower(), Some(RangeBound::Infinite) | None);
                let upper_unbounded = matches!(val.upper(), Some(RangeBound::Infinite) | None);

                Ok(<TantivyValue as TryFrom<TantivyRange<S>>>::try_from(
                    TantivyRangeBuilder::new()
                        .lower(lower)
                        .upper(upper)
                        .lower_inclusive(lower_inclusive)
                        .upper_inclusive(upper_inclusive)
                        .lower_unbounded(lower_unbounded)
                        .upper_unbounded(upper_unbounded)
                        .build(),
                )?)
            }
        }
    }
}

impl RangeToTantivyValue<i32, i32> for TantivyValue {}
impl RangeToTantivyValue<i64, i64> for TantivyValue {}
// numrange uses SortableDecimal which serializes as hex-encoded lexicographically sortable bytes.
// This preserves full NUMERIC precision while allowing string comparison to give correct ordering.
impl RangeToTantivyValue<pgrx::AnyNumeric, SortableDecimal> for TantivyValue {}
impl RangeToTantivyValue<Date, Date> for TantivyValue {}
impl RangeToTantivyValue<Timestamp, Timestamp> for TantivyValue {}
impl RangeToTantivyValue<TimestampWithTimeZone, TimestampWithTimeZone> for TantivyValue {}
