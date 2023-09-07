use std::collections::HashMap;

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum TantivyType {
    Text,
    I64,
    F64,
    Bool,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PostgresType {
    Text,
    CharacterVarying,
    SmallInt,
    BigInt,
    Integer,
    Numeric,
    Oid,
    Serial,
    BigSerial,
    Real,
    DoublePrecision,
    Bool,
    Json,
    JsonB,
}

impl PostgresType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "character varying" => Some(Self::CharacterVarying),
            "smallint" => Some(Self::SmallInt),
            "bigint" => Some(Self::BigInt),
            "integer" => Some(Self::Integer),
            "numeric" => Some(Self::Numeric),
            "oid" => Some(Self::Oid),
            "serial" => Some(Self::Serial),
            "bigserial" => Some(Self::BigSerial),
            "real" => Some(Self::Real),
            "double precision" => Some(Self::DoublePrecision),
            "boolean" => Some(Self::Bool),
            "json" => Some(Self::Json),
            "jsonb" => Some(Self::JsonB),
            _ => None,
        }
    }
}

pub fn postgres_to_tantivy_map() -> HashMap<PostgresType, TantivyType> {
    let mut type_map: HashMap<PostgresType, TantivyType> = HashMap::new();

    for &pg_type in &[PostgresType::Text, PostgresType::CharacterVarying] {
        type_map.insert(pg_type, TantivyType::Text);
    }

    for &pg_type in &[
        PostgresType::SmallInt,
        PostgresType::BigInt,
        PostgresType::Integer,
        PostgresType::Oid,
        PostgresType::Serial,
        PostgresType::BigSerial,
    ] {
        type_map.insert(pg_type, TantivyType::I64);
    }

    for &pg_type in &[
        PostgresType::Real,
        PostgresType::DoublePrecision,
        PostgresType::Numeric,
    ] {
        type_map.insert(pg_type, TantivyType::F64);
    }

    for &pg_type in &[PostgresType::Json, PostgresType::JsonB] {
        type_map.insert(pg_type, TantivyType::Json);
    }

    type_map.insert(PostgresType::Bool, TantivyType::Bool);

    type_map
}
