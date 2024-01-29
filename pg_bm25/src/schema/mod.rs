mod config;
mod document;
mod fields;

use std::collections::HashMap;

pub use config::*;
pub use document::*;
pub use fields::*;
use pgrx::{PgBuiltInOids, PgOid};
use serde::{Deserialize, Serialize};
use tantivy::schema::{Field, Schema, FAST, INDEXED, STORED};
use thiserror::Error;

/// The id of a field, stored in the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct SearchFieldName(pub String);
/// The name of a field, as it appears to Postgres.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchFieldId(pub Field);
/// The name of the index, as it appears to Postgres.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchIndexName(pub String);
/// The type of the search field.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchFieldType {
    Text,
    I64,
    F64,
    Bool,
    Json,
}

impl TryFrom<&PgOid> for SearchFieldType {
    type Error = SearchIndexSchemaError;
    fn try_from(pg_oid: &PgOid) -> Result<Self, Self::Error> {
        match &pg_oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => Ok(SearchFieldType::Text),
                PgBuiltInOids::INT2OID
                | PgBuiltInOids::INT4OID
                | PgBuiltInOids::INT8OID
                | PgBuiltInOids::OIDOID
                | PgBuiltInOids::XIDOID => Ok(SearchFieldType::I64),
                PgBuiltInOids::FLOAT4OID | PgBuiltInOids::FLOAT8OID | PgBuiltInOids::NUMERICOID => {
                    Ok(SearchFieldType::F64)
                }
                PgBuiltInOids::BOOLOID => Ok(SearchFieldType::Bool),
                PgBuiltInOids::JSONOID | PgBuiltInOids::JSONBOID => Ok(SearchFieldType::Json),
                _ => Err(SearchIndexSchemaError::InvalidPgOid(pg_oid.clone())),
            },
            _ => Err(SearchIndexSchemaError::InvalidPgOid(pg_oid.clone())),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum SearchFieldConfig {
    Text(ParadeTextOptions),
    Json(ParadeJsonOptions),
    Numeric(ParadeNumericOptions),
    Boolean(ParadeBooleanOptions),
    Key,
    Ctid,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchField {
    /// The id of the field, stored in the index.
    pub id: SearchFieldId,
    /// The name of the field, as it appears to Postgres.
    pub name: SearchFieldName,
    /// Configuration for the field passed at index build time.
    pub config: SearchFieldConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchIndexSchema {
    /// The fields that are stored in the index.
    pub fields: Vec<SearchField>,
    /// The index of the key field in the fields vector.
    pub key: usize,
    /// The index of the ctid field in the fields vector.
    pub ctid: usize,
    /// The underlying tantivy schema
    pub schema: Schema,
    /// A lookup cache for retrieving search fields.
    #[serde(skip_serializing)]
    pub lookup: Option<HashMap<SearchFieldName, usize>>,
}

impl SearchIndexSchema {
    pub fn new(
        fields: Vec<(SearchFieldName, SearchFieldConfig)>,
    ) -> Result<Self, SearchIndexSchemaError> {
        let mut builder = Schema::builder();
        let mut search_fields = vec![];

        for (field_name, field_config) in fields {
            let search_field = match &field_config {
                SearchFieldConfig::Text(config) => SearchField {
                    id: SearchFieldId(builder.add_text_field(&field_name.0, config)),
                    name: field_name,
                    config: SearchFieldConfig::Text(*config),
                },
                SearchFieldConfig::Numeric(config) => SearchField {
                    id: SearchFieldId(builder.add_i64_field(&field_name.0, config)),
                    name: field_name,
                    config: SearchFieldConfig::Numeric(*config),
                },
                SearchFieldConfig::Boolean(config) => SearchField {
                    id: SearchFieldId(builder.add_bool_field(&field_name.0, config)),
                    name: field_name,
                    config: SearchFieldConfig::Boolean(*config),
                },
                SearchFieldConfig::Json(config) => SearchField {
                    id: SearchFieldId(builder.add_json_field(&field_name.0, config)),
                    name: field_name,
                    config: SearchFieldConfig::Json(*config),
                },
                SearchFieldConfig::Key => SearchField {
                    id: SearchFieldId(
                        builder.add_i64_field(&field_name.0, INDEXED | STORED | FAST),
                    ),
                    name: field_name,
                    config: SearchFieldConfig::Key,
                },
                SearchFieldConfig::Ctid => SearchField {
                    id: SearchFieldId(
                        builder.add_u64_field(&field_name.0, INDEXED | STORED | FAST),
                    ),
                    name: field_name,
                    config: SearchFieldConfig::Ctid,
                },
            };

            search_fields.push(search_field);
        }

        let key_index = search_fields
            .iter()
            .position(|field| match field.config {
                SearchFieldConfig::Key => true,
                _ => false,
            })
            .ok_or(SearchIndexSchemaError::NoKeyFieldSpecified)?;

        let ctid_index = search_fields
            .iter()
            .position(|field| match field.config {
                SearchFieldConfig::Key => true,
                _ => false,
            })
            .ok_or(SearchIndexSchemaError::NoCtidFieldSpecified)?;

        let schema = builder.build();

        // pgrx::log!("SCHEMA {:#?}", serde_json::to_string(&schema));

        Ok(Self {
            key: key_index,
            ctid: ctid_index,
            schema,
            lookup: Self::build_lookup(&search_fields).into(),
            fields: search_fields,
        })
    }

    fn build_lookup(search_fields: &Vec<SearchField>) -> HashMap<SearchFieldName, usize> {
        let mut lookup = HashMap::new();
        search_fields
            .iter()
            .enumerate()
            .for_each(|(idx, search_field)| {
                let name = search_field.name.clone();
                lookup.insert(name, idx);
            });
        lookup
    }

    pub fn ctid_field(&self) -> SearchField {
        self.fields
            .get(self.ctid)
            .expect("ctid field should be present on search schema")
            .clone()
    }

    pub fn key_field(&self) -> SearchField {
        self.fields
            .get(self.key)
            .expect("key field should be present on search schema")
            .clone()
    }

    pub fn new_document(&self) -> SearchDocument {
        let doc = tantivy::Document::new();
        let key = self.key_field().id;
        let ctid = self.ctid_field().id;
        SearchDocument { doc, key, ctid }
    }

    pub fn get_search_field(&self, name: &SearchFieldName) -> Option<&SearchField> {
        if let Some(lookup) = &self.lookup {
            lookup.get(name).and_then(|idx| self.fields.get(*idx))
        } else {
            let lookup = Self::build_lookup(&self.fields);
            lookup.get(name).and_then(|idx| self.fields.get(*idx))
        }
    }
}

#[derive(Debug, Error)]
pub enum SearchIndexSchemaError {
    #[error("invalid postgres oid passed to search index schema: {0:?}")]
    InvalidPgOid(PgOid),
    #[error("no key field specified for search index")]
    NoKeyFieldSpecified,
    #[error("no ctid field specified for search index")]
    NoCtidFieldSpecified,
}
