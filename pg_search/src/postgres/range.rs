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

use crate::postgres::types::{PdbOwnedValue, TantivyValue, TantivyValueError};
use crate::query::numeric::bytes_to_hex;
use crate::schema::range::{TantivyRange, TantivyRangeBuilder};
use decimal_bytes::Decimal;
use pgrx::datum::{Date, RangeBound, Timestamp, TimestampWithTimeZone};
use std::str::FromStr;

/// A wrapper around `Decimal` that serializes to hex-encoded lexicographically sortable bytes.
/// This allows string comparison in Tantivy's JSON fields to give correct numeric ordering.
///
/// Used for `numrange` to preserve full NUMERIC precision in range queries.
#[derive(Clone, Debug)]
pub(crate) struct SortableDecimal(pub Decimal);

impl TryFrom<pgrx::AnyNumeric> for SortableDecimal {
    type Error = decimal_bytes::DecimalError;

    fn try_from(val: pgrx::AnyNumeric) -> Result<Self, Self::Error> {
        let numeric_str = val.normalize().to_string();
        let decimal = Decimal::from_str(&numeric_str)?;
        Ok(SortableDecimal(decimal))
    }
}

impl TryFrom<SortableDecimal> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(value: SortableDecimal) -> Result<Self, Self::Error> {
        // "Serialize" as hex-encoded sortable bytes.
        // Hex encoding preserves lexicographic ordering since:
        // - Each byte maps to exactly 2 hex chars
        // - Hex chars compare in the same order as byte values
        let hex_str = bytes_to_hex(value.0.as_bytes());
        Ok(TantivyValue(PdbOwnedValue::Str(hex_str)))
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
        match val.is_empty() {
            true => Ok(<TantivyValue as TryFrom<TantivyRange<S>>>::try_from(
                TantivyRangeBuilder::<S>::new().empty(true).build(),
            )?),
            false => {
                let lower = match val.lower() {
                    Some(RangeBound::Inclusive(val)) => Some(S::try_from(val.clone()).unwrap()),
                    Some(RangeBound::Exclusive(val)) => Some(S::try_from(val.clone()).unwrap()),
                    Some(RangeBound::Infinite) | None => None,
                };
                let upper = match val.upper() {
                    Some(RangeBound::Inclusive(val)) => Some(S::try_from(val.clone()).unwrap()),
                    Some(RangeBound::Exclusive(val)) => Some(S::try_from(val.clone()).unwrap()),
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
