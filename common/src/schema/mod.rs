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

pub mod anyenum;
pub mod config;
pub mod range;

use crate::api::tokenizers::{Typmod, type_is_alias, type_is_tokenizer};
use crate::api::version::{Version, VersionInfo};
use crate::postgres::catalog::{is_citext_oid, is_ltree_oid, is_pgvector_oid};
use crate::postgres::types::is_datetime_type;
pub use crate::postgres::utils::{FieldSource, convert_pg_date_string};
use crate::postgres::utils::{extract_numeric_precision_scale, resolve_base_type};
use crate::vector::metric::VectorMetric;
pub use anyenum::AnyEnum;
pub use config::*;
use decimal_bytes::{MAX_DECIMAL64_NO_SCALE_PRECISION, MAX_DECIMAL64_NO_SCALE_SCALE};
use pgrx::{PgBuiltInOids, PgOid, pg_sys};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokenizers::SearchNormalizer;

/// The type of the search field.
/// Like Tantivy's [`FieldType`](https://docs.rs/tantivy/latest/tantivy/schema/enum.FieldType.html),
/// but with the Postgres Oid of the column that the field is based on.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchFieldType {
    Text(pg_sys::Oid),
    Tokenized(pg_sys::Oid, Typmod, pg_sys::Oid),
    Uuid(pg_sys::Oid),
    Inet(pg_sys::Oid),
    Ltree(pg_sys::Oid),
    I64(pg_sys::Oid),
    F64(pg_sys::Oid),
    U64(pg_sys::Oid),
    Bool(pg_sys::Oid),
    Json(pg_sys::Oid),
    Date(pg_sys::Oid),
    Range(pg_sys::Oid),
    /// NUMERIC with precision <= 18 and a scale in -18..=18: stored as I64 with fixed-point
    /// scaling. The i16 is the scale (number of decimal places).
    Numeric64(pg_sys::Oid, i16),
    /// Any other NUMERIC, including unlimited precision: stored as lexicographically sortable
    /// bytes. The `Option<i16>` is the scale (number of decimal places), or None for unlimited
    /// precision.
    NumericBytes(pg_sys::Oid, Option<i16>),
    /// Dense vector field (pgvector type). The usize is the number of dimensions,
    /// and `VectorMetric` is the distance metric (default L2).
    Vector(pg_sys::Oid, usize, VectorMetric),
}

impl SearchFieldType {
    pub fn default_config(&self) -> SearchFieldConfig {
        match self {
            SearchFieldType::Text(oid) => {
                if is_citext_oid(*oid) {
                    let mut cfg = SearchFieldConfig::default_text();
                    cfg.set_normalizer(Some(SearchNormalizer::Lowercase));
                    cfg
                } else {
                    SearchFieldConfig::default_text()
                }
            }
            SearchFieldType::Tokenized(..) => {
                // NB:  check `search_field_config_from_type` to make sure the tokenizer is properly represented
                panic!("CustomText fields do not have a default config")
            }
            SearchFieldType::Uuid(_) => SearchFieldConfig::default_uuid(),
            SearchFieldType::Inet(_) => SearchFieldConfig::default_inet(),
            SearchFieldType::Ltree(_) => SearchFieldConfig::default_ltree(),
            SearchFieldType::I64(_) => SearchFieldConfig::default_numeric(),
            SearchFieldType::F64(_) => SearchFieldConfig::default_numeric(),
            SearchFieldType::U64(_) => SearchFieldConfig::default_numeric(),
            SearchFieldType::Numeric64(_, scale) => SearchFieldConfig::default_numeric64(*scale),
            SearchFieldType::NumericBytes(_, scale) => {
                SearchFieldConfig::default_numeric_bytes(*scale)
            }
            SearchFieldType::Bool(_) => SearchFieldConfig::default_boolean(),
            SearchFieldType::Json(_) => SearchFieldConfig::default_json(),
            SearchFieldType::Date(_) => SearchFieldConfig::default_date(),
            SearchFieldType::Range(_) => SearchFieldConfig::default_range(),
            SearchFieldType::Vector(_, dims, _) => SearchFieldConfig::default_vector(*dims),
        }
    }

    pub fn typeoid(&self) -> PgOid {
        match self {
            SearchFieldType::Text(oid) => *oid,
            SearchFieldType::Tokenized(oid, ..) => *oid,
            SearchFieldType::Uuid(oid) => *oid,
            SearchFieldType::Inet(oid) => *oid,
            SearchFieldType::Ltree(oid) => *oid,
            SearchFieldType::I64(oid) => *oid,
            SearchFieldType::F64(oid) => *oid,
            SearchFieldType::U64(oid) => *oid,
            SearchFieldType::Bool(oid) => *oid,
            SearchFieldType::Json(oid) => *oid,
            SearchFieldType::Date(oid) => *oid,
            SearchFieldType::Range(oid) => *oid,
            SearchFieldType::Numeric64(oid, _) => *oid,
            SearchFieldType::NumericBytes(oid, _) => *oid,
            SearchFieldType::Vector(oid, _, _) => *oid,
        }
        .into()
    }

    pub fn typmod(&self) -> Typmod {
        match self {
            SearchFieldType::Tokenized(_, typmod, ..) => *typmod,
            _ => -1,
        }
    }

    /// Returns the scale for Numeric64/NumericBytes fields, or None for other types.
    pub fn numeric_scale(&self) -> Option<i16> {
        match self {
            SearchFieldType::Numeric64(_, scale) => Some(*scale),
            SearchFieldType::NumericBytes(_, scale) => *scale,
            _ => None,
        }
    }

    /// Returns true if this is a NUMERIC type (either Numeric64 or NumericBytes).
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            SearchFieldType::Numeric64(..) | SearchFieldType::NumericBytes(..)
        )
    }

    pub fn supports_top_hits_sort(&self) -> bool {
        matches!(
            self,
            SearchFieldType::I64(_)
                | SearchFieldType::U64(_)
                | SearchFieldType::F64(_)
                | SearchFieldType::Date(_)
                | SearchFieldType::Numeric64(..)
        )
    }

    pub fn arrow_data_type(&self) -> arrow_schema::DataType {
        match self {
            SearchFieldType::Text(_)
            | SearchFieldType::Tokenized(..)
            | SearchFieldType::Uuid(_)
            | SearchFieldType::Inet(_)
            | SearchFieldType::Ltree(_)
            | SearchFieldType::Json(_)
            | SearchFieldType::Range(_) => arrow_schema::DataType::Utf8View,

            SearchFieldType::I64(oid) if is_datetime_type(*oid) => {
                arrow_schema::DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, None)
            }
            SearchFieldType::I64(_) => arrow_schema::DataType::Int64,
            SearchFieldType::U64(_) => arrow_schema::DataType::UInt64,

            SearchFieldType::F64(_) => arrow_schema::DataType::Float64,

            SearchFieldType::Bool(_) => arrow_schema::DataType::Boolean,

            SearchFieldType::Date(_) => {
                arrow_schema::DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, None)
            }

            SearchFieldType::Numeric64(..) => arrow_schema::DataType::Int64,

            SearchFieldType::NumericBytes(..) => arrow_schema::DataType::BinaryView,

            SearchFieldType::Vector(..) => arrow_schema::DataType::BinaryView,
        }
    }

    pub fn is_bytes_storage(&self) -> bool {
        matches!(
            self,
            SearchFieldType::NumericBytes(..) | SearchFieldType::Vector(..)
        )
    }

    pub fn is_dictionary_storage(&self) -> bool {
        self.is_bytes_storage()
            || matches!(
                self,
                SearchFieldType::Text(_)
                    | SearchFieldType::Tokenized(..)
                    | SearchFieldType::Uuid(_)
                    | SearchFieldType::Inet(_)
                    | SearchFieldType::Ltree(_)
                    | SearchFieldType::Json(_)
                    | SearchFieldType::Range(_)
            )
    }

    pub fn try_from_type_info(
        pg_oid: PgOid,
        typmod: Typmod,
        inner_typoid: pg_sys::Oid,
        index_created_by_version: Option<Version>,
    ) -> Result<Self, SearchIndexSchemaError> {
        if matches!(
            pg_oid,
            PgOid::BuiltIn(pg_sys::BuiltinOid::JSONBARRAYOID | pg_sys::BuiltinOid::JSONARRAYOID)
        ) {
            return Err(SearchIndexSchemaError::JsonArraysNotYetSupported);
        }

        let (mut base_oid, _) = resolve_base_type(pg_oid)
            .unwrap_or_else(|| pgrx::error!("Failed to resolve base type for type {:?}", pg_oid));

        if matches!(base_oid, PgOid::Custom(alias_oid) if type_is_alias(alias_oid)) {
            base_oid = resolve_base_type(PgOid::from_untagged(inner_typoid))
                .unwrap_or_else(|| {
                    pgrx::error!(
                        "Failed to resolve base type for inner type {:?}",
                        inner_typoid
                    )
                })
                .0;
        }

        match &base_oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                    Ok(SearchFieldType::Text((*builtin).into()))
                }
                PgBuiltInOids::UUIDOID => Ok(SearchFieldType::Uuid((*builtin).into())),
                PgBuiltInOids::INETOID => Ok(SearchFieldType::Inet((*builtin).into())),
                PgBuiltInOids::INT2OID | PgBuiltInOids::INT4OID | PgBuiltInOids::INT8OID => {
                    Ok(SearchFieldType::I64((*builtin).into()))
                }
                PgBuiltInOids::OIDOID | PgBuiltInOids::XIDOID => {
                    Ok(SearchFieldType::U64((*builtin).into()))
                }
                PgBuiltInOids::FLOAT4OID | PgBuiltInOids::FLOAT8OID => {
                    Ok(SearchFieldType::F64((*builtin).into()))
                }
                PgBuiltInOids::NUMERICOID => {
                    let (precision, scale) = extract_numeric_precision_scale(typmod);
                    if let Some(scale) = scale
                        && precision > 0
                        && precision <= MAX_DECIMAL64_NO_SCALE_PRECISION as u16
                        && i32::from(scale).abs() <= MAX_DECIMAL64_NO_SCALE_SCALE
                    {
                        return Ok(SearchFieldType::Numeric64((*builtin).into(), scale));
                    }
                    Ok(SearchFieldType::NumericBytes((*builtin).into(), scale))
                }
                PgBuiltInOids::BOOLOID => Ok(SearchFieldType::Bool((*builtin).into())),
                PgBuiltInOids::JSONOID | PgBuiltInOids::JSONBOID => {
                    Ok(SearchFieldType::Json((*builtin).into()))
                }
                PgBuiltInOids::INT4RANGEOID
                | PgBuiltInOids::INT8RANGEOID
                | PgBuiltInOids::NUMRANGEOID
                | PgBuiltInOids::DATERANGEOID
                | PgBuiltInOids::TSRANGEOID
                | PgBuiltInOids::TSTZRANGEOID => Ok(SearchFieldType::Range((*builtin).into())),
                PgBuiltInOids::TIMESTAMPOID
                | PgBuiltInOids::TIMESTAMPTZOID
                | PgBuiltInOids::DATEOID
                | PgBuiltInOids::TIMEOID
                | PgBuiltInOids::TIMETZOID => {
                    if index_created_by_version.stores_datetimes_in_i64() {
                        Ok(SearchFieldType::I64((*builtin).into()))
                    } else {
                        Ok(SearchFieldType::Date((*builtin).into()))
                    }
                }
                _ => Err(SearchIndexSchemaError::InvalidPgOid(pg_oid)),
            },
            PgOid::Custom(custom) if unsafe { pgrx::pg_sys::type_is_enum(*custom) } => {
                Ok(SearchFieldType::F64(*custom))
            }

            PgOid::Custom(tokenizer_oid) if type_is_tokenizer(*tokenizer_oid) => Ok(
                SearchFieldType::Tokenized(*tokenizer_oid, typmod, inner_typoid),
            ),

            PgOid::Custom(custom) if is_pgvector_oid(*custom) => {
                let dims = if typmod > 0 { typmod as usize } else { 0 };
                Ok(SearchFieldType::Vector(
                    *custom,
                    dims,
                    VectorMetric::default(),
                ))
            }

            PgOid::Custom(custom) => {
                if is_citext_oid(*custom) {
                    Ok(SearchFieldType::Text(*custom))
                } else if is_ltree_oid(*custom) {
                    Ok(SearchFieldType::Ltree(*custom))
                } else {
                    Err(SearchIndexSchemaError::InvalidPgOid(pg_oid))
                }
            }

            _ => Err(SearchIndexSchemaError::InvalidPgOid(pg_oid)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CategorizedFieldData {
    pub attno: usize,
    pub source: FieldSource,
    pub pg_type: PgOid,  // Original PostgreSQL type OID (e.g., pdb.alias)
    pub base_oid: PgOid, // Resolved base type OID (e.g., integer)
    pub is_array: bool,
    pub is_json: bool,
}

#[derive(Debug, Error)]
pub enum SearchIndexSchemaError {
    #[error("invalid postgres oid passed to search index schema: {0:?}")]
    InvalidPgOid(PgOid),

    #[error("json(b) arrays are not yet supported")]
    JsonArraysNotYetSupported,
}
