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

use std::net::Ipv6Addr;

use serde::ser::SerializeMap;
use tantivy::schema::{Facet, OwnedValue};

use crate::api::version::{Version, VersionInfo};
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::types::TantivyValueError;

pub const PDB_DATE_TAG: &str = "$__pdb_date__";

/// This is a "reimplementation" of tantivy's OwnedValue. We need our own because we represent dates
/// differently (as microseconds from PgEpoch, wheraas tantivy uses nanoseconds from the unix epoch)
/// Other than the Date variant, this is equivalent to Tantivy's OwnedValue
#[derive(Clone, Debug, PartialEq)]
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

impl PdbOwnedValue {
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
                if object.len() == 1 {
                    if let (PDB_DATE_TAG, OwnedValue::Str(s)) = (object[0].0.as_str(), &object[0].1)
                    {
                        // Strings that parse as a datetime must be assumed to be datetimes
                        if let Ok(pgdt) = PostgresDateTime::try_from(s.as_str()) {
                            return PdbOwnedValue::Date(pgdt);
                        }
                    }
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
        }
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
