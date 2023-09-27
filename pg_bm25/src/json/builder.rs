use crate::json::json_string::JsonString;
use pgrx::JsonB;
use tantivy::schema::Field;
use tantivy::Document;

#[derive(Debug)]
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
    json_string(pgrx::JsonString),
    jsonb(JsonB),
    json_value(serde_json::Value),
}

#[derive(Debug)]
pub struct JsonBuilder {
    pub values: Vec<(String, JsonBuilderValue)>,
}

#[allow(dead_code)]
impl JsonBuilder {
    pub fn new(num_fields: usize) -> Self {
        JsonBuilder {
            values: Vec::with_capacity(num_fields + 5),
        }
    }

    #[inline]
    pub fn add_bool(&mut self, attname: String, value: bool) {
        self.values.push((attname, JsonBuilderValue::bool(value)));
    }

    #[inline]
    pub fn add_i16(&mut self, attname: String, value: i16) {
        self.values.push((attname, JsonBuilderValue::i16(value)));
    }

    #[inline]
    pub fn add_i32(&mut self, attname: String, value: i32) {
        self.values.push((attname, JsonBuilderValue::i32(value)));
    }

    #[inline]
    pub fn add_i64(&mut self, attname: String, value: i64) {
        self.values.push((attname, JsonBuilderValue::i64(value)));
    }

    #[inline]
    pub fn add_u32(&mut self, attname: String, value: u32) {
        self.values.push((attname, JsonBuilderValue::u32(value)));
    }

    #[inline]
    pub fn add_u64(&mut self, attname: String, value: u64) {
        self.values.push((attname, JsonBuilderValue::u64(value)));
    }

    #[inline]
    pub fn add_f32(&mut self, attname: String, value: f32) {
        self.values.push((attname, JsonBuilderValue::f32(value)));
    }

    #[inline]
    pub fn add_f64(&mut self, attname: String, value: f64) {
        self.values.push((attname, JsonBuilderValue::f64(value)));
    }

    #[inline]
    pub fn add_string(&mut self, attname: String, value: String) {
        self.values.push((attname, JsonBuilderValue::string(value)));
    }

    #[inline]
    pub fn add_json_string(&mut self, attname: String, value: pgrx::JsonString) {
        self.values
            .push((attname, JsonBuilderValue::json_string(value)));
    }

    #[inline]
    pub fn add_jsonb(&mut self, attname: String, value: JsonB) {
        self.values.push((attname, JsonBuilderValue::jsonb(value)));
    }

    #[inline]
    pub fn add_json_value(&mut self, attname: String, value: serde_json::Value) {
        self.values
            .push((attname, JsonBuilderValue::json_value(value)));
    }

    pub fn build(&self, json: &mut Vec<u8>) {
        json.push(b'{');
        for (idx, (key, value)) in self.values.iter().enumerate() {
            if idx > 0 {
                json.push(b',');
            }

            // key was pre-quoted during categorize_tupdesc
            json.extend_from_slice(key.as_bytes());
            json.push(b':');

            match value {
                JsonBuilderValue::bool(v) => v.push_json(json),
                JsonBuilderValue::i16(v) => v.push_json(json),
                JsonBuilderValue::i32(v) => v.push_json(json),
                JsonBuilderValue::i64(v) => v.push_json(json),
                JsonBuilderValue::u32(v) => v.push_json(json),
                JsonBuilderValue::u64(v) => v.push_json(json),
                JsonBuilderValue::f32(v) => v.push_json(json),
                JsonBuilderValue::f64(v) => v.push_json(json),
                JsonBuilderValue::string(v) => v.push_json(json),
                JsonBuilderValue::json_string(v) => v.push_json(json),
                JsonBuilderValue::jsonb(v) => v.push_json(json),
                JsonBuilderValue::json_value(v) => v.push_json(json),
            }
        }
        json.push(b'}');
    }
}

impl JsonBuilderValue {
    pub fn add_to_tantivy_doc(&self, doc: &mut Document, field: &Field) {
        match self {
            JsonBuilderValue::bool(val) => doc.add_bool(*field, *val),
            JsonBuilderValue::i64(val) => doc.add_i64(*field, *val),
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
            JsonBuilderValue::jsonb(JsonB(serde_json::Value::Object(map))) => {
                doc.add_json_object(*field, map.clone());
            }
            JsonBuilderValue::json_value(serde_json::Value::Object(map)) => {
                doc.add_json_object(*field, map.clone());
            }
            _ => {} // Ignore other types for now
        }
    }
}
