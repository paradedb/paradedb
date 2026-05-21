use crate::postgres::datetime::PostgresDateTime;
use serde::{Deserialize, Serialize};
use tantivy::schema::OwnedValue;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum QueryValue {
    Tantivy(OwnedValue),
    // This is tagged with date to make it seperable during serialization/deserialization
    DateTime { date: PostgresDateTime },
}
impl QueryValue {
    pub fn as_tantivy(self) -> Option<OwnedValue> {
        match self {
            Self::Tantivy(value) => {
                assert!(
                    !matches!(value, OwnedValue::Date(_)),
                    "QueryValue::Tantivy should never hold an OwnedValue::Date"
                );
                Some(value)
            }
            Self::DateTime { .. } => None,
        }
    }
}
