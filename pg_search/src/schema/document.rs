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

use std::{io, io::Cursor};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use tantivy::schema::{FieldValue, OwnedValue};
use tantivy::TantivyDocument;
use tantivy_common::{BinarySerializable, VInt};

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
    // let mut buffer = Vec::new();
    // // BinarySerializable::serialize(doc, &mut buffer).map_err(serde::ser::Error::custom)?;
    // let field_values = doc.field_values();
    // BinarySerializable::serialize(&VInt(field_values.len() as u64), &mut buffer).unwrap();
    // for field_value in field_values {
    //     // field_value.serialize(&mut buffer).unwrap();
    //     BinarySerializable::serialize(&field_value.field, &mut buffer).unwrap();
    //     BinarySerializable::serialize(&field_value.value, &mut buffer).unwrap();
    // }
    // serializer.serialize_bytes(&buffer)
    // doc.serialize(serializer)
    let doc_string = serde_json::to_string(doc).unwrap();
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
            // formatter.write_str("a byte array representing a TantivyDocument")
            formatter.write_str("a str representing a TantivyDocument")
        }

        // fn visit_bytes<E>(self, value: &[u8]) -> Result<TantivyDocument, E>
        // where
        //     E: serde::de::Error,
        // {
        //     // let mut cursor = Cursor::new(value);
        //     // // BinarySerializable::deserialize(&mut cursor)
        //     // //     .map_err(|err| E::custom(format!("Error deserializing TantivyDocument: {}", err)))

        //     // let num_field_values = VInt::deserialize(&mut cursor).unwrap().val() as usize;
        //     // let field_values = (0..num_field_values)
        //     //     .map(|_| FieldValue::deserialize(&mut cursor ))
        //     //     .collect::<io::Result<Vec<FieldValue>>>().unwrap();
        //     let field_values: Vec<FieldValue> = vec![];
        //     Ok(TantivyDocument::from(field_values))
        // }

        fn visit_str<E>(self, value: &str) -> Result<TantivyDocument, E>
        where
            E: serde::de::Error,
        {
            Ok(serde_json::from_str(value).unwrap())
        }
    }

    deserializer.deserialize_str(DocumentVisitor)

    // deserializer.deserialize_bytes(DocumentVisitor)
    // TantivyDocument::deserialize(deserializer)
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
