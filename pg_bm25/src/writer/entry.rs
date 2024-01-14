use serde::{Deserialize, Serialize};
use serde_json::Map;
use tantivy::{
    schema::{Field, Value},
    Term,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("unsupported value for attribute '{0}': {1}")]
    UnsupportedValue(String, String),

    #[error("could not dereference postgres datum")]
    DatumDeref,

    #[error("couldn't get writer for index {0}: {1}")]
    GetWriterFailed(String, String),

    #[error("{0} has a type oid of InvalidOid")]
    InvalidOid(String),

    #[error(transparent)]
    TantivyError(#[from] tantivy::TantivyError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
}

pub type IndexKey = Field;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexValue {
    Bool(bool),
    I16(i16),
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
    Json(String),
    JsonB(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub key: IndexKey,
    pub value: IndexValue,
}

impl IndexEntry {
    pub fn new(key: IndexKey, value: IndexValue) -> Self {
        Self { key, value }
    }
}

impl TryFrom<IndexValue> for Value {
    type Error = IndexError;
    fn try_from(value: IndexValue) -> Result<Self, IndexError> {
        let tantivy_value = match value {
            IndexValue::Bool(val) => val.into(),
            IndexValue::I16(val) => (val as i64).into(),
            IndexValue::I32(val) => (val as i64).into(),
            IndexValue::I64(val) => val.into(),
            IndexValue::U32(val) => (val as u64).into(),
            IndexValue::U64(val) => val.into(),
            IndexValue::F32(val) => (val as f64).into(),
            IndexValue::F64(val) => val.into(),
            IndexValue::String(val) => val.into(),
            IndexValue::Json(val) => {
                serde_json::from_str::<Map<String, serde_json::Value>>(&val)?.into()
            }
            IndexValue::JsonB(val) => {
                serde_json::from_slice::<Map<String, serde_json::Value>>(&val)?.into()
            }
        };

        Ok(tantivy_value)
    }
}

impl From<IndexEntry> for Term {
    fn from(entry: IndexEntry) -> Self {
        let IndexEntry { key, value } = entry;
        match value {
            IndexValue::Bool(val) => Term::from_field_bool(key, val),
            IndexValue::I16(val) => Term::from_field_i64(key, val as i64),
            IndexValue::I32(val) => Term::from_field_i64(key, val as i64),
            IndexValue::I64(val) => Term::from_field_i64(key, val),
            IndexValue::U32(val) => Term::from_field_u64(key, val as u64),
            IndexValue::U64(val) => Term::from_field_u64(key, val),
            IndexValue::F32(val) => Term::from_field_f64(key, val as f64),
            IndexValue::F64(val) => Term::from_field_f64(key, val),
            IndexValue::String(val) => Term::from_field_text(key, &val),
            IndexValue::Json(val) => Term::from_field_text(key, &val),
            IndexValue::JsonB(val) => Term::from_field_bytes(key, &val),
        }
    }
}
