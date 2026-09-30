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
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::spi::{OwnedPreparedStatement, Query};
use pgrx::{
    Array, PgLogLevel, PgOid, PgSqlErrorCode, PgSubXactCallbackEvent, PgXactCallbackEvent, Spi,
    extension_sql, function_name, pg_extern, pg_sys, register_subxact_callback,
    register_xact_callback,
};
use std::cell::{Cell, RefCell};
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
    save_typmod(typmod_parts.iter()).unwrap_or_else(|e| e.report())
}

#[pg_extern(immutable, parallel_safe)]
pub fn generic_typmod_out(typmod: i32) -> CString {
    let parsed = load_typmod(typmod).unwrap_or_else(|e| e.report());

    // make sure the typmods are string-quoted literals
    let mut parts = Vec::with_capacity(parsed.len());
    for prop in parsed.properties.iter() {
        let s = prop.to_string();
        parts.push(format!("'{}'", s));
    }

    CString::new(format!("({})", parts.join(", "))).unwrap()
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

impl Error {
    /// Raises this error as a Postgres `ERROR`, worded for the user who hit it.
    pub(crate) fn report(self) -> ! {
        let Error::TypmodNotFound(id) = self else {
            pgrx::error!("{self}");
        };
        ErrorReport::new(
            PgSqlErrorCode::ERRCODE_INTERNAL_ERROR,
            "stored tokenizer options could not be found",
            function_name!(),
        )
        .set_detail(format!("paradedb._typmod_cache has no entry with id {id}."))
        .report(PgLogLevel::ERROR);
        unreachable!("an ERROR report does not return")
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
    Float(PropertyKey, f64),
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
            Property::Float(Some(key), v) => write!(f, "{key}={v}"),
            Property::Boolean(Some(key), b) => write!(f, "{key}={b}"),

            Property::None(None) => write!(f, ""),
            Property::String(None, s) => write!(f, "{}", s.replace("'", "''")),
            Property::Regex(None, r) => write!(f, "/{}/", r.as_str().replace("'", "''")),
            Property::Integer(None, i) => write!(f, "{i}"),
            Property::Float(None, v) => write!(f, "{v}"),
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
                } else if let Ok(v) = s.parse::<f64>() {
                    Ok(Property::Float(key, v))
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
            | Property::Float(key, _)
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

    /// Truncates f64 → f32 and i64 → f32. Safe for small magnitudes;
    /// do not use for values where precision matters.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Property::Float(_, v) => Some(*v as f32),
            Property::Integer(_, i) => Some(*i as f32),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Property::Float(_, v) => Some(*v),
            Property::Integer(_, i) => Some(*i as f64),
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
                    "czech" => Ok(Language::Czech),
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
                            "czech" => Ok(Language::Czech),
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
                        "czech" => Language::Czech,
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
                Property::Float(key, _) if key.as_deref() == Some(index) => return prop,
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

/// A prepared statement kept for the life of the backend.
///
/// It has no lock around it: the SQL it runs can call back into the same function, and a lock
/// held while the statement executes would make that inner call wait on itself forever.
#[repr(transparent)]
struct StmtHolder(OwnedPreparedStatement);

// SAFETY:  we don't do threads in postgres
unsafe impl Send for StmtHolder {}
unsafe impl Sync for StmtHolder {}

static LOAD_CACHE: OnceLock<Mutex<crate::api::HashMap<i32, ParsedTypmod>>> = OnceLock::new();
static SAVE_CACHE: OnceLock<Mutex<crate::api::HashMap<Vec<String>, i32>>> = OnceLock::new();

/// A cache entry added in the current transaction.
enum AddedEntry {
    Load(i32),
    Save(Vec<String>),
}

thread_local! {
    static XACT_CALLBACKS_REGISTERED: Cell<bool> = const { Cell::new(false) };
    /// The entries added in the current transaction, each with the transaction nesting level it
    /// was added at, so that an abort removes those entries and keeps everything cached before.
    static ADDED: RefCell<Vec<(i32, AddedEntry)>> = const { RefCell::new(Vec::new()) };
}

/// Records `entry` so that aborting the (sub)transaction it is added in removes it again.
fn remember_added(entry: AddedEntry) {
    ensure_xact_callbacks_registered();
    let level = unsafe { pg_sys::GetCurrentTransactionNestLevel() };
    ADDED.with_borrow_mut(|added| added.push((level, entry)));
}

/// Removes the entries added at nesting level `level` or deeper from the caches.
fn forget_added(level: i32) {
    let removed: Vec<_> = ADDED.with_borrow_mut(|added| {
        added
            .extract_if(.., |(entry_level, _)| *entry_level >= level)
            .collect()
    });
    for (_, entry) in removed {
        match entry {
            AddedEntry::Load(typmod) => {
                if let Some(cache) = LOAD_CACHE.get() {
                    cache.lock().remove(&typmod);
                }
            }
            AddedEntry::Save(key) => {
                if let Some(cache) = SAVE_CACHE.get() {
                    cache.lock().remove(&key);
                }
            }
        }
    }
}

fn ensure_xact_callbacks_registered() {
    if XACT_CALLBACKS_REGISTERED.get() {
        return;
    }

    // pgrx removes these callbacks at the end of the top-level transaction, so they are
    // registered again when the next transaction adds its first entry.
    register_xact_callback(PgXactCallbackEvent::Commit, || {
        ADDED.with_borrow_mut(Vec::clear);
        XACT_CALLBACKS_REGISTERED.set(false);
    });
    register_xact_callback(PgXactCallbackEvent::Abort, || {
        forget_added(0);
        XACT_CALLBACKS_REGISTERED.set(false);
    });
    register_subxact_callback(PgSubXactCallbackEvent::AbortSub, |_, _| {
        forget_added(unsafe { pg_sys::GetCurrentTransactionNestLevel() });
    });
    // A released savepoint's entries now belong to its parent, as its row changes do, so
    // rolling back a later sibling savepoint must not remove them.
    register_subxact_callback(PgSubXactCallbackEvent::CommitSub, |_, _| {
        let level = unsafe { pg_sys::GetCurrentTransactionNestLevel() };
        ADDED.with_borrow_mut(|added| {
            for (entry_level, _) in added.iter_mut() {
                if *entry_level == level {
                    *entry_level = level - 1;
                }
            }
        });
    });
    XACT_CALLBACKS_REGISTERED.set(true);
}

pub fn load_typmod(typmod: i32) -> Result<ParsedTypmod> {
    if typmod == -1 {
        return Ok(ParsedTypmod::new());
    }

    // Don't hold the cache across SPI: a FATAL exits without unwinding, so the guard would
    // never drop, and an abort callback would block on it forever during exit.
    let cache = LOAD_CACHE.get_or_init(Default::default);
    if let Some(parsed_typmod) = cache.lock().get(&typmod) {
        return Ok(parsed_typmod.clone());
    }

    let parsed_typmod = ParsedTypmod::try_from(
        Spi::connect(|client| {
            static STMT: OnceLock<StmtHolder> = OnceLock::new();

            let prepared = STMT.get_or_init(|| {
                StmtHolder(
                    client
                        .prepare(
                            "SELECT typmod FROM paradedb._typmod_cache WHERE id = $1",
                            &[PgOid::BuiltIn(BuiltinOid::INT4OID)],
                        )
                        .expect("failed to prepare statement")
                        .keep(),
                )
            });

            let datum = unsafe { [DatumWithOid::new(typmod, pg_sys::INT4OID)] };
            let rows = (&prepared.0).execute(client, None, &datum)?;
            if rows.is_empty() {
                return Ok(None);
            }
            rows.first().get::<Vec<String>>(1)
        })?
        .ok_or_else(|| Error::TypmodNotFound(typmod))?,
    )?;

    remember_added(AddedEntry::Load(typmod));
    cache.lock().insert(typmod, parsed_typmod.clone());
    Ok(parsed_typmod)
}

pub fn save_typmod<'a>(typmod: impl Iterator<Item = Option<&'a CStr>>) -> Result<i32> {
    let as_text = typmod
        .map(|e| {
            e.ok_or(Error::EmptyProperty)
                .map(|e| e.to_str().unwrap().to_string())
        })
        .collect::<Result<Vec<_>>>()?;

    // Not held across SPI, for the same reason as in `load_typmod`.
    let cache = SAVE_CACHE.get_or_init(Default::default);
    if let Some(id) = cache.lock().get(&as_text) {
        return Ok(*id);
    }

    let datum = unsafe { [DatumWithOid::new(as_text.clone(), pg_sys::TEXTARRAYOID)] };

    let id = Spi::connect(|client| {
        static STMT: OnceLock<StmtHolder> = OnceLock::new();

        let prepared = STMT.get_or_init(|| {
            StmtHolder(
                client
                    .prepare(
                        "SELECT id FROM paradedb._typmod_cache WHERE typmod = $1",
                        &[PgOid::BuiltIn(BuiltinOid::TEXTARRAYOID)],
                    )
                    .expect("failed to prepare statement")
                    .keep(),
            )
        });

        let rows = (&prepared.0).execute(client, None, &datum)?;
        if rows.is_empty() {
            return Ok(None);
        }
        rows.first().get::<i32>(1)
    })?;

    let id = match id {
        Some(id) => id,
        None => Spi::get_one_with_args::<i32>("SELECT paradedb._save_typmod($1)", &datum)?
            .ok_or(Error::NullTypmodEntry)?,
    };
    remember_added(AddedEntry::Save(as_text.clone()));
    cache.lock().insert(as_text, id);
    Ok(id)
}

extension_sql!(
    r#"
CREATE TABLE paradedb._typmod_cache(id SERIAL NOT NULL PRIMARY KEY, typmod text[] NOT NULL UNIQUE);
SELECT pg_catalog.pg_extension_config_dump('paradedb._typmod_cache', '');
SELECT pg_catalog.pg_extension_config_dump('paradedb._typmod_cache_id_seq', '');

-- The typmod cache is shared extension state that every user's ParadeDB indexes
-- resolve their tokenizer configuration through. Ordinary roles only ever read
-- it (via SPI in load_typmod) and insert into it through the SECURITY DEFINER
-- function paradedb._save_typmod below, so grant no more than SELECT to PUBLIC.
-- Writes (INSERT/UPDATE/DELETE/TRUNCATE) stay with the table owner; letting
-- PUBLIC mutate rows would let any role silently repoint or orphan the typmod
-- IDs that other users' indexes depend on.
--
-- Both relations are registered with pg_extension_config_dump above, so pg_dump
-- reads the sequence's last_value directly. SELECT on a sequence permits that
-- but not nextval/setval, so PUBLIC keeps enough to dump and nothing more; the
-- sequence is only ever advanced from inside _save_typmod, as its owner.
GRANT SELECT ON TABLE paradedb._typmod_cache TO PUBLIC;
GRANT SELECT ON SEQUENCE paradedb._typmod_cache_id_seq TO PUBLIC;

-- search_path is pinned so this SECURITY DEFINER function's name resolution
-- can't be redirected by a caller-controlled search_path. The body only touches
-- the schema-qualified paradedb._typmod_cache plus pg_catalog operators.
CREATE OR REPLACE FUNCTION paradedb._save_typmod(typmod_in text[])
RETURNS integer SECURITY DEFINER STRICT VOLATILE PARALLEL UNSAFE
SET search_path = pg_catalog, pg_temp
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
