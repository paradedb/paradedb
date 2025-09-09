mod definitions;

use pgrx::datum::DatumWithOid;
use pgrx::spi::Query;
use pgrx::{extension_sql, pg_sys, Array, PgOid, Spi};
use std::ffi::CStr;
use std::fmt::Display;
use std::ops::Index;
use std::str::FromStr;
use tantivy::tokenizer::Language;
use thiserror::Error;
use tokenizers::manager::SearchTokenizerFilters;

pub use definitions::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("typmod not found: {0}")]
    TypmodNotFound(i32),

    #[error("missing key: {0}")]
    MissingKey(&'static str),

    #[error("empty property")]
    EmptyProperty,

    #[error("invalid property name: {0}")]
    InvalidProperty(Property),

    #[error("property not utf8")]
    InvalidPropertyUtf8(#[from] std::str::Utf8Error),

    #[error("invalid regex: /{0}/")]
    InvalidRegex(regex::Error),

    #[error("invalid language: {0}")]
    InvalidLanguage(String),

    #[error("SPI failure: {0}")]
    SpiError(#[from] pgrx::spi::Error),

    #[error("null typmod entry")]
    NullTypmodEntry,
}

pub type Result<T> = std::result::Result<T, Error>;

pub type PropertyKey = Option<String>;
#[derive(Debug, Clone)]
pub enum Property {
    NoSuchProperty,
    None(PropertyKey),
    String(PropertyKey, String),
    Regex(PropertyKey, regex::Regex),
    Integer(PropertyKey, i64),
    Boolean(PropertyKey, bool),
}

impl Display for Property {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Property::NoSuchProperty => panic!("cannot display `Property::NoSuchProperty`"),
            Property::None(Some(key)) => write!(f, "{key}"),
            Property::String(Some(key), s) => write!(f, "{key}={}", s.replace("'", "''")),
            Property::Regex(Some(key), r) => write!(f, "{key}=/{}/", r.as_str().replace("'", "''")),
            Property::Integer(Some(key), i) => write!(f, "{key}={i}"),
            Property::Boolean(Some(key), b) => write!(f, "{key}={b}"),

            Property::None(None) => write!(f, ""),
            Property::String(None, s) => write!(f, "{}", s.replace("'", "''")),
            Property::Regex(None, r) => write!(f, "/{}/", r.as_str().replace("'", "''")),
            Property::Integer(None, i) => write!(f, "{i}"),
            Property::Boolean(None, b) => write!(f, "{b}"),
        }
    }
}

impl FromStr for Property {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut parts = s.splitn(2, '=');
        let mut key = parts.next();
        let mut value = parts.next();

        if key.is_some() && value.is_none() {
            value = key;
            key = None;
        }

        let key = key.map(String::from);
        match value {
            None => Ok(Property::None(key)),
            Some(s) => {
                if s.starts_with('/') && s.ends_with('/') {
                    let regex = s.trim_matches('/').to_string();
                    Ok(Property::Regex(
                        key,
                        regex::Regex::new(&regex).map_err(|e| Error::InvalidRegex(e))?,
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

impl Property {
    pub fn as_usize(&self) -> Option<usize> {
        match self {
            Property::Integer(_, i) => Some(*i as usize),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Property::Boolean(_, b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Property::String(_, s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_regex(&self) -> Option<Result<regex::Regex>> {
        match self {
            Property::Regex(_, r) => Some(Ok(r.clone())),
            Property::String(_, s) => {
                Some(regex::Regex::new(s.as_str()).map_err(|e| Error::InvalidRegex(e)))
            }
            _ => None,
        }
    }

    pub fn as_language(&self) -> Result<Language> {
        match self {
            Property::String(_, stemmer) => {
                let lcase = stemmer.to_lowercase();
                match lcase.as_str() {
                    "arabic" => Ok(Language::Arabic),
                    "danish" => Ok(Language::Danish),
                    "dutch" => Ok(Language::Dutch),
                    "english" => Ok(Language::English),
                    "finnish" => Ok(Language::Finnish),
                    "french" => Ok(Language::French),
                    "german" => Ok(Language::German),
                    "greek" => Ok(Language::Greek),
                    "hungarian" => Ok(Language::Hungarian),
                    "italian" => Ok(Language::Italian),
                    "norwegian" => Ok(Language::Norwegian),
                    "portuguese" => Ok(Language::Portuguese),
                    "romanian" => Ok(Language::Romanian),
                    "russian" => Ok(Language::Russian),
                    "spanish" => Ok(Language::Spanish),
                    "swedish" => Ok(Language::Swedish),
                    "tamil" => Ok(Language::Tamil),
                    "turkish" => Ok(Language::Turkish),
                    other => Err(Error::InvalidLanguage(other.to_string())),
                }
            }
            _ => Err(Error::InvalidProperty(self.clone())),
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

impl TryFrom<Vec<String>> for ParsedTypmod {
    type Error = Error;

    fn try_from(value: Vec<String>) -> std::result::Result<Self, Self::Error> {
        let mut parsed = ParsedTypmod::with_capacity(value.len());

        for entry in value {
            let property: Property = (&entry).parse()?;
            parsed.add_property(property);
        }

        Ok(parsed)
    }
}

impl<'mcx> TryFrom<Array<'mcx, &'mcx CStr>> for ParsedTypmod {
    type Error = Error;
    fn try_from(value: Array<'mcx, &'mcx CStr>) -> std::result::Result<Self, Self::Error> {
        let mut parsed = ParsedTypmod::with_capacity(value.len());
        for entry in value.iter() {
            match entry {
                None => parsed.add_property(Property::None(None)),
                Some(e) => {
                    let s = e.to_str()?;
                    let property: Property = s.parse()?;
                    parsed.add_property(property)
                }
            }
        }
        Ok(parsed)
    }
}

impl From<&ParsedTypmod> for SearchTokenizerFilters {
    fn from(value: &ParsedTypmod) -> Self {
        SearchTokenizerFilters {
            remove_long: value.get("remove_long").map(|p| p.as_usize()).flatten(),
            lowercase: value.get("lowercase").map(|p| p.as_bool()).flatten(),
            stemmer: value
                .get("stemmer")
                .map(|p| p.as_str())
                .flatten()
                .map(|stemmer| {
                    let lcase = stemmer.to_lowercase();
                    match lcase.as_str() {
                        "arabic" => Language::Arabic,
                        "danish" => Language::Danish,
                        "dutch" => Language::Dutch,
                        "english" => Language::English,
                        "finnish" => Language::Finnish,
                        "french" => Language::French,
                        "german" => Language::German,
                        "greek" => Language::Greek,
                        "hungarian" => Language::Hungarian,
                        "italian" => Language::Italian,
                        "norwegian" => Language::Norwegian,
                        "portuguese" => Language::Portuguese,
                        "romanian" => Language::Romanian,
                        "russian" => Language::Russian,
                        "spanish" => Language::Spanish,
                        "swedish" => Language::Swedish,
                        "tamil" => Language::Tamil,
                        "turkish" => Language::Turkish,
                        other => panic!("unknown stemmer: {}", other),
                    }
                }),
            // TODO: handle stopwords
            stopwords_language: None,
            stopwords: None,
        }
    }
}

impl Index<usize> for ParsedTypmod {
    type Output = Property;
    fn index(&self, index: usize) -> &Self::Output {
        &self.properties[index]
    }
}

impl<'a> Index<&'a str> for ParsedTypmod {
    type Output = Property;

    fn index(&self, index: &'a str) -> &Self::Output {
        for prop in self.properties.iter() {
            match prop {
                Property::None(key) if key.as_deref() == Some(index) => return prop,
                Property::String(key, _) if key.as_deref() == Some(index) => return prop,
                Property::Regex(key, _) if key.as_deref() == Some(index) => return prop,
                Property::Integer(key, _) if key.as_deref() == Some(index) => return prop,
                Property::Boolean(key, _) if key.as_deref() == Some(index) => return prop,

                _ => {}
            }
        }
        // TODO:  is this smart or too clever?
        &Property::NoSuchProperty
    }
}

impl ParsedTypmod {
    pub fn new() -> Self {
        Self { properties: vec![] }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            properties: Vec::with_capacity(capacity),
        }
    }

    pub fn add_property(&mut self, property: Property) {
        self.properties.push(property);
    }

    pub fn get(&self, key: &str) -> Option<&Property> {
        let prop = &self[key];
        if matches!(prop, Property::NoSuchProperty) {
            return None;
        }
        Some(prop)
    }

    pub fn try_get(&self, key: &str, index: usize) -> Option<&Property> {
        let prop = &self[key];
        if matches!(prop, Property::NoSuchProperty) {
            return self.properties.get(index);
        }
        Some(prop)
    }
}

pub fn load_typmod(typmod: i32) -> Result<ParsedTypmod> {
    ParsedTypmod::try_from(
        Spi::get_one_with_args::<Vec<String>>("SELECT paradedb._typmod($1)", unsafe {
            &[DatumWithOid::new(typmod, pg_sys::INT4OID)]
        })?
        .ok_or_else(|| Error::TypmodNotFound(typmod))?,
    )
}

pub fn save_typmod<'a>(typmod: impl Iterator<Item = Option<&'a CStr>>) -> Result<i32> {
    let as_text = typmod
        .map(|e| e.ok_or(Error::EmptyProperty).map(|e| e.to_str().unwrap()))
        .collect::<Result<Vec<_>>>()?;

    let id = Spi::get_one_with_args::<i32>("SELECT paradedb._typmod($1)", unsafe {
        &[DatumWithOid::new(as_text, pg_sys::TEXTARRAYOID)]
    })?
    .ok_or(Error::NullTypmodEntry)?;
    Ok(id)
}

extension_sql!(
    r#"
CREATE TABLE paradedb._typmod_cache(id SERIAL NOT NULL PRIMARY KEY, typmod text[] NOT NULL UNIQUE);
SELECT pg_catalog.pg_extension_config_dump('paradedb._typmod_cache', '');
SELECT pg_catalog.pg_extension_config_dump('paradedb._typmod_cache_id_seq', '');
GRANT ALL ON TABLE paradedb._typmod_cache TO PUBLIC;
GRANT ALL ON SEQUENCE paradedb._typmod_cache_id_seq TO PUBLIC;

CREATE OR REPLACE FUNCTION paradedb._typmod(typmod_in text[])
RETURNS integer SECURITY DEFINER STRICT VOLATILE PARALLEL UNSAFE
LANGUAGE plpgsql AS $$
DECLARE
    v_id integer;
BEGIN
    SELECT id INTO v_id
    FROM paradedb._typmod_cache
    WHERE typmod = typmod_in;

    IF v_id IS NOT NULL THEN
        RETURN v_id;
    END IF;

    -- not been inserted yet
    INSERT INTO paradedb._typmod_cache (typmod)
    VALUES (typmod_in)
    ON CONFLICT (typmod) DO NOTHING
    RETURNING id INTO v_id;

    IF v_id IS NOT NULL THEN
        RETURN v_id;
    END IF;

    -- someone else inserted it concurrently, go read it again
    SELECT id INTO v_id
    FROM paradedb._typmod_cache
    WHERE typmod = typmod_in;

    IF v_id IS NULL THEN
        RAISE EXCEPTION 'typmod "%" not found after upsert', typmod_in;
    END IF;

    RETURN v_id;
END;
$$;

CREATE OR REPLACE FUNCTION paradedb._typmod(int)
RETURNS text[]
SECURITY DEFINER PARALLEL SAFE STRICT LANGUAGE SQL AS $$
    SELECT typmod FROM paradedb._typmod_cache WHERE id = $1;
$$;

"#,
    name = "typmod_cache"
);
