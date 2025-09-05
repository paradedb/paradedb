use std::fmt::Display;
use std::str::FromStr;
use pgrx::{extension_sql, pg_sys, Spi};
use pgrx::datum::DatumWithOid;
use thiserror::Error;

#[derive(Error)]
pub enum TypmodError {
    #[error("missing key")]
    MissingKey,

    #[error("invalid property name: {0}")]
    InvalidProperty(String),

    #[error("invalid regex: /{0}/")]
    InvalidRegex(regex::Error),

    #[error("SPI failure: {0}")]
    SpiError(pgrx::spi::Error),
}


pub type PropertyKey = String;
#[derive(Debug)]
pub enum Property {
    None(PropertyKey),
    String(PropertyKey, String),
    Regex(PropertyKey, regex::Regex),
    Integer(PropertyKey, i64),
    Boolean(PropertyKey, bool),
}

impl Display for Property {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Property::None(key) => write!(f, "{key}"),
            Property::String(key, s) => write!(f, "{key}={}", s.replace("'", "''")),
            Property::Regex(key, r) => write!(f, "{key}=/{}/", r.as_str().replace("'", "''")),
            Property::Integer(key, i) => write!(f, "{key}={i}"),
            Property::Boolean(key, b) => write!(f, "{key}={b}"),
        }
    }
}

impl FromStr for Property {
    type Err = TypmodError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(1, '=');
        let key = parts.next().ok_or(TypmodError::MissingKey)?.to_string();
        let value = parts.next();

        match value {
            None => Ok(Property::None(key.to_string())),
            Some(s) => {
                if s.starts_with('/') && s.ends_with('/') {
                    let regex = s.trim_matches('/').to_string();
                    Ok(Property::Regex(
                        key,
                        regex::Regex::new(&regex).map_err(|e| TypmodError::InvalidRegex(e))?,
                    ))
                } else if s == "true" || s == "false" {
                    Ok(Property::Boolean(key, s == "true"))
                } else if let Ok(i) = s.parse::<i64>() {
                    Ok(Property::Integer(key, i))
                } else {
                    Ok(Property::String(key, s.to_string()))
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ParsedTypmod {
    properties: Vec<Property>,
}

impl Display for ParsedTypmod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, prop) in self.properties.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{prop}")?;
        }
        Ok(())
    }
}

impl ParsedTypmod {
    pub fn new() -> Self {
        Self { properties: vec![] }
    }

    pub fn add_property(&mut self, property: Property) {
        self.properties.push(property);
    }
}

pub fn load_typmod(typmod: i32) -> pgrx::spi::Result<ParsedTypmod> {
    let value = Spi::get_one_with_args::<String>("SELECT paradedb._typmod($1)", &[unsafe {DatumWithOid::new(typmod, pg_sys::INT4OID)}])?;
    let parsed =
}


extension_sql!(
    r#"
CREATE TABLE paradedb._typmod_cache(id SERIAL NOT NULL PRIMARY KEY, regex TEXT NOT NULL UNIQUE);
SELECT pg_catalog.pg_extension_config_dump('paradedb._typmod_cache', '');
SELECT pg_catalog.pg_extension_config_dump('paradedb._typmod_cache_id_seq', '');
GRANT ALL ON TABLE paradedb._typmod_cache TO PUBLIC;
GRANT ALL ON SEQUENCE paradedb._typmod_cache_id_seq TO PUBLIC;

CREATE OR REPLACE FUNCTION _typmod(int)
RETURNS text
SECURITY DEFINER
LANGUAGE sql AS $$
   SELECT regex FROM paradedb._typmod_cache WHERE id = $1;
$$;

CREATE OR REPLACE FUNCTION _typmod(text)
RETURNS integer
SECURITY DEFINER
LANGUAGE sql AS $$
  WITH
  -- serialize by regex so only one session can decide/insert at a time
  lock AS (
    SELECT pg_advisory_xact_lock(hashtext($1)::bigint)
  ),
  ins AS (
    INSERT INTO paradedb._typmod_cache (regex)
    SELECT $1
    WHERE NOT EXISTS (
      SELECT 1 FROM paradedb._typmod_cache WHERE regex = $1
    )
    RETURNING id
  )
  -- if we inserted, return that id, otherwise return the existing id
  SELECT id FROM ins
  UNION ALL
  SELECT id FROM paradedb._typmod_cache WHERE regex = $1
  LIMIT 1;
$$;
"#,
    name = "typmod_cache"
);

