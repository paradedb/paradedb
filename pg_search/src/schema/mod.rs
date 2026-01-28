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

mod anyenum;
mod config;
pub mod range;

use crate::api::FieldName;
use crate::api::HashMap;
use crate::postgres::options::{BM25IndexOptions, SortByDirection, SortByField};
pub use crate::postgres::utils::FieldSource;
use crate::postgres::utils::{resolve_base_type, ExtractedFieldAttribute};
pub use anyenum::AnyEnum;
use anyhow::bail;
pub use config::*;
use std::cell::{Ref, RefCell};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use tantivy::index::{IndexSortByField, Order};

use crate::api::tokenizers::{type_is_alias, type_is_tokenizer, Typmod};
use crate::index::utils::load_index_schema;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::extract_numeric_precision_scale;
use crate::query::QueryError;
use anyhow::Result;
use decimal_bytes::MAX_DECIMAL64_NO_SCALE_PRECISION;
use derive_more::Into;
use pgrx::{pg_sys, PgBuiltInOids, PgOid};
use serde::{Deserialize, Serialize};
use tantivy::schema::{Field, FieldEntry, FieldType, OwnedValue, Schema};
use thiserror::Error;
use tokenizers::manager::SearchTokenizerFilters;
use tokenizers::{SearchNormalizer, SearchTokenizer};

/// The type of the search field.
/// Like Tantivy's [`FieldType`](https://docs.rs/tantivy/latest/tantivy/schema/enum.FieldType.html),
/// but with the Postgres Oid of the column that the field is based on.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchFieldType {
    Text(pg_sys::Oid),
    Tokenized(pg_sys::Oid, Typmod, pg_sys::Oid),
    Uuid(pg_sys::Oid),
    Inet(pg_sys::Oid),
    I64(pg_sys::Oid),
    F64(pg_sys::Oid),
    U64(pg_sys::Oid),
    Bool(pg_sys::Oid),
    Json(pg_sys::Oid),
    Date(pg_sys::Oid),
    Range(pg_sys::Oid),
    /// NUMERIC with precision <= 18: stored as I64 with fixed-point scaling.
    /// The i16 is the scale (number of decimal places).
    Numeric64(pg_sys::Oid, i16),
    /// NUMERIC with precision > 18 or unlimited: stored as lexicographically sortable bytes.
    NumericBytes(pg_sys::Oid),
}

impl SearchFieldType {
    pub fn default_config(&self) -> SearchFieldConfig {
        match self {
            SearchFieldType::Text(_) => SearchFieldConfig::default_text(),
            SearchFieldType::Tokenized(..) => {
                // NB:  check `search_field_config_from_type` to make sure the tokenizer is properly represented
                panic!("CustomText fields do not have a default config")
            }
            SearchFieldType::Uuid(_) => SearchFieldConfig::default_uuid(),
            SearchFieldType::Inet(_) => SearchFieldConfig::default_inet(),
            SearchFieldType::I64(_) => SearchFieldConfig::default_numeric(),
            SearchFieldType::F64(_) => SearchFieldConfig::default_numeric(),
            SearchFieldType::U64(_) => SearchFieldConfig::default_numeric(),
            SearchFieldType::Numeric64(_, scale) => SearchFieldConfig::default_numeric64(*scale),
            SearchFieldType::NumericBytes(_) => SearchFieldConfig::default_numeric_bytes(),
            SearchFieldType::Bool(_) => SearchFieldConfig::default_boolean(),
            SearchFieldType::Json(_) => SearchFieldConfig::default_json(),
            SearchFieldType::Date(_) => SearchFieldConfig::default_date(),
            SearchFieldType::Range(_) => SearchFieldConfig::default_range(),
        }
    }

    pub fn typeoid(&self) -> PgOid {
        match self {
            SearchFieldType::Text(oid) => *oid,
            SearchFieldType::Tokenized(oid, ..) => *oid,
            SearchFieldType::Uuid(oid) => *oid,
            SearchFieldType::Inet(oid) => *oid,
            SearchFieldType::I64(oid) => *oid,
            SearchFieldType::F64(oid) => *oid,
            SearchFieldType::U64(oid) => *oid,
            SearchFieldType::Bool(oid) => *oid,
            SearchFieldType::Json(oid) => *oid,
            SearchFieldType::Date(oid) => *oid,
            SearchFieldType::Range(oid) => *oid,
            SearchFieldType::Numeric64(oid, _) => *oid,
            SearchFieldType::NumericBytes(oid) => *oid,
        }
        .into()
    }

    pub fn typmod(&self) -> Typmod {
        match self {
            SearchFieldType::Tokenized(_, typmod, ..) => *typmod,
            _ => -1,
        }
    }

    /// Returns the scale for Numeric64 fields, or None for other types.
    pub fn numeric_scale(&self) -> Option<i16> {
        match self {
            SearchFieldType::Numeric64(_, scale) => Some(*scale),
            _ => None,
        }
    }

    /// Returns true if this is a NUMERIC type (either Numeric64 or NumericBytes).
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            SearchFieldType::Numeric64(..) | SearchFieldType::NumericBytes(_)
        )
    }
}

impl TryFrom<(PgOid, Typmod, pg_sys::Oid)> for SearchFieldType {
    type Error = SearchIndexSchemaError;
    fn try_from(value: (PgOid, Typmod, pg_sys::Oid)) -> Result<Self, Self::Error> {
        let pg_oid = value.0;
        let typmod = value.1;
        let inner_typoid = value.2;

        if matches!(
            pg_oid,
            PgOid::BuiltIn(pg_sys::BuiltinOid::JSONBARRAYOID | pg_sys::BuiltinOid::JSONARRAYOID)
        ) {
            return Err(SearchIndexSchemaError::JsonArraysNotYetSupported);
        }

        let (mut base_oid, _) = resolve_base_type(pg_oid)
            .unwrap_or_else(|| pgrx::error!("Failed to resolve base type for type {:?}", pg_oid));

        if matches!(base_oid, PgOid::Custom(alias_oid) if type_is_alias(alias_oid)) {
            // For pdb.alias types, resolve the inner_typoid to get the base element type
            // This strips array information (e.g., timestamptz[] -> timestamptz)
            // which matches how non-alias array fields are handled
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
                    // Route NUMERIC based on precision:
                    // - precision <= 18 with defined scale -> Numeric64 (I64 fixed-point)
                    // - precision > 18 or unlimited -> NumericBytes (lexicographic bytes)
                    let (precision, scale) = extract_numeric_precision_scale(typmod);
                    if let Some(scale) = scale {
                        if precision > 0 && precision <= MAX_DECIMAL64_NO_SCALE_PRECISION as u16 {
                            return Ok(SearchFieldType::Numeric64((*builtin).into(), scale));
                        }
                    }
                    Ok(SearchFieldType::NumericBytes((*builtin).into()))
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
                PgBuiltInOids::DATEOID
                | PgBuiltInOids::TIMESTAMPOID
                | PgBuiltInOids::TIMESTAMPTZOID
                | PgBuiltInOids::TIMEOID
                | PgBuiltInOids::TIMETZOID => Ok(SearchFieldType::Date((*builtin).into())),
                _ => Err(SearchIndexSchemaError::InvalidPgOid(pg_oid)),
            },
            PgOid::Custom(custom) if unsafe { pgrx::pg_sys::type_is_enum(*custom) } => {
                Ok(SearchFieldType::F64(*custom))
            }

            PgOid::Custom(tokenizer_oid) if type_is_tokenizer(*tokenizer_oid) => Ok(
                SearchFieldType::Tokenized(*tokenizer_oid, typmod, inner_typoid),
            ),

            PgOid::Custom(_) => Err(SearchIndexSchemaError::InvalidPgOid(pg_oid)),

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
    pub is_key_field: bool,
    pub is_array: bool,
    pub is_json: bool,
}

#[derive(Clone, Into)]
pub struct SearchIndexSchema {
    #[into]
    schema: Schema,
    bm25_options: BM25IndexOptions,
    categorized: Rc<RefCell<Vec<(SearchField, CategorizedFieldData)>>>,
}

impl SearchIndexSchema {
    pub fn open(indexrel: &PgSearchRelation) -> tantivy::Result<Self> {
        Ok(load_index_schema(indexrel)?
            .map(|schema| Self {
                schema,
                bm25_options: indexrel.options().clone(),
                categorized: Default::default(),
            })
            .unwrap_or_else(|| Self {
                schema: Schema::builder().build(),
                bm25_options: indexrel.options().clone(),
                categorized: Default::default(),
            }))
    }

    pub fn tantivy_schema(&self) -> &Schema {
        &self.schema
    }

    pub fn ctid_field(&self) -> Field {
        self.schema
            .get_field("ctid")
            .expect("ctid field should be present in the index")
    }

    pub fn key_field_name(&self) -> FieldName {
        self.bm25_options.key_field_name()
    }

    pub fn key_field_type(&self) -> SearchFieldType {
        self.bm25_options.key_field_type()
    }

    /// Convert sort_by configuration to Tantivy's IndexSortByField.
    ///
    /// Validates that the sort field exists in the schema and is a fast field.
    /// Returns None if sort_by is empty (no segment sorting).
    ///
    /// This is an associated function (not a method) because it's also used during
    /// index creation when only the Tantivy Schema is available.
    pub fn build_sort_by_field(
        sort_by: &[SortByField],
        schema: &Schema,
    ) -> Option<IndexSortByField> {
        // Empty sort_by means no segment sorting
        if sort_by.is_empty() {
            return None;
        }

        // Multi-field validation is done in options.rs during parsing
        let sort_field = &sort_by[0];
        let field_name = sort_field.field_name.as_ref();

        // Validate field exists in schema
        let field = schema.get_field(field_name).unwrap_or_else(|_| {
            panic!(
                "sort_by field '{}' does not exist in the index schema",
                field_name
            )
        });

        // Validate field is a fast field
        let field_entry = schema.get_field_entry(field);
        if !field_entry.is_fast() {
            panic!(
                "sort_by field '{}' must be a fast field. Add it to the index with 'fast: true'",
                field_name
            );
        }

        // Convert direction
        let order = match sort_field.direction {
            SortByDirection::Asc => Order::Asc,
            SortByDirection::Desc => Order::Desc,
        };

        Some(IndexSortByField {
            field: field_name.to_string(),
            order,
        })
    }

    pub fn get_field_type(&self, name: impl AsRef<str>) -> Option<SearchFieldType> {
        self.bm25_options
            .get_field_type(&FieldName::from(name.as_ref()))
    }

    pub fn search_field(&self, name: impl AsRef<str>) -> Option<SearchField> {
        let field_name = FieldName::from(name.as_ref());
        match self.schema.get_field(&field_name.root()) {
            Ok(field) => Some(SearchField::new(field, &self.bm25_options, &self.schema)),
            Err(_) => None,
        }
    }

    pub fn fields(&self) -> impl Iterator<Item = (Field, &FieldEntry)> {
        self.schema.fields()
    }

    /// A lookup from a Postgres column name to search fields that have
    /// marked it as their source column with the 'column' key.
    pub fn alias_lookup(&self) -> HashMap<String, Vec<SearchField>> {
        let mut lookup = HashMap::default();
        let aliased_text_configs = self.bm25_options.aliased_text_configs();
        let aliased_json_configs = self.bm25_options.aliased_json_configs();

        for (alias_name, config) in aliased_text_configs {
            let alias = config
                .alias()
                .expect("aliased text config must have an alias");
            let alias_field = self
                .search_field(alias_name)
                .expect("aliased text config must have a search field");
            lookup
                .entry(alias.to_string())
                .or_insert_with(Vec::new)
                .push(alias_field);
        }

        for (alias_name, config) in aliased_json_configs {
            let alias = config
                .alias()
                .expect("aliased json config must have an alias");
            let alias_field = self
                .search_field(alias_name)
                .expect("aliased json config must have a search field");
            lookup
                .entry(alias.to_string())
                .or_insert_with(Vec::new)
                .push(alias_field);
        }

        lookup
    }

    pub fn categorized_fields(&self) -> Ref<'_, Vec<(SearchField, CategorizedFieldData)>> {
        let is_empty = self.categorized.borrow().is_empty();
        if is_empty {
            let key_field_name = self.key_field_name();
            let mut categorized = self.categorized.borrow_mut();
            let mut alias_lookup = self.alias_lookup();
            for (
                attname,
                ExtractedFieldAttribute {
                    attno,
                    source,
                    pg_type,
                    tantivy_type,
                    inner_typoid,
                    ..
                },
            ) in self.bm25_options.attributes().iter()
            {
                // List any indexed fields that use this column as source data.
                let mut search_fields = alias_lookup.remove(attname.as_ref()).unwrap_or_default();

                // If there's an indexed field with the same name as a this column, add it to the list.
                if let Some(index_field) = self.search_field(attname) {
                    search_fields.push(index_field)
                };

                for search_field in search_fields {
                    let (base_oid, is_array) = resolve_base_type(PgOid::from_untagged(
                        *inner_typoid,
                    ))
                    .unwrap_or_else(|| {
                        pgrx::error!(
                            "Failed to resolve base type for column {} with type {:?}",
                            attname,
                            tantivy_type.typeoid()
                        )
                    });
                    let is_key_field = key_field_name == *search_field.field_name();
                    let is_json = matches!(
                        base_oid,
                        PgOid::BuiltIn(pg_sys::BuiltinOid::JSONBOID | pg_sys::BuiltinOid::JSONOID)
                    );
                    categorized.push((
                        search_field,
                        CategorizedFieldData {
                            attno: *attno,
                            source: *source,
                            pg_type: *pg_type,
                            base_oid,
                            is_key_field,
                            is_array,
                            is_json,
                        },
                    ));
                }
            }
        }

        self.categorized.borrow()
    }
}

#[derive(Debug, Clone)]
pub struct SearchField {
    field: Field,
    field_name: FieldName,
    field_entry: FieldEntry,
    field_type: SearchFieldType,
    field_config: SearchFieldConfig,
}

impl Hash for SearchField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.field.hash(state);
    }
}

impl Eq for SearchField {}

impl PartialEq for SearchField {
    fn eq(&self, other: &Self) -> bool {
        self.field == other.field
    }
}

impl SearchField {
    pub fn new(field: Field, options: &BM25IndexOptions, schema: &Schema) -> Self {
        let field_entry = schema.get_field_entry(field).clone();
        let field_name: FieldName = field_entry.name().into();
        let field_config = options.field_config_or_default(&field_name);
        let field_type = options.get_field_type(&field_name).unwrap_or_else(|| {
            panic!("`{field_name}`'s configuration not found in index WITH options")
        });

        Self {
            field,
            field_name,
            field_entry,
            field_type,
            field_config,
        }
    }

    pub fn field(&self) -> Field {
        self.field
    }

    pub fn field_name(&self) -> &FieldName {
        &self.field_name
    }

    pub fn field_entry(&self) -> &FieldEntry {
        &self.field_entry
    }

    pub fn field_type(&self) -> SearchFieldType {
        self.field_type
    }

    pub fn field_config(&self) -> &SearchFieldConfig {
        &self.field_config
    }

    pub fn is_raw_sortable(&self) -> bool {
        self.is_sortable(SearchNormalizer::Raw)
    }

    pub fn is_lower_sortable(&self) -> bool {
        self.is_sortable(SearchNormalizer::Lowercase)
    }

    pub fn is_fast(&self) -> bool {
        self.field_entry.is_fast()
    }

    pub fn is_numeric_fast(&self) -> bool {
        match self.field_entry.field_type() {
            FieldType::I64(options) => options.is_fast(),
            FieldType::U64(options) => options.is_fast(),
            FieldType::F64(options) => options.is_fast(),
            FieldType::Bool(options) => options.is_fast(),
            FieldType::Date(options) => options.is_fast(),
            _ => false,
        }
    }

    fn is_sortable(&self, desired_normalizer: SearchNormalizer) -> bool {
        // NOTE: This list of supported field types must be synced with the field types which are
        // specialized (in a few spots!) in SearchIndexReader.
        match self.field_entry.field_type() {
            #[allow(deprecated)]
            FieldType::Str(options) => {
                options.is_fast()
                    && options.get_fast_field_tokenizer_name() == Some(desired_normalizer.name())
            }
            FieldType::I64(options) => options.is_fast(),
            FieldType::U64(options) => options.is_fast(),
            FieldType::F64(options) => options.is_fast(),
            FieldType::Bool(options) => options.is_fast(),
            FieldType::Date(options) => options.is_fast(),
            // TODO: Neither JSON nor range fields are not yet sortable by us
            FieldType::JsonObject(_) => false,
            _ => false,
        }
    }

    pub fn is_ctid(&self) -> bool {
        self.field_name().is_ctid()
    }

    pub fn is_datetime(&self) -> bool {
        self.field_entry.field_type().is_date()
    }

    pub fn is_text(&self) -> bool {
        self.field_entry.field_type().is_str()
    }

    pub fn with_positions(self) -> Result<Self, QueryError> {
        if self.supports_positions() {
            Ok(self)
        } else {
            let tokenizer = self
                .field_config()
                .tokenizer()
                .map(|t| t.name().to_string());

            Err(QueryError::TokenizerDoesNotSupportQueryType {
                field: self.field_name().clone(),
                tokenizer,
            })
        }
    }

    fn supports_positions(&self) -> bool {
        let tokenizer = self.field_config.tokenizer();

        // these tokenizers only emit one token, so they implicitly "support" positions
        #[allow(deprecated)]
        if matches!(
            tokenizer,
            Some(SearchTokenizer::Keyword)
                | Some(SearchTokenizer::KeywordDeprecated)
                | Some(SearchTokenizer::Raw(..))
                | Some(SearchTokenizer::LiteralNormalized(..))
        ) {
            return true;
        }

        let has_positions = self
            .field_entry
            .field_type()
            .get_index_record_option()
            .map(|opt| opt.has_positions())
            .unwrap_or(false);

        let ngram_supports_positions = match tokenizer {
            Some(SearchTokenizer::Ngram {
                min_gram,
                max_gram,
                positions: true,
                ..
            }) => min_gram == max_gram,
            Some(SearchTokenizer::Ngram { .. }) => false,
            _ => true,
        };

        (self.is_text() || self.is_json()) && has_positions && ngram_supports_positions
    }

    pub fn is_json(&self) -> bool {
        self.field_entry.field_type().is_json()
    }

    #[allow(deprecated)]
    pub fn is_keyword(&self) -> bool {
        self.field_config
            .tokenizer()
            .map(|tokenizer| {
                (*tokenizer == SearchTokenizer::Keyword)
                    || (*tokenizer
                        == SearchTokenizer::Raw(SearchTokenizerFilters::keyword().clone()))
            })
            .unwrap_or(false)
    }

    #[allow(deprecated)]
    pub fn uses_raw_tokenizer(&self) -> bool {
        self.field_config
            .tokenizer()
            .map(|tokenizer| matches!(tokenizer, SearchTokenizer::Raw(_)))
            .unwrap_or(false)
    }

    pub fn try_coerce(&self, value: &mut OwnedValue) -> Result<()> {
        match (self.field_entry().field_type(), value.clone()) {
            (FieldType::Str(_), OwnedValue::Str(_))
            | (FieldType::U64(_), OwnedValue::U64(_))
            | (FieldType::I64(_), OwnedValue::I64(_))
            | (FieldType::F64(_), OwnedValue::F64(_))
            | (FieldType::Bool(_), OwnedValue::Bool(_))
            | (FieldType::Date(_), OwnedValue::Date(_))
            | (FieldType::JsonObject(_), OwnedValue::Object(_)) => Ok(()),
            (FieldType::U64(_), OwnedValue::I64(v)) => {
                *value = OwnedValue::U64(v.try_into()?);
                Ok(())
            }
            (FieldType::I64(_), OwnedValue::U64(v)) => {
                *value = OwnedValue::I64(v.try_into()?);
                Ok(())
            }
            _ => bail!(
                "cannot coerce value {:?} to field type {:?}",
                value,
                self.field_entry().field_type()
            ),
        }
    }
}

#[derive(Debug, Error)]
pub enum SearchIndexSchemaError {
    #[error("invalid postgres oid passed to search index schema: {0:?}")]
    InvalidPgOid(PgOid),

    #[error("json(b) arrays are not yet supported")]
    JsonArraysNotYetSupported,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use tantivy::schema::{IpAddrOptions, JsonObjectOptions, NumericOptions, TextOptions};

    use crate::schema::SearchFieldConfig;

    #[rstest]
    fn test_search_text_options() {
        let json = r#"{
            "indexed": true,
            "fast": false,
            "fieldnorms": true,
            "record": "basic",
            "normalizer": "raw"
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let search_text_option: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Text": config})).unwrap();
        let expected: TextOptions = search_text_option.into();

        let text_options: TextOptions = SearchFieldConfig::default_text().into();
        assert_eq!(
            expected.get_fast_field_tokenizer_name(),
            text_options.get_fast_field_tokenizer_name()
        );

        let text_options = text_options.set_fast(Some("index"));
        assert_ne!(expected.is_fast(), text_options.is_fast());
    }

    #[rstest]
    fn test_search_inet_options() {
        let json = r#"{
            "indexed": true,
            "fast": true
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let expected: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Inet": config})).unwrap();
        let inet_options: IpAddrOptions = SearchFieldConfig::default_inet().into();

        assert_eq!(inet_options, expected.into());
    }

    #[rstest]
    fn test_search_numeric_options() {
        let json = r#"{
            "indexed": true,
            "fast": true
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let expected: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Numeric": config})).unwrap();
        let int_options: NumericOptions = SearchFieldConfig::default_numeric().into();

        assert_eq!(int_options, expected.into());
    }

    #[rstest]
    fn test_search_boolean_options() {
        let json = r#"{
            "indexed": true,
            "fast": true
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let expected: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Boolean": config})).unwrap();
        let int_options: NumericOptions = SearchFieldConfig::default_numeric().into();

        assert_eq!(int_options, expected.into());
    }

    #[rstest]
    fn test_search_jsonobject_options() {
        let json = r#"{
            "indexed": true,
            "fast": false,
            "expand_dots": true,
            "record": "basic",
            "normalizer": "raw"
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let search_json_option: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Json": config})).unwrap();
        let expected: JsonObjectOptions = search_json_option.into();

        let json_object_options: JsonObjectOptions = SearchFieldConfig::default_json().into();
        assert_eq!(
            expected.get_fast_field_tokenizer_name(),
            json_object_options.get_fast_field_tokenizer_name()
        );
        assert_eq!(
            expected.is_expand_dots_enabled(),
            json_object_options.is_expand_dots_enabled()
        );

        let text_options = json_object_options.set_fast(Some("index"));
        assert_ne!(expected.is_fast(), text_options.is_fast());
    }
}
