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

mod anyenum;
mod config;
pub mod range;

use crate::api::FieldName;
use crate::api::HashMap;
use crate::index::mvcc::MVCCDirectory;
use crate::postgres::options::SearchIndexOptions;
pub use anyenum::AnyEnum;
use anyhow::bail;
pub use config::*;

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::extract_field_attributes;
use anyhow::Result;
use derive_more::Into;
use pgrx::{pg_sys, PgBuiltInOids, PgOid};
use serde::{Deserialize, Serialize};
use tantivy::index::Index;
use tantivy::schema::{Field, FieldEntry, FieldType, OwnedValue, Schema};
use thiserror::Error;
use tokenizers::manager::SearchTokenizerFilters;
use tokenizers::{SearchNormalizer, SearchTokenizer};

/// The type of the search field.
/// Like Tantivy's [`FieldType`](https://docs.rs/tantivy/latest/tantivy/schema/enum.FieldType.html),
/// but with the Postgres Oid of the column that the field is based on.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchFieldType {
    Text(pg_sys::Oid),
    Uuid(pg_sys::Oid),
    I64(pg_sys::Oid),
    F64(pg_sys::Oid),
    U64(pg_sys::Oid),
    Bool(pg_sys::Oid),
    Json(pg_sys::Oid),
    Date(pg_sys::Oid),
    Range(pg_sys::Oid),
}

impl SearchFieldType {
    pub fn default_config(&self) -> SearchFieldConfig {
        match self {
            SearchFieldType::Text(_) => SearchFieldConfig::default_text(),
            SearchFieldType::Uuid(_) => SearchFieldConfig::default_uuid(),
            SearchFieldType::I64(_) => SearchFieldConfig::default_numeric(),
            SearchFieldType::F64(_) => SearchFieldConfig::default_numeric(),
            SearchFieldType::U64(_) => SearchFieldConfig::default_numeric(),
            SearchFieldType::Bool(_) => SearchFieldConfig::default_boolean(),
            SearchFieldType::Json(_) => SearchFieldConfig::default_json(),
            SearchFieldType::Date(_) => SearchFieldConfig::default_date(),
            SearchFieldType::Range(_) => SearchFieldConfig::default_range(),
        }
    }

    pub fn typeoid(&self) -> pg_sys::Oid {
        match self {
            SearchFieldType::Text(oid) => *oid,
            SearchFieldType::Uuid(oid) => *oid,
            SearchFieldType::I64(oid) => *oid,
            SearchFieldType::F64(oid) => *oid,
            SearchFieldType::U64(oid) => *oid,
            SearchFieldType::Bool(oid) => *oid,
            SearchFieldType::Json(oid) => *oid,
            SearchFieldType::Date(oid) => *oid,
            SearchFieldType::Range(oid) => *oid,
        }
    }
}

impl TryFrom<&PgOid> for SearchFieldType {
    type Error = SearchIndexSchemaError;
    fn try_from(pg_oid: &PgOid) -> Result<Self, Self::Error> {
        let array_type = unsafe { pg_sys::get_element_type(pg_oid.value()) };
        let base_oid = if array_type != pg_sys::InvalidOid {
            PgOid::from(array_type)
        } else {
            *pg_oid
        };
        match &base_oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                    Ok(SearchFieldType::Text((*builtin).into()))
                }
                PgBuiltInOids::UUIDOID => Ok(SearchFieldType::Uuid((*builtin).into())),
                PgBuiltInOids::INT2OID | PgBuiltInOids::INT4OID | PgBuiltInOids::INT8OID => {
                    Ok(SearchFieldType::I64((*builtin).into()))
                }
                PgBuiltInOids::OIDOID | PgBuiltInOids::XIDOID => {
                    Ok(SearchFieldType::U64((*builtin).into()))
                }
                PgBuiltInOids::FLOAT4OID | PgBuiltInOids::FLOAT8OID | PgBuiltInOids::NUMERICOID => {
                    Ok(SearchFieldType::F64((*builtin).into()))
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
                _ => Err(SearchIndexSchemaError::InvalidPgOid(*pg_oid)),
            },
            PgOid::Custom(custom) => {
                if unsafe { pgrx::pg_sys::type_is_enum(*custom) } {
                    Ok(SearchFieldType::F64(*custom))
                } else {
                    Err(SearchIndexSchemaError::InvalidPgOid(*pg_oid))
                }
            }
            _ => Err(SearchIndexSchemaError::InvalidPgOid(*pg_oid)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Into)]
pub struct SearchIndexSchema {
    #[into]
    schema: Schema,
    relation_oid: pg_sys::Oid,
}

impl SearchIndexSchema {
    pub fn from_index(indexrel: &PgSearchRelation, index: &Index) -> Self {
        Self {
            schema: index.schema(),
            relation_oid: indexrel.oid(),
        }
    }

    pub fn open(indexrel: &PgSearchRelation) -> Result<Self> {
        let directory = MVCCDirectory::snapshot(indexrel);
        let index = Index::open(directory)?;
        Ok(Self::from_index(indexrel, &index))
    }

    pub fn ctid_field(&self) -> Field {
        self.schema
            .get_field("ctid")
            .expect("ctid field should be present in the index")
    }

    pub fn key_field(&self) -> SearchField {
        let index_relation = PgSearchRelation::open(self.relation_oid);
        let options = unsafe { SearchIndexOptions::from_relation(&index_relation) };
        let key_field_name = options.key_field_name();
        let field = self.schema.get_field(&key_field_name.root()).unwrap();
        SearchField::new(field, self.relation_oid, self.schema.clone())
    }

    pub fn search_field(&self, name: impl AsRef<str>) -> Option<SearchField> {
        match self.schema.get_field(name.as_ref()) {
            Ok(field) => Some(SearchField::new(
                field,
                self.relation_oid,
                self.schema.clone(),
            )),
            Err(_) => None,
        }
    }

    pub fn fields(&self) -> impl Iterator<Item = (Field, &FieldEntry)> {
        self.schema.fields()
    }

    pub fn search_fields(&self) -> impl Iterator<Item = SearchField> + use<'_> {
        self.schema
            .fields()
            .filter(|(_, entry)| !FieldName::from(entry.name()).is_ctid())
            .map(|(field, _)| SearchField::new(field, self.relation_oid, self.schema.clone()))
    }

    /// A lookup from a Postgres column name to search fields that have
    /// marked it as their source column with the 'column' key.
    pub fn alias_lookup(&self) -> HashMap<String, Vec<SearchField>> {
        let mut lookup = HashMap::default();
        let index_relation = PgSearchRelation::open(self.relation_oid);

        let options = unsafe { SearchIndexOptions::from_relation(&index_relation) };
        let aliased_text_configs = options.aliased_text_configs();
        let aliased_json_configs = options.aliased_json_configs();

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
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct SearchField {
    field: Field,
    field_type: SearchFieldType,
    field_config: SearchFieldConfig,
    schema: Schema,
}

impl SearchField {
    pub fn new(field: Field, relation_oid: pg_sys::Oid, schema: Schema) -> Self {
        let index_relation = PgSearchRelation::open(relation_oid);
        let options = unsafe { SearchIndexOptions::from_relation(&index_relation) };

        let field_name: FieldName = schema.get_field_name(field).into();
        let field_config = options.field_config_or_default(&field_name);
        let attribute_name = field_config.alias().unwrap_or(field_name.as_ref());

        let field_type: SearchFieldType = if field_name.is_ctid() {
            // the "ctid" field isn't an attribute, per se, in the index itself
            // it's one we add directly, so we need to account for it here
            SearchFieldType::U64(pg_sys::TIDOID)
        } else {
            let attribute_type_oid: PgOid = extract_field_attributes(&index_relation)
                .into_iter()
                .find(|(name, _)| **name == *attribute_name)
                .map(|(_, type_oid)| type_oid.into())
                .unwrap_or_else(|| {
                    panic!(
                        "the column {} referenced by the field configuration for {} should exist",
                        field_name, attribute_name
                    )
                });
            (&attribute_type_oid).try_into().unwrap_or_else(|_| {
                panic!(
                    "failed to convert attribute {} to search field type",
                    attribute_name
                )
            })
        };

        Self {
            field,
            field_type,
            schema,
            field_config,
        }
    }

    pub fn field(&self) -> Field {
        self.field
    }

    pub fn field_name(&self) -> FieldName {
        self.schema.get_field_name(self.field).into()
    }

    pub fn field_entry(&self) -> &FieldEntry {
        self.schema.get_field_entry(self.field)
    }

    pub fn field_type(&self) -> SearchFieldType {
        self.field_type.clone()
    }

    pub fn is_raw_sortable(&self) -> bool {
        self.is_sortable(SearchNormalizer::Raw)
    }

    pub fn is_lower_sortable(&self) -> bool {
        self.is_sortable(SearchNormalizer::Lowercase)
    }

    pub fn is_fast(&self) -> bool {
        self.schema.get_field_entry(self.field).is_fast()
    }

    pub fn is_numeric_fast(&self) -> bool {
        match self.schema.get_field_entry(self.field).field_type() {
            FieldType::I64(options) => options.is_fast(),
            FieldType::U64(options) => options.is_fast(),
            FieldType::F64(options) => options.is_fast(),
            FieldType::Bool(options) => options.is_fast(),
            FieldType::Date(options) => options.is_fast(),
            _ => false,
        }
    }

    fn is_sortable(&self, desired_normalizer: SearchNormalizer) -> bool {
        match self.schema.get_field_entry(self.field).field_type() {
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
        self.schema
            .get_field_entry(self.field)
            .field_type()
            .is_date()
    }

    pub fn is_text(&self) -> bool {
        self.schema
            .get_field_entry(self.field)
            .field_type()
            .is_str()
    }

    pub fn is_json(&self) -> bool {
        matches!(self.field_type, SearchFieldType::Json(_))
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
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use tantivy::schema::{JsonObjectOptions, NumericOptions, TextOptions};

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
