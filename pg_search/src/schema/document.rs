// Copyright (c) 2023-2024 Retake, Inc.
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

use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use tantivy::schema::{Field, FieldValue, OwnedValue, Value};
use tantivy::TantivyDocument;

use crate::schema::SearchFieldId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDocument {
    #[serde(serialize_with = "serialize_document")]
    #[serde(deserialize_with = "deserialize_document")]
    pub doc: TantivyDocument,
    pub key: SearchFieldId,
    pub ctid: SearchFieldId,
}

impl SearchDocument {
    pub fn insert(&mut self, SearchFieldId(key): SearchFieldId, value: OwnedValue) {
        self.doc.add_field_value(key, value)
    }
}

impl From<SearchDocument> for TantivyDocument {
    fn from(value: SearchDocument) -> Self {
        value.doc
    }
}

fn serialize_document<S>(doc: &TantivyDocument, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut field_values: Vec<(String, String, OwnedValue)> = vec![];
    for field_value in doc.field_values() {
        let field_string =
            serde_json::to_string(&field_value.field()).map_err(serde::ser::Error::custom)?;
        // We have to store the type for the following because positive i64s and dates get automatically deserialized as u64s
        let (value_type, value) = match field_value.value() {
            tantivy::schema::document::OwnedValue::I64(i64) => (
                "i64".into(),
                tantivy::schema::document::OwnedValue::Str(
                    serde_json::to_string(i64).map_err(serde::ser::Error::custom)?,
                ),
            ),
            tantivy::schema::document::OwnedValue::Date(date) => (
                "date".into(),
                tantivy::schema::document::OwnedValue::Str(
                    serde_json::to_string(date).map_err(serde::ser::Error::custom)?,
                ),
            ),
            val => ("other".into(), val.clone()),
        };
        field_values.push((field_string, value_type, value.clone()));
    }

    let doc_string = serde_json::to_string(&field_values).map_err(serde::ser::Error::custom)?;

    serializer.serialize_str(&doc_string)
}

fn deserialize_document<'de, D>(deserializer: D) -> Result<TantivyDocument, D::Error>
where
    D: Deserializer<'de>,
{
    struct DocumentVisitor;

    impl<'de> Visitor<'de> for DocumentVisitor {
        type Value = TantivyDocument;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a str representing a TantivyDocument")
        }

        fn visit_str<E>(self, value: &str) -> Result<TantivyDocument, E>
        where
            E: serde::de::Error,
        {
            let doc_vec: Vec<(String, String, OwnedValue)> =
                serde_json::from_str(value).map_err(|err| {
                    E::custom(format!(
                        "Error deserializing TantivyDocument string: {}",
                        err
                    ))
                })?;
            let mut field_values: Vec<FieldValue> = vec![];
            for vec_entry in doc_vec {
                let field: Field = serde_json::from_str(&vec_entry.0).map_err(|err| {
                    E::custom(format!("Error deserializing Field from string: {}", err))
                })?;
                let value_type: String = vec_entry.1;
                let owned_value: OwnedValue = match value_type.as_str() {
                    "i64" => tantivy::schema::document::OwnedValue::I64(
                        serde_json::from_str(
                            (&vec_entry.2)
                                .as_str()
                                .ok_or(E::custom("Could not get OwnedValue as str".to_string()))?,
                        )
                        .map_err(|err| {
                            E::custom(format!("Error deserializing i64 from string: {}", err))
                        })?,
                    ),
                    "date" => tantivy::schema::document::OwnedValue::Date(
                        serde_json::from_str(
                            (&vec_entry.2)
                                .as_str()
                                .ok_or(E::custom("Could not get OwnedValue as str".to_string()))?,
                        )
                        .map_err(|err| {
                            E::custom(format!("Error deserializing DateTime from string: {}", err))
                        })?,
                    ),
                    _ => vec_entry.2,
                };

                field_values.push(FieldValue::new(field, owned_value));
            }

            Ok(TantivyDocument::from(field_values))
        }
    }

    deserializer.deserialize_str(DocumentVisitor)
}

#[cfg(test)]
mod tests {
    use crate::fixtures::*;
    use crate::schema::SearchDocument;
    use rstest::*;

    #[rstest]
    fn test_search_document_serialization(simple_doc: SearchDocument) {
        let ser = bincode::serialize(&simple_doc).unwrap();
        let de: SearchDocument = bincode::deserialize(&ser).unwrap();

        assert_eq!(de, simple_doc);
    }
}
