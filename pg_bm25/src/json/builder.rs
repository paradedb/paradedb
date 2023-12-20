use crate::json::json_string::JsonString;
use pgrx::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tantivy::schema::Field;
use tantivy::Document;

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum JsonBuilderValue {
    bool(bool),
    i16(i16),
    i32(i32),
    i64(i64),
    u32(u32),
    u64(u64),
    f32(f32),
    f64(f64),
    string(String),
    json_string(String),
    jsonb(serde_json::Value),
    json_value(serde_json::Value),
    string_array(Vec<Option<String>>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonBuilder {
    // Using IndexMap to maintain insertion order.
    pub values: HashMap<Field, JsonBuilderValue>,
    // The field_map is only used as a lookup for adding values.
    // It's not requited on the deserialization side.
    #[serde(skip_serializing)]
    field_map: Option<HashMap<String, Field>>,
}

impl JsonBuilder {
    pub fn new(fields: HashMap<String, Field>) -> Self {
        let field_map = Some(fields.clone());
        // We will check for existing field_name keys in the `add` methods below.
        // Fields will only be inserted into the JSON builder if they we passed
        // to this `new` method.
        JsonBuilder {
            values: HashMap::new(),
            field_map,
        }
    }

    fn insert(&mut self, attname: String, value: JsonBuilderValue) {
        let field_map = &self
            .field_map
            .as_ref()
            .expect("JsonBuilder field_map has not been initialized");

        if let Some(field) = field_map.get(&attname) {
            self.values.insert(field.clone(), value);
        }
    }

    #[inline]
    pub fn add_bool(&mut self, attname: String, value: bool) {
        self.insert(attname, JsonBuilderValue::bool(value));
    }

    #[inline]
    pub fn add_i16(&mut self, attname: String, value: i16) {
        self.insert(attname, JsonBuilderValue::i16(value));
    }

    #[inline]
    pub fn add_i32(&mut self, attname: String, value: i32) {
        self.insert(attname, JsonBuilderValue::i32(value));
    }

    #[inline]
    pub fn add_i64(&mut self, attname: String, value: i64) {
        self.insert(attname, JsonBuilderValue::i64(value));
    }

    #[inline]
    pub fn add_u32(&mut self, attname: String, value: u32) {
        self.insert(attname, JsonBuilderValue::u32(value));
    }

    #[inline]
    pub fn add_u64(&mut self, attname: String, value: u64) {
        self.insert(attname, JsonBuilderValue::u64(value));
    }

    #[inline]
    pub fn add_f32(&mut self, attname: String, value: f32) {
        self.insert(attname, JsonBuilderValue::f32(value));
    }

    #[inline]
    pub fn add_f64(&mut self, attname: String, value: f64) {
        self.insert(attname, JsonBuilderValue::f64(value));
    }

    #[inline]
    pub fn add_string(&mut self, attname: String, value: String) {
        self.insert(attname, JsonBuilderValue::string(value));
    }

    #[inline]
    pub fn add_json_string(&mut self, attname: String, pgrx::JsonString(value): pgrx::JsonString) {
        self.insert(attname, JsonBuilderValue::json_string(value));
    }

    #[inline]
    pub fn add_jsonb(&mut self, attname: String, JsonB(value): JsonB) {
        self.insert(attname, JsonBuilderValue::jsonb(value));
    }

    #[inline]
    pub fn add_json_value(&mut self, attname: String, value: serde_json::Value) {
        self.insert(attname, JsonBuilderValue::json_value(value));
    }

    #[inline]
    pub fn add_string_array(&mut self, attname: String, value: Vec<Option<String>>) {
        self.insert(attname, JsonBuilderValue::string_array(value));
    }
}

impl JsonBuilderValue {
    pub fn add_to_tantivy_doc(&self, doc: &mut Document, field: &Field) {
        match self {
            JsonBuilderValue::bool(val) => doc.add_bool(*field, *val),
            JsonBuilderValue::i16(val) => doc.add_i64(*field, *val as i64),
            JsonBuilderValue::i32(val) => doc.add_i64(*field, *val as i64),
            JsonBuilderValue::i64(val) => doc.add_i64(*field, *val),
            JsonBuilderValue::u32(val) => doc.add_u64(*field, *val as u64),
            JsonBuilderValue::u64(val) => doc.add_u64(*field, *val),
            JsonBuilderValue::f32(val) => doc.add_f64(*field, *val as f64),
            JsonBuilderValue::f64(val) => doc.add_f64(*field, *val),
            JsonBuilderValue::string(val) => doc.add_text(*field, val),
            JsonBuilderValue::json_string(val) => {
                let mut s = Vec::new();
                val.push_json(&mut s);
                if let Ok(json_str) = String::from_utf8(s) {
                    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(&json_str) {
                        doc.add_json_object(*field, map.clone());
                    }
                }
            }
            JsonBuilderValue::jsonb(serde_json::Value::Object(map)) => {
                doc.add_json_object(*field, map.clone());
            }
            JsonBuilderValue::json_value(serde_json::Value::Object(map)) => {
                doc.add_json_object(*field, map.clone());
            }
            JsonBuilderValue::string_array(val) => {
                for v in val.iter().flatten() {
                    doc.add_text(*field, v);
                }
            }
            _ => {} // Ignore other types for now
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::*;

    use super::JsonBuilder;

    #[pg_test]
    fn test_new_builder() {
        let builder = JsonBuilder::new(std::collections::HashMap::new());
        assert_eq!(builder.values.len(), 0);
    }
}
