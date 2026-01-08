// Copyright (c) 2023-2026 ParadeDB, Inc.
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

mod definitions;
mod validation;

use parking_lot::Mutex;
use pgrx::datum::DatumWithOid;
use pgrx::pg_sys::BuiltinOid;
use pgrx::spi::{OwnedPreparedStatement, Query};
use pgrx::{
    extension_sql, pg_extern, pg_sys, register_xact_callback, Array, PgOid, PgXactCallbackEvent,
    Spi,
};
use std::collections::hash_map::Entry;
use std::ffi::{CStr, CString};
use std::fmt::Display;
use std::ops::Index;
use std::str::FromStr;
use std::sync::OnceLock;
use tantivy::tokenizer::Language;
use thiserror::Error;
use tokenizers::manager::SearchTokenizerFilters;
pub use validation::{TypmodSchema, ValidationError};

pub use definitions::*;
use tokenizers::SearchNormalizer;

#[pg_extern(immutable, parallel_safe)]
fn generic_typmod_in(typmod_parts: Array<&CStr>) -> i32 {
    save_typmod(typmod_parts.iter()).expect("should not fail to save typmod")
}

#[pg_extern(immutable, parallel_safe)]
pub fn generic_typmod_out(typmod: i32) -> CString {
    let parsed = load_typmod(typmod).expect("should not fail to load typmod");
    CString::new(format!("({parsed})")).unwrap()
}

pub type Typmod = i32;

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
    Spi(#[from] pgrx::spi::Error),

    #[error("null typmod entry")]
    NullTypmodEntry,

    #[error("paradedb._typmod_cache table is missing")]
    MissingTypmodCache,

    #[error("{0}")]
    Validation(ValidationError),
}

impl From<ValidationError> for Error {
    fn from(err: ValidationError) -> Self {
        Error::Validation(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub type PropertyKey = Option<String>;
#[derive(Debug, Clone)]
pub enum Property {
    #[allow(clippy::enum_variant_names)] // what a stupid lint
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
                        regex::Regex::new(&regex).map_err(Error::InvalidRegex)?,
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
    pub fn key(&self) -> Option<&str> {
        match self {
            Property::NoSuchProperty => None,
            Property::None(key)
            | Property::String(key, _)
            | Property::Regex(key, _)
            | Property::Integer(key, _)
            | Property::Boolean(key, _) => key.as_deref(),
        }
    }

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
                Some(regex::Regex::new(s.as_str()).map_err(Error::InvalidRegex))
            }
            _ => None,
        }
    }

    pub fn as_normalizer(&self) -> Option<SearchNormalizer> {
        if let Some(s) = self.as_str() {
            let lcase = s.to_lowercase();
            match lcase.as_str() {
                "raw" => Some(SearchNormalizer::Raw),
                "lowercase" => Some(SearchNormalizer::Lowercase),
                _ => None,
            }
        } else {
            None
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
                    "polish" => Ok(Language::Polish),
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

    /// Parse comma-separated languages (e.g., "English,French")
    pub fn as_languages(&self) -> Result<Vec<Language>> {
        match self {
            Property::String(_, value) => {
                let languages: std::result::Result<Vec<_>, _> = value
                    .split(',')
                    .map(|s| {
                        let lcase = s.trim().to_lowercase();
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
                            "polish" => Ok(Language::Polish),
                            "portuguese" => Ok(Language::Portuguese),
                            "romanian" => Ok(Language::Romanian),
                            "russian" => Ok(Language::Russian),
                            "spanish" => Ok(Language::Spanish),
                            "swedish" => Ok(Language::Swedish),
                            "tamil" => Ok(Language::Tamil),
                            "turkish" => Ok(Language::Turkish),
                            other => Err(Error::InvalidLanguage(other.to_string())),
                        }
                    })
                    .collect();
                languages
            }
            _ => Err(Error::InvalidProperty(self.clone())),
        }
    }
}

#[derive(Debug, Clone)]
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
            let property: Property = entry.parse()?;
            parsed.add_property(property);
        }

        Ok(parsed)
    }
}

impl<'mcx> TryFrom<&Array<'mcx, &'mcx CStr>> for ParsedTypmod {
    type Error = Error;
    fn try_from(value: &Array<'mcx, &'mcx CStr>) -> std::result::Result<Self, Self::Error> {
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
            remove_long: value.get("remove_long").and_then(|p| p.as_usize()),
            remove_short: value.get("remove_short").and_then(|p| p.as_usize()),
            lowercase: value.get("lowercase").and_then(|p| p.as_bool()),
            stemmer: value
                .get("stemmer")
                .and_then(|p| p.as_str())
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
                        "polish" => Language::Polish,
                        "portuguese" => Language::Portuguese,
                        "romanian" => Language::Romanian,
                        "russian" => Language::Russian,
                        "spanish" => Language::Spanish,
                        "swedish" => Language::Swedish,
                        "tamil" => Language::Tamil,
                        "turkish" => Language::Turkish,
                        other => panic!("unknown stemmer: {other}"),
                    }
                }),
            stopwords_language: value
                .get("stopwords_language")
                .and_then(|p| p.as_languages().ok()),
            stopwords: None, // TODO: handle stopwords list in a new way we haven't done up to this point
            alpha_num_only: value.get("alpha_num_only").and_then(|p| p.as_bool()),
            ascii_folding: value.get("ascii_folding").and_then(|p| p.as_bool()),
            trim: value.get("trim").and_then(|p| p.as_bool()),
            normalizer: value.get("normalizer").and_then(|p| p.as_normalizer()),
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

impl Default for ParsedTypmod {
    fn default() -> Self {
        Self::new()
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

    pub fn len(&self) -> usize {
        self.properties.len()
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

#[repr(transparent)]
struct StmtHolder(OwnedPreparedStatement);

// SAFETY:  we don't do threads in postgres
unsafe impl Send for StmtHolder {}
unsafe impl Sync for StmtHolder {}

pub fn load_typmod(typmod: i32) -> Result<ParsedTypmod> {
    static CACHE: OnceLock<Mutex<crate::api::HashMap<i32, ParsedTypmod>>> = OnceLock::new();

    if typmod == -1 {
        return Ok(ParsedTypmod::new());
    }

    let cache = CACHE.get_or_init(Default::default);
    let mut locked = cache.lock();

    match locked.entry(typmod) {
        Entry::Occupied(e) => Ok(e.get().clone()),
        Entry::Vacant(e) => {
            let parsed_typmod = ParsedTypmod::try_from(
                Spi::connect(|client| {
                    static STMT: OnceLock<Mutex<StmtHolder>> = OnceLock::new();

                    let prepared = STMT.get_or_init(|| {
                        Mutex::new(StmtHolder(
                            client
                                .prepare(
                                    "SELECT typmod FROM paradedb._typmod_cache WHERE id = $1",
                                    &[PgOid::BuiltIn(BuiltinOid::INT4OID)],
                                )
                                .expect("failed to prepare statement")
                                .keep(),
                        ))
                    });

                    let datum = unsafe { [DatumWithOid::new(typmod, pg_sys::INT4OID)] };
                    (&prepared.lock().0)
                        .execute(client, None, &datum)?
                        .first()
                        .get::<Vec<String>>(1)
                })?
                .ok_or_else(|| Error::TypmodNotFound(typmod))?,
            )?;

            e.insert(parsed_typmod.clone());
            register_xact_callback(PgXactCallbackEvent::Abort, move || {
                CACHE.get_or_init(Default::default).lock().remove(&typmod);
            });
            Ok(parsed_typmod)
        }
    }
}

pub fn save_typmod<'a>(typmod: impl Iterator<Item = Option<&'a CStr>>) -> Result<i32> {
    static CACHE: OnceLock<Mutex<crate::api::HashMap<Vec<String>, i32>>> = OnceLock::new();

    let as_text = typmod
        .map(|e| {
            e.ok_or(Error::EmptyProperty)
                .map(|e| e.to_str().unwrap().to_string())
        })
        .collect::<Result<Vec<_>>>()?;

    let cache = CACHE.get_or_init(Default::default);
    let mut locked = cache.lock();

    match locked.entry(as_text.clone()) {
        Entry::Occupied(e) => Ok(*e.get()),
        Entry::Vacant(e) => {
            let datum = unsafe { [DatumWithOid::new(e.key().clone(), pg_sys::TEXTARRAYOID)] };

            let id = Spi::connect(|client| {
                static STMT: OnceLock<Mutex<StmtHolder>> = OnceLock::new();

                let prepared = STMT.get_or_init(|| {
                    Mutex::new(StmtHolder(
                        client
                            .prepare(
                                "SELECT id FROM paradedb._typmod_cache WHERE typmod = $1",
                                &[PgOid::BuiltIn(BuiltinOid::TEXTARRAYOID)],
                            )
                            .expect("failed to prepare statement")
                            .keep(),
                    ))
                });

                let datum = unsafe { [DatumWithOid::new(as_text.clone(), pg_sys::TEXTARRAYOID)] };
                (&prepared.lock().0)
                    .execute(client, None, &datum)?
                    .first()
                    .get::<i32>(1)
            })
            .ok()
            .flatten();

            let id = match id {
                Some(id) => id,
                None => {
                    let id =
                        Spi::get_one_with_args::<i32>("SELECT paradedb._save_typmod($1)", &datum)?
                            .ok_or(Error::NullTypmodEntry)?;

                    register_xact_callback(PgXactCallbackEvent::Abort, move || {
                        CACHE.get_or_init(Default::default).lock().remove(&as_text);
                    });

                    id
                }
            };
            e.insert(id);
            Ok(id)
        }
    }
}

extension_sql!(
    r#"
CREATE TABLE paradedb._typmod_cache(id SERIAL NOT NULL PRIMARY KEY, typmod text[] NOT NULL UNIQUE);
SELECT pg_catalog.pg_extension_config_dump('paradedb._typmod_cache', '');
SELECT pg_catalog.pg_extension_config_dump('paradedb._typmod_cache_id_seq', '');
GRANT ALL ON TABLE paradedb._typmod_cache TO PUBLIC;
GRANT ALL ON SEQUENCE paradedb._typmod_cache_id_seq TO PUBLIC;

CREATE OR REPLACE FUNCTION paradedb._save_typmod(typmod_in text[])
RETURNS integer SECURITY DEFINER STRICT VOLATILE PARALLEL UNSAFE
LANGUAGE plpgsql AS $$
DECLARE
    v_id integer;
BEGIN
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
"#,
    name = "typmod_cache"
);
