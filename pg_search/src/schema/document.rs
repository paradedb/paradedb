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
use std::io::Cursor;
use tantivy::{schema::Value, Document};
use tantivy_common::BinarySerializable;

use crate::schema::SearchFieldId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDocument {
    #[serde(serialize_with = "serialize_document")]
    #[serde(deserialize_with = "deserialize_document")]
    pub doc: Document,
    pub key: SearchFieldId,
    pub ctid: SearchFieldId,
}

impl SearchDocument {
    pub fn insert(&mut self, SearchFieldId(key): SearchFieldId, value: Value) {
        self.doc.add_field_value(key, value)
    }
}

impl From<SearchDocument> for Document {
    fn from(value: SearchDocument) -> Self {
        value.doc
    }
}

fn serialize_document<S>(doc: &Document, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut buffer = Vec::new();
    BinarySerializable::serialize(doc, &mut buffer).map_err(serde::ser::Error::custom)?;
    serializer.serialize_bytes(&buffer)
}

fn deserialize_document<'de, D>(deserializer: D) -> Result<Document, D::Error>
where
    D: Deserializer<'de>,
{
    struct DocumentVisitor;

    impl<'de> Visitor<'de> for DocumentVisitor {
        type Value = Document;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a byte array representing a Document")
        }

        fn visit_bytes<E>(self, value: &[u8]) -> Result<Document, E>
        where
            E: serde::de::Error,
        {
            let mut cursor = Cursor::new(value);
            BinarySerializable::deserialize(&mut cursor)
                .map_err(|err| E::custom(format!("Error deserializing Document: {}", err)))
        }
    }

    deserializer.deserialize_bytes(DocumentVisitor)
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
