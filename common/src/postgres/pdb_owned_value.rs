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

#![allow(dead_code)]

use std::hash::{Hash, Hasher};
use std::net::Ipv6Addr;

use arrow_schema::DataType;
use serde::ser::SerializeMap;
use tantivy::schema::{Facet, OwnedValue};

use crate::api::version::{Version, VersionInfo};
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::types::TantivyValueError;
use crate::schema::SearchFieldType;

pub const PDB_DATE_TAG: &str = "$__pdb_date__";
pub const PDB_FLOAT_TAG: &str = "$__pdb_float__";

/// This is a "reimplementation" of tantivy's OwnedValue. We need our own because we represent dates
/// differently (as microseconds from PgEpoch, wheraas tantivy uses nanoseconds from the unix epoch)
/// Other than the Date variant, this is equivalent to Tantivy's OwnedValue
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum PdbOwnedValue {
    Null,
    Str(String),
    PreTokStr(tantivy::tokenizer::PreTokenizedString),
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
    Date(PostgresDateTime),
    Facet(Facet),
    Bytes(Vec<u8>),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
    IpAddr(Ipv6Addr),
}

impl Eq for PdbOwnedValue {}

/// Agrees with the derived `PartialEq`: a float hashes by value, so `0.0` and
/// `-0.0` collide the way they compare equal, and pre-tokenized text hashes by
/// its text alone, a coarser key than the equality that also compares tokens.
impl Hash for PdbOwnedValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            PdbOwnedValue::Null => {}
            PdbOwnedValue::Str(v) => v.hash(state),
            PdbOwnedValue::PreTokStr(v) => v.text.hash(state),
            PdbOwnedValue::U64(v) => v.hash(state),
            PdbOwnedValue::I64(v) => v.hash(state),
            PdbOwnedValue::F64(v) => (if *v == 0.0 { 0.0f64 } else { *v }).to_bits().hash(state),
            PdbOwnedValue::Bool(v) => v.hash(state),
            PdbOwnedValue::Date(v) => v.hash(state),
            PdbOwnedValue::Facet(v) => v.hash(state),
            PdbOwnedValue::Bytes(v) => v.hash(state),
            PdbOwnedValue::Array(v) => v.hash(state),
            PdbOwnedValue::Object(v) => v.hash(state),
            PdbOwnedValue::IpAddr(v) => v.hash(state),
        }
    }
}

impl PdbOwnedValue {
    /// A total order for sorting and range routing. NULL sorts first. Floats use the same
    /// monotonic `u64` mapping as `TopK` and `RangeQuery`, so a boundary orders the way the scans
    /// that read it do, `NaN` included. A merged two-side join sample can tag one integer column
    /// `I64` on one side and `U64` on the other, so those compare by value. Everything else
    /// follows the derived `PartialOrd`; a column never mixes an integer with a float, so that
    /// pairing does not arise (an `assert!` guards it).
    pub fn total_cmp(&self, other: &Self) -> std::cmp::Ordering {
        use tantivy::columnar::MonotonicallyMappableToU64;
        match (self, other) {
            (Self::F64(a), Self::F64(b)) => a.to_u64().cmp(&b.to_u64()),
            (Self::I64(a), Self::U64(b)) => (*a as i128).cmp(&(*b as i128)),
            (Self::U64(a), Self::I64(b)) => (*a as i128).cmp(&(*b as i128)),
            _ => {
                // The derived order ranks by variant, not value, so a float against an integer
                // would sort by declaration order. That pairing can't reach here, so catch it
                // rather than rank it wrong in silence.
                assert!(
                    !matches!(
                        (self, other),
                        (Self::F64(_), Self::I64(_) | Self::U64(_))
                            | (Self::I64(_) | Self::U64(_), Self::F64(_))
                    ),
                    "total_cmp got a float against an integer: {self:?} vs {other:?}"
                );
                self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
            }
        }
    }

    pub fn into_tantivy_value(self, index_created_by_version: Option<Version>) -> OwnedValue {
        match self {
            PdbOwnedValue::Date(date) => {
                if index_created_by_version.stores_datetimes_in_i64() {
                    OwnedValue::I64(date.into_inner())
                } else {
                    OwnedValue::Date(
                        date.try_into()
                            .expect("legacy timestamps should always fit into tantivy's DateTime"),
                    )
                }
            }
            PdbOwnedValue::Array(array) => OwnedValue::Array(
                array
                    .into_iter()
                    .map(|v| Self::into_tantivy_value(v, index_created_by_version))
                    .collect(),
            ),
            PdbOwnedValue::Object(object) => OwnedValue::Object(
                object
                    .into_iter()
                    .map(|(k, v)| (k, Self::into_tantivy_value(v, index_created_by_version)))
                    .collect(),
            ),
            _ => Self::into_tantivy_value_no_version_awareness(self),
        }
    }

    /// Convert variants that don't require knowledge of the `index_created_by_version`.
    /// Useful for locations where Date/Array/Object are handled separately.
    pub fn into_tantivy_value_no_version_awareness(self) -> OwnedValue {
        match self {
            PdbOwnedValue::Null => OwnedValue::Null,
            PdbOwnedValue::Str(val) => OwnedValue::Str(val),
            PdbOwnedValue::PreTokStr(val) => OwnedValue::PreTokStr(val),
            PdbOwnedValue::U64(val) => OwnedValue::U64(val),
            PdbOwnedValue::I64(val) => OwnedValue::I64(val),
            PdbOwnedValue::F64(val) => OwnedValue::F64(val),
            PdbOwnedValue::Bool(val) => OwnedValue::Bool(val),
            PdbOwnedValue::Facet(val) => OwnedValue::Facet(val),
            PdbOwnedValue::Bytes(val) => OwnedValue::Bytes(val),
            PdbOwnedValue::IpAddr(val) => OwnedValue::IpAddr(val),
            PdbOwnedValue::Date(_) | PdbOwnedValue::Array(_) | PdbOwnedValue::Object(_) => {
                unreachable!(
                    "This should be handled by PdbOwnedValue::into_tantivy_value or elsewhere"
                )
            }
        }
    }

    /// This is intended only for use during deserialization to a PdbOwnedValue
    fn from_deserialized_tantivy_value(tv: OwnedValue) -> Self {
        match tv {
            OwnedValue::Date(_) => unreachable!(
                "We serialize PdbOwnedValue::Date as a string, so this should never happen"
            ),
            OwnedValue::Array(array) => PdbOwnedValue::Array(
                array
                    .into_iter()
                    .map(Self::from_deserialized_tantivy_value)
                    .collect(),
            ),
            OwnedValue::Object(object) => {
                // serialized Date's end up as objects, so we'll need to look for their shape here.
                if object.len() == 1
                    && let (PDB_DATE_TAG, OwnedValue::Str(s)) = (object[0].0.as_str(), &object[0].1)
                {
                    // Strings that parse as a datetime must be assumed to be datetimes
                    if let Ok(pgdt) = PostgresDateTime::try_from_serialized(s.as_str()) {
                        return PdbOwnedValue::Date(pgdt);
                    }
                }

                // So do NaN and the infinities, which JSON has no form for.
                if object.len() == 1
                    && let (PDB_FLOAT_TAG, OwnedValue::Str(s)) =
                        (object[0].0.as_str(), &object[0].1)
                    && let Some(value) = s.parse::<f64>().ok().filter(|value| !value.is_finite())
                {
                    return PdbOwnedValue::F64(value);
                }

                PdbOwnedValue::Object(
                    object
                        .into_iter()
                        .map(|(k, v)| (k, Self::from_deserialized_tantivy_value(v)))
                        .collect(),
                )
            }
            OwnedValue::Null => PdbOwnedValue::Null,
            OwnedValue::I64(val) => PdbOwnedValue::I64(val),
            OwnedValue::U64(val) => PdbOwnedValue::U64(val),
            OwnedValue::F64(val) => PdbOwnedValue::F64(val),
            // User-supplied json can have timestamps as strings. We need to attempt to parse them
            // here so we can correctly convert them to PdbOwnedValue::Date
            OwnedValue::Str(s) => PdbOwnedValue::Str(s),
            OwnedValue::Bool(val) => PdbOwnedValue::Bool(val),
            OwnedValue::Facet(val) => PdbOwnedValue::Facet(val),
            OwnedValue::Bytes(val) => PdbOwnedValue::Bytes(val),
            OwnedValue::IpAddr(val) => PdbOwnedValue::IpAddr(val),
            OwnedValue::PreTokStr(val) => PdbOwnedValue::PreTokStr(val),
            OwnedValue::Custom(_) => unreachable!(
                "custom values are plugin-owned and never serialized, so this should never happen"
            ),
        }
    }

    /// Returns `true` if every non-null `ScalarValue` of Arrow `data_type` is supported by
    /// `from_execution_scalar` for `field_type`.
    pub fn supports_execution_data_type(
        data_type: &DataType,
        field_type: &SearchFieldType,
    ) -> bool {
        matches!(
            (data_type, field_type),
            (
                DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64,
                SearchFieldType::I64(_)
            ) | (
                DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64,
                SearchFieldType::U64(_)
            ) | (
                DataType::Float32 | DataType::Float64 | DataType::Int32 | DataType::Int64,
                SearchFieldType::F64(_)
            ) | (DataType::Boolean, SearchFieldType::Bool(_))
                | (
                    DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View,
                    SearchFieldType::Text(_)
                        | SearchFieldType::Tokenized(..)
                        | SearchFieldType::Uuid(_)
                )
                | (
                    DataType::Int64 | DataType::Float64,
                    SearchFieldType::Numeric64(_, _)
                )
        )
    }
}

impl serde::Serialize for PdbOwnedValue {
    /// For variants that can be cleanly converted to tantivy's OwnedValue, just do that and use its
    /// serialize.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            // tag dates so that we can identify them when deserializing
            PdbOwnedValue::Date(date) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(PDB_DATE_TAG, &date)?;
                map.end()
            }
            // tag NaN and the infinities too, since JSON would write them as `null`
            PdbOwnedValue::F64(value) if !value.is_finite() => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(PDB_FLOAT_TAG, &value.to_string())?;
                map.end()
            }
            PdbOwnedValue::Object(ref obj) => {
                let mut map = serializer.serialize_map(Some(obj.len()))?;
                for (k, v) in obj {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            PdbOwnedValue::Array(ref array) => array.serialize(serializer),
            _ => self
                .clone()
                .into_tantivy_value_no_version_awareness()
                .serialize(serializer),
        }
    }
}

impl<'de> serde::Deserialize<'de> for PdbOwnedValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let tantivy_value = OwnedValue::deserialize(deserializer)?;
        Ok(PdbOwnedValue::from_deserialized_tantivy_value(
            tantivy_value,
        ))
    }
}

/// The exact, lossless serde form for the scalar variants of [`PdbOwnedValue`], for callers that
/// must reconstruct a value bit for bit (partition boundaries shipped over the DSM wire). The
/// `Serialize`/`Deserialize` impls above route through tantivy's `OwnedValue`, which is lossy: a
/// non-negative `I64` comes back as `U64`, and a NaN loses its bit pattern.
///
/// Scalar values only. It errors on `Null` and the composite variants (`Array`, `Object`,
/// `Facet`, `PreTokStr`), which never appear as partition split values. `F64` travels as its
/// bits and dates as their raw microseconds, so both survive any codec without calling into
/// Postgres. Use through `#[serde(with = "...")]` on a `PdbOwnedValue` field.
pub mod exact_scalar_wire {
    use std::net::Ipv6Addr;

    use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser};

    use super::PdbOwnedValue;
    use crate::postgres::datetime::PostgresDateTime;

    #[derive(Serialize, Deserialize)]
    enum WireValue {
        Str(String),
        U64(u64),
        I64(i64),
        Bool(bool),
        Date(i64),
        // The wire needs the exact value back, not an order, so it carries `f64::to_bits`
        // (not the ordering `u64` mapping `total_cmp` uses). `NaN` and the infinities survive.
        F64Bits(u64),
        Bytes(Vec<u8>),
        IpAddr(Ipv6Addr),
    }

    pub fn serialize<S: Serializer>(value: &PdbOwnedValue, s: S) -> Result<S::Ok, S::Error> {
        let wire = match value {
            PdbOwnedValue::Str(v) => WireValue::Str(v.clone()),
            PdbOwnedValue::U64(v) => WireValue::U64(*v),
            PdbOwnedValue::I64(v) => WireValue::I64(*v),
            PdbOwnedValue::F64(v) => WireValue::F64Bits(v.to_bits()),
            PdbOwnedValue::Bool(v) => WireValue::Bool(*v),
            PdbOwnedValue::Date(v) => WireValue::Date(v.into_inner()),
            PdbOwnedValue::Bytes(v) => WireValue::Bytes(v.clone()),
            PdbOwnedValue::IpAddr(v) => WireValue::IpAddr(*v),
            other => {
                return Err(ser::Error::custom(format!(
                    "unsupported partition split value: {other:?}"
                )));
            }
        };
        wire.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PdbOwnedValue, D::Error> {
        Ok(match WireValue::deserialize(d)? {
            WireValue::Str(v) => PdbOwnedValue::Str(v),
            WireValue::U64(v) => PdbOwnedValue::U64(v),
            WireValue::I64(v) => PdbOwnedValue::I64(v),
            WireValue::F64Bits(v) => PdbOwnedValue::F64(f64::from_bits(v)),
            WireValue::Bool(v) => PdbOwnedValue::Bool(v),
            WireValue::Date(v) => PdbOwnedValue::Date(
                PostgresDateTime::try_from_raw(v)
                    .map_err(|e| de::Error::custom(format!("invalid split date: {e:?}")))?,
            ),
            WireValue::Bytes(v) => PdbOwnedValue::Bytes(v),
            WireValue::IpAddr(v) => PdbOwnedValue::IpAddr(v),
        })
    }
}

impl PdbOwnedValue {
    /// A `Display` that renders this value for logs without calling into Postgres, so it works
    /// outside a backend. The complementary `TantivyValue` `Display` routes dates through
    /// Postgres output functions; this one renders dates as UTC RFC 3339 via `chrono` instead.
    /// Strings are quoted so a value reads unambiguously in a log line, and the composite and
    /// unsupported variants fall back to `Debug`.
    pub fn plain_display(&self) -> impl std::fmt::Display + '_ {
        PlainDisplay(self)
    }
}

struct PlainDisplay<'a>(&'a PdbOwnedValue);

impl std::fmt::Display for PlainDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            PdbOwnedValue::Str(v) => write!(f, "{v:?}"),
            PdbOwnedValue::U64(v) => write!(f, "{v}"),
            PdbOwnedValue::I64(v) => write!(f, "{v}"),
            PdbOwnedValue::F64(v) => write!(f, "{v}"),
            PdbOwnedValue::Bool(v) => write!(f, "{v}"),
            PdbOwnedValue::IpAddr(v) => write!(f, "{v}"),
            PdbOwnedValue::Date(v) => match v.to_rfc3339() {
                Some(s) => write!(f, "{s}"),
                None => write!(f, "{v:?}"),
            },
            other => write!(f, "{other:?}"),
        }
    }
}

impl From<String> for PdbOwnedValue {
    fn from(value: String) -> Self {
        Self::Str(value)
    }
}
impl From<i64> for PdbOwnedValue {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}
impl From<u64> for PdbOwnedValue {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}
impl From<f64> for PdbOwnedValue {
    fn from(value: f64) -> Self {
        Self::F64(value)
    }
}
impl From<bool> for PdbOwnedValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl From<tantivy::DateTime> for PdbOwnedValue {
    fn from(value: tantivy::DateTime) -> Self {
        let pg_dt = PostgresDateTime::try_from(value)
            .expect("We should never see a timestamp that postgres can't represent");
        Self::Date(pg_dt)
    }
}

impl TryFrom<serde_json::Value> for PdbOwnedValue {
    type Error = TantivyValueError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let pdb_value: PdbOwnedValue = serde_json::from_value(value)?;
        Ok(pdb_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;
    use std::net::Ipv6Addr;

    #[test]
    fn total_cmp_orders_across_numeric_variants() {
        // NULL sorts first.
        assert_eq!(
            PdbOwnedValue::Null.total_cmp(&PdbOwnedValue::I64(0)),
            Ordering::Less
        );
        // `I64`/`U64` compare by value, not by variant.
        assert_eq!(
            PdbOwnedValue::I64(-1).total_cmp(&PdbOwnedValue::U64(0)),
            Ordering::Less
        );
        assert_eq!(
            PdbOwnedValue::U64(5).total_cmp(&PdbOwnedValue::I64(5)),
            Ordering::Equal
        );
        assert_eq!(
            PdbOwnedValue::I64(10).total_cmp(&PdbOwnedValue::U64(3)),
            Ordering::Greater
        );
        // Floats order by the monotonic mapping: -inf < finite < +inf < NaN, with a fixed
        // place for NaN (no panic, no `Equal` surprise).
        let ordered = [
            PdbOwnedValue::F64(f64::NEG_INFINITY),
            PdbOwnedValue::F64(-1.0),
            PdbOwnedValue::F64(0.0),
            PdbOwnedValue::F64(1.0),
            PdbOwnedValue::F64(f64::INFINITY),
            PdbOwnedValue::F64(f64::NAN),
        ];
        for pair in ordered.windows(2) {
            assert_eq!(pair[0].total_cmp(&pair[1]), Ordering::Less, "{pair:?}");
        }
        // Non-numeric variants fall through to the derived order.
        assert_eq!(
            PdbOwnedValue::Str("a".into()).total_cmp(&PdbOwnedValue::Str("b".into())),
            Ordering::Less
        );
    }

    // Round-trips a value through `exact_scalar_wire` and `postcard`, the way a partition split
    // value travels over the DSM.
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wire(#[serde(with = "super::exact_scalar_wire")] PdbOwnedValue);

    fn round_trip(value: &PdbOwnedValue) -> PdbOwnedValue {
        let bytes = postcard::to_allocvec(&Wire(value.clone())).unwrap();
        postcard::from_bytes::<Wire>(&bytes).unwrap().0
    }

    #[test]
    fn exact_scalar_wire_preserves_every_scalar_variant() {
        let cases = [
            PdbOwnedValue::Str("héllo \"world\"".into()),
            PdbOwnedValue::U64(u64::MAX),
            // A non-negative `I64` must not come back as `U64` (the tantivy default serde bug).
            PdbOwnedValue::I64(1000),
            PdbOwnedValue::I64(-42),
            PdbOwnedValue::F64(3.5),
            PdbOwnedValue::F64(f64::INFINITY),
            PdbOwnedValue::F64(f64::NEG_INFINITY),
            PdbOwnedValue::Bool(true),
            PdbOwnedValue::Bytes(vec![0, 1, 255]),
            PdbOwnedValue::IpAddr(Ipv6Addr::LOCALHOST),
            PdbOwnedValue::Date(PostgresDateTime::try_from_raw(123_456).unwrap()),
        ];
        for value in &cases {
            assert_eq!(&round_trip(value), value, "did not round-trip: {value:?}");
        }
        // `PartialEq` treats `NaN != NaN`, so check the bit pattern survived instead.
        assert!(matches!(
            round_trip(&PdbOwnedValue::F64(f64::NAN)),
            PdbOwnedValue::F64(f) if f.is_nan()
        ));
    }

    #[test]
    fn exact_scalar_wire_rejects_non_scalar_variants() {
        #[derive(serde::Serialize)]
        struct W(#[serde(with = "super::exact_scalar_wire")] PdbOwnedValue);
        for value in [PdbOwnedValue::Null, PdbOwnedValue::Array(vec![])] {
            assert!(postcard::to_allocvec(&W(value)).is_err());
        }
    }

    #[test]
    fn non_finite_floats_survive_json() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let json = serde_json::to_string(&PdbOwnedValue::F64(value)).unwrap();
            assert!(json.contains(PDB_FLOAT_TAG), "not tagged: {json}");
            let PdbOwnedValue::F64(decoded) = serde_json::from_str(&json).unwrap() else {
                panic!("did not decode as a float: {json}");
            };
            assert_eq!(decoded.to_bits(), value.to_bits());
        }
    }

    #[test]
    fn finite_floats_stay_plain_json_numbers() {
        assert_eq!(
            serde_json::to_string(&PdbOwnedValue::F64(1.5)).unwrap(),
            "1.5"
        );
    }

    #[test]
    fn float_tag_only_decodes_non_finite_values() {
        for text in ["1.5", "not a float"] {
            let value: PdbOwnedValue =
                serde_json::from_value(serde_json::json!({ PDB_FLOAT_TAG: text })).unwrap();
            assert!(matches!(value, PdbOwnedValue::Object(_)), "{value:?}");
        }
    }

    #[test]
    fn plain_display_renders_without_a_backend() {
        assert_eq!(
            PdbOwnedValue::Str("x\"y".into())
                .plain_display()
                .to_string(),
            "\"x\\\"y\""
        );
        assert_eq!(PdbOwnedValue::I64(-7).plain_display().to_string(), "-7");
        assert_eq!(PdbOwnedValue::U64(7).plain_display().to_string(), "7");
        assert_eq!(
            PdbOwnedValue::Bool(false).plain_display().to_string(),
            "false"
        );
        assert_eq!(PdbOwnedValue::F64(1.5).plain_display().to_string(), "1.5");
        assert_eq!(
            PdbOwnedValue::IpAddr(Ipv6Addr::LOCALHOST)
                .plain_display()
                .to_string(),
            "::1"
        );
        // A date at the Postgres epoch renders as RFC 3339 UTC.
        let epoch = PostgresDateTime::try_from_raw(0).unwrap();
        assert_eq!(
            PdbOwnedValue::Date(epoch).plain_display().to_string(),
            "2000-01-01T00:00:00+00:00"
        );
        // NULL and composites fall back to `Debug` rather than panic.
        assert_eq!(PdbOwnedValue::Null.plain_display().to_string(), "Null");
        assert!(
            !PdbOwnedValue::Object(vec![])
                .plain_display()
                .to_string()
                .is_empty()
        );
    }
}
