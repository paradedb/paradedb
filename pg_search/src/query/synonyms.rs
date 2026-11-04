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

//! Query-time synonym expansion.
//!
//! A set is the rows of `pdb.synonym_rules` that share a `set_name`, one Solr-format rule each.
//! A set is compiled against the analyzer of the query, so stemming and lowercasing apply to the
//! rules and to the query alike. The compiled form, an FST over the rule keys plus one blob of
//! alternatives, is kept per backend until a rule change drops it through a relcache
//! invalidation.

use crate::api::HashMap;
use crate::postgres::utils::ActiveSnapshotGuard;
use parking_lot::Mutex;
use pgrx::trigger_support::{PgTrigger, PgTriggerOperation};
use pgrx::{Spi, pg_guard, pg_sys};
use std::ffi::CStr;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use tantivy::tokenizer::TextAnalyzer;
use tantivy_fst::{Map, MapBuilder, Streamer};
use thiserror::Error;
use tokenizers::SearchTokenizer;

/// A side of a rule past this many terms compiles to the square of that many alternatives.
const MAX_RULE_TERMS: usize = 256;

#[derive(Debug, Error)]
pub enum SynonymError {
    #[error("synonym set '{0}' does not exist")]
    NotFound(String),
    #[error("synonym set '{set}' has an invalid rule '{rule}': {reason}")]
    InvalidRule {
        set: String,
        rule: String,
        reason: String,
    },
    #[error("the stored form of synonym set '{set}' does not load: {reason}")]
    Stored { set: String, reason: String },
    #[error(transparent)]
    Spi(#[from] pgrx::spi::Error),
    #[error(transparent)]
    Fst(#[from] tantivy_fst::Error),
}

/// One rule of a set, split into its terms but not yet analyzed.
#[derive(Debug, PartialEq, Eq)]
enum Rule {
    /// `a, b, c`. Every term expands to the whole group.
    Equivalent(Vec<String>),
    /// `a, b => c, d`. Every term on the left is replaced by the right side.
    Directed { from: Vec<String>, to: Vec<String> },
}

/// Splits a rule on `=>` and `,`. A backslash escapes the character after it, as in Solr, so a
/// term can contain a comma.
fn split_rule(rule: &str) -> Vec<Vec<String>> {
    let mut sides = vec![Vec::new()];
    let mut term = String::new();
    let mut chars = rule.chars().peekable();

    fn push(side: &mut Vec<String>, term: &mut String) {
        let trimmed = term.trim();
        if !trimmed.is_empty() {
            side.push(trimmed.to_string());
        }
        term.clear();
    }

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(escaped) = chars.next() {
                    term.push(escaped);
                }
            }
            ',' => push(sides.last_mut().unwrap(), &mut term),
            '=' if chars.peek() == Some(&'>') => {
                chars.next();
                push(sides.last_mut().unwrap(), &mut term);
                sides.push(Vec::new());
            }
            _ => term.push(c),
        }
    }
    push(sides.last_mut().unwrap(), &mut term);
    sides
}

/// Returns `None` for a blank line or a `#` comment, which Solr files contain.
fn parse_rule(rule: &str) -> Result<Option<Rule>, String> {
    let rule = rule.trim();
    if rule.is_empty() || rule.starts_with('#') {
        return Ok(None);
    }
    let mut sides = split_rule(rule);
    if sides.iter().any(|side| side.len() > MAX_RULE_TERMS) {
        return Err(format!(
            "a side of a rule can have at most {MAX_RULE_TERMS} terms"
        ));
    }
    match sides.len() {
        1 => Ok(Some(Rule::Equivalent(sides.pop().unwrap()))),
        2 => {
            let to = sides.pop().unwrap();
            let from = sides.pop().unwrap();
            if from.is_empty() || to.is_empty() {
                return Err("both sides of '=>' need at least one term".to_string());
            }
            Ok(Some(Rule::Directed { from, to }))
        }
        _ => Err("a rule can have at most one '=>'".to_string()),
    }
}

/// Why `rule` is not a Solr rule, for the domain check on `pdb.synonym_rules.rule`.
pub fn rule_error(rule: &str) -> Option<String> {
    parse_rule(rule).err()
}

pub fn tokenize(analyzer: &mut TextAnalyzer, text: &str) -> Vec<String> {
    let mut stream = analyzer.token_stream(text);
    let mut tokens = Vec::new();
    while stream.advance() {
        tokens.push(stream.token().text.clone());
    }
    tokens
}

/// A run of query tokens and what it expands to. `alternatives` is `None` when no rule matched.
#[derive(Debug, PartialEq, Eq)]
pub struct Expansion {
    pub tokens: Vec<String>,
    pub alternatives: Option<Vec<Vec<String>>>,
}

impl Expansion {
    /// True when the expansion builds one `TermQuery`, the shape Block-WAND prunes. A token in
    /// no rule, or a rule whose one alternative is one word, is a single term.
    #[allow(dead_code)]
    pub fn is_single_term(&self) -> bool {
        match &self.alternatives {
            None => true,
            Some(alternatives) => alternatives.len() == 1 && alternatives[0].len() == 1,
        }
    }

    /// Keeps at most `max` alternatives, in rule order. The typed tokens survive the cap, so a
    /// long rule never drops what the user typed.
    #[allow(dead_code)]
    pub fn cap(&mut self, max: usize) {
        let Some(alternatives) = &mut self.alternatives else {
            return;
        };
        if alternatives.len() <= max {
            return;
        }
        match alternatives.iter().position(|a| *a == self.tokens) {
            Some(typed) if typed >= max => {
                let typed = alternatives.remove(typed);
                alternatives.truncate(max - 1);
                alternatives.push(typed);
            }
            _ => alternatives.truncate(max),
        }
    }
}

/// The first byte of the alternatives blob. A new layout gets a new build stamp as well, so an
/// old row is never decoded with it.
const BLOB_FORMAT: u8 = 1;

/// Joins the tokens of a key. Postgres text never contains a zero byte, so no token does.
const KEY_SEPARATOR: u8 = 0;

fn encode_key(tokens: &[String]) -> Vec<u8> {
    tokens
        .iter()
        .map(String::as_bytes)
        .collect::<Vec<_>>()
        .join(&KEY_SEPARATOR)
}

fn put_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn get_varint(bytes: &[u8], pos: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    let mut shift = 0;
    loop {
        let byte = *bytes.get(*pos)?;
        *pos += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
}

/// A set compiled against one analyzer. `keys` maps the analyzed tokens of a rule's left side
/// to the offset of its alternatives in `blob`, the `SynonymMap` layout from Lucene. Both parts
/// are plain bytes, so a `pdb.synonym_fsts` row holds them as they are.
pub struct SynonymFst {
    keys: Map<Vec<u8>>,
    blob: Vec<u8>,
    max_key_tokens: usize,
}

impl std::fmt::Debug for SynonymFst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SynonymFst")
            .field("keys", &self.keys.len())
            .field("blob_bytes", &self.blob.len())
            .finish()
    }
}

impl SynonymFst {
    pub fn compile(
        set: &str,
        rules: &[String],
        analyzer: &mut TextAnalyzer,
    ) -> Result<Self, SynonymError> {
        let mut map: HashMap<Vec<String>, Vec<Vec<String>>> = HashMap::default();
        let mut analyze_all = |terms: &[String]| -> Vec<Vec<String>> {
            terms
                .iter()
                .map(|term| tokenize(analyzer, term))
                .filter(|tokens| !tokens.is_empty())
                .collect()
        };
        fn add(
            map: &mut HashMap<Vec<String>, Vec<Vec<String>>>,
            key: Vec<String>,
            alternatives: &[Vec<String>],
        ) {
            let entry = map.entry(key).or_default();
            for alternative in alternatives {
                if !entry.contains(alternative) {
                    entry.push(alternative.clone());
                }
            }
        }

        for raw in rules {
            let rule = parse_rule(raw).map_err(|reason| SynonymError::InvalidRule {
                set: set.to_string(),
                rule: raw.clone(),
                reason,
            })?;
            match rule {
                None => {}
                Some(Rule::Equivalent(terms)) => {
                    let group = analyze_all(&terms);
                    for key in &group {
                        add(&mut map, key.clone(), &group);
                    }
                }
                Some(Rule::Directed { from, to }) => {
                    let to = analyze_all(&to);
                    for key in analyze_all(&from) {
                        add(&mut map, key, &to);
                    }
                }
            }
        }
        // A key that only maps to itself changes nothing. A key whose alternatives all analyzed
        // to nothing, such as a right side made of stopwords, must keep the typed term instead of
        // matching nothing.
        map.retain(|key, alternatives| {
            !alternatives.is_empty() && alternatives.as_slice() != [key.clone()]
        });
        Self::from_map(map)
    }

    fn from_map(map: HashMap<Vec<String>, Vec<Vec<String>>>) -> Result<Self, SynonymError> {
        let max_key_tokens = map.keys().map(Vec::len).max().unwrap_or(0);
        let mut blob = vec![BLOB_FORMAT];
        put_varint(&mut blob, max_key_tokens as u64);

        // The FST builder takes its keys in byte order.
        let mut entries: Vec<(Vec<u8>, Vec<Vec<String>>)> = map
            .into_iter()
            .map(|(key, alternatives)| (encode_key(&key), alternatives))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let mut builder = MapBuilder::memory();
        // The keys of one equivalence rule share one list, so it goes into the blob once.
        let mut offsets: HashMap<Vec<Vec<String>>, u64> = HashMap::default();
        for (key, alternatives) in entries {
            let offset = match offsets.get(&alternatives) {
                Some(&offset) => offset,
                None => {
                    let offset = blob.len() as u64;
                    put_varint(&mut blob, alternatives.len() as u64);
                    for alternative in &alternatives {
                        put_varint(&mut blob, alternative.len() as u64);
                        for token in alternative {
                            put_varint(&mut blob, token.len() as u64);
                            blob.extend_from_slice(token.as_bytes());
                        }
                    }
                    offsets.insert(alternatives, offset);
                    offset
                }
            };
            builder.insert(key, offset)?;
        }
        Ok(Self {
            keys: Map::from_bytes(builder.into_inner()?)?,
            blob,
            max_key_tokens,
        })
    }

    #[allow(dead_code)]
    pub fn to_bytes(&self) -> (Vec<u8>, Vec<u8>) {
        (self.keys.as_fst().to_vec(), self.blob.clone())
    }

    #[allow(dead_code)]
    pub fn from_bytes(set: &str, fst: Vec<u8>, blob: Vec<u8>) -> Result<Self, SynonymError> {
        let stored = |reason: String| SynonymError::Stored {
            set: set.to_string(),
            reason,
        };
        let keys = Map::from_bytes(fst).map_err(|e| stored(e.to_string()))?;
        if blob.first() != Some(&BLOB_FORMAT) {
            return Err(stored("unknown blob format".to_string()));
        }
        let mut pos = 1;
        let max_key_tokens = get_varint(&blob, &mut pos)
            .ok_or_else(|| stored("truncated blob header".to_string()))?
            as usize;
        let set = Self {
            keys,
            blob,
            max_key_tokens,
        };
        // Every value is decoded once here, so a lookup never runs into a damaged blob.
        let mut values = set.keys.stream();
        while let Some((_, offset)) = values.next() {
            if set.alternatives_at(offset).is_none() {
                return Err(stored("damaged alternatives".to_string()));
            }
        }
        Ok(set)
    }

    fn alternatives_at(&self, offset: u64) -> Option<Vec<Vec<String>>> {
        let blob = &self.blob;
        let mut pos = usize::try_from(offset).ok()?;
        // A count never exceeds the bytes left, so a damaged count cannot ask for the moon.
        let left = |pos: usize| blob.len().saturating_sub(pos);
        let count = get_varint(blob, &mut pos)? as usize;
        let mut alternatives = Vec::with_capacity(count.min(left(pos)));
        for _ in 0..count {
            let tokens = get_varint(blob, &mut pos)? as usize;
            let mut alternative = Vec::with_capacity(tokens.min(left(pos)));
            for _ in 0..tokens {
                let len = get_varint(blob, &mut pos)? as usize;
                let end = pos.checked_add(len)?;
                let bytes = blob.get(pos..end)?;
                pos = end;
                alternative.push(String::from_utf8(bytes.to_vec()).ok()?);
            }
            alternatives.push(alternative);
        }
        Some(alternatives)
    }

    /// Longest key first, left to right, one pass. An alternative is never expanded again, the
    /// same as Lucene's `SynonymGraphFilter`, so a pasted Elasticsearch file behaves the same.
    pub fn expand(&self, tokens: &[String]) -> Vec<Expansion> {
        let mut expansions = Vec::with_capacity(tokens.len());
        let mut i = 0;
        while i < tokens.len() {
            let longest = (1..=self.max_key_tokens.min(tokens.len() - i))
                .rev()
                .find_map(|len| {
                    self.keys
                        .get(encode_key(&tokens[i..i + len]))
                        .map(|offset| (len, offset))
                });
            match longest {
                Some((len, offset)) => {
                    let alternatives = self
                        .alternatives_at(offset)
                        .expect("a stored synonym blob decodes");
                    expansions.push(Expansion {
                        tokens: tokens[i..i + len].to_vec(),
                        alternatives: Some(alternatives),
                    });
                    i += len;
                }
                None => {
                    expansions.push(Expansion {
                        tokens: vec![tokens[i].clone()],
                        alternatives: None,
                    });
                    i += 1;
                }
            }
        }
        expansions
    }
}

/// Identifies an analyzer configuration by its `SearchTokenizer` as JSON text. A field with no
/// tokenizer config has an empty key.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AnalyzerKey(String);

impl AnalyzerKey {
    pub fn new(tokenizer: Option<&SearchTokenizer>) -> Self {
        Self(tokenizer.map_or_else(String::new, |tokenizer| {
            serde_json::to_string(&tokenizer.to_stored_value())
                .expect("a tokenizer config serializes")
        }))
    }
}

/// Per backend: the compiled sets read so far, by set and analyzer.
type CompiledSets = HashMap<String, HashMap<AnalyzerKey, Arc<SynonymFst>>>;

static CACHE: OnceLock<Mutex<CompiledSets>> = OnceLock::new();

/// The relation id of `pdb.synonym_sets`, so the relcache callback can tell its invalidation
/// from any other table's. Set whenever the cache fills, since a catalog lookup is not safe
/// inside the callback.
static SETS_RELID: AtomicU32 = AtomicU32::new(0);

/// Moves on every invalidation of `pdb.synonym_sets`. A fill that started before one read the
/// set as it was, so it does not go into the cache.
static GENERATION: AtomicU64 = AtomicU64::new(0);

fn cache() -> &'static Mutex<CompiledSets> {
    CACHE.get_or_init(Default::default)
}

fn cached(name: &str, key: &AnalyzerKey) -> Option<Arc<SynonymFst>> {
    cache().lock().get(name)?.get(key).cloned()
}

/// A transaction with its own snapshot can read a version of the set older than the
/// invalidations it has processed, and nothing later would drop that copy.
fn uses_transaction_snapshot() -> bool {
    unsafe { pg_sys::XactIsoLevel >= pg_sys::XACT_REPEATABLE_READ as i32 }
}

fn remember(name: &str, key: AnalyzerKey, compiled: Arc<SynonymFst>, generation: u64) {
    if generation != GENERATION.load(Ordering::Relaxed) || uses_transaction_snapshot() {
        return;
    }
    SETS_RELID.store(relid(c"synonym_sets").to_u32(), Ordering::Relaxed);
    cache()
        .lock()
        .entry(name.to_string())
        .or_default()
        .insert(key, compiled);
}

fn relid(table: &CStr) -> pg_sys::Oid {
    unsafe {
        let namespace = pg_sys::get_namespace_oid(c"pdb".as_ptr(), false);
        pg_sys::get_relname_relid(table.as_ptr(), namespace)
    }
}

pub fn register_relcache_callback() {
    unsafe {
        pg_sys::CacheRegisterRelcacheCallback(Some(relcache_callback), pg_sys::Datum::from(0));
    }
}

/// Runs for every relcache invalidation in this backend, so it does nothing but compare ids.
#[pg_guard]
unsafe extern "C-unwind" fn relcache_callback(_arg: pg_sys::Datum, relid: pg_sys::Oid) {
    let ours = relid == pg_sys::InvalidOid || relid.to_u32() == SETS_RELID.load(Ordering::Relaxed);
    if ours {
        GENERATION.fetch_add(1, Ordering::Relaxed);
        if let Some(cache) = CACHE.get() {
            cache.lock().clear();
        }
    }
}

/// The set's version and rules, read together so they belong to one snapshot.
fn load_rules(name: &str) -> Result<(i64, Vec<String>), SynonymError> {
    let _snapshot = ActiveSnapshotGuard::ensure();
    Spi::connect(|client| {
        let rows = client.select(
            "SELECT s.version, r.rule::pg_catalog.text \
             FROM pdb.synonym_sets s \
             LEFT JOIN pdb.synonym_rules r ON r.set_name = s.name \
             WHERE s.name = $1",
            None,
            &[name.into()],
        )?;
        let rows = rows
            .map(|row| {
                let version = row
                    .get::<i64>(1)?
                    .expect("synonym_sets.version is NOT NULL");
                Ok::<_, pgrx::spi::Error>((version, row.get::<String>(2)?))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let Some(&(version, _)) = rows.first() else {
            return Err(SynonymError::NotFound(name.to_string()));
        };
        let rules = rows.into_iter().filter_map(|(_, rule)| rule).collect();
        Ok((version, rules))
    })
}

/// The set `name` compiled against `analyzer`, which must be the analyzer `tokenizer` builds,
/// because the copy is kept by `tokenizer`.
pub fn compiled_set(
    name: &str,
    tokenizer: Option<&SearchTokenizer>,
    analyzer: &mut TextAnalyzer,
) -> Result<Arc<SynonymFst>, SynonymError> {
    let key = AnalyzerKey::new(tokenizer);
    if let Some(hit) = cached(name, &key) {
        return Ok(hit);
    }
    let generation = GENERATION.load(Ordering::Relaxed);
    let (_, rules) = load_rules(name)?;
    let compiled = Arc::new(SynonymFst::compile(name, &rules, analyzer)?);
    remember(name, key, compiled.clone(), generation);
    Ok(compiled)
}

/// The sets a statement on `pdb.synonym_rules` touched, from its transition tables. A
/// `TRUNCATE` has none and touches every set.
pub fn changed_sets(trigger: &PgTrigger) -> Result<Vec<String>, SynonymError> {
    if matches!(trigger.op(), Ok(PgTriggerOperation::Truncate)) {
        return Spi::connect(|client| {
            Ok(client
                .select("SELECT name FROM pdb.synonym_sets", None, &[])?
                .map(|row| row.get::<String>(1))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect())
        });
    }
    let tables: Vec<String> = [
        trigger.old_transition_table_name(),
        trigger.new_transition_table_name(),
    ]
    .into_iter()
    .filter_map(|table| table.ok().flatten())
    .map(pgrx::spi::quote_identifier)
    .collect();
    let trigger_data = trigger.trigger_data() as *const pg_sys::TriggerData as *mut _;
    Spi::connect(|client| {
        // A transition table is only visible to SPI once registered, after the connect.
        let registered = unsafe { pg_sys::SPI_register_trigger_data(trigger_data) };
        if registered != pg_sys::SPI_OK_TD_REGISTER as i32 {
            panic!("could not register the trigger's transition tables with SPI");
        }
        let mut sets = Vec::new();
        for table in tables {
            let sql = format!("SELECT DISTINCT set_name FROM {table}");
            let rows = client.select(sql.as_str(), None, &[])?;
            for row in rows {
                sets.push(
                    row.get::<String>(1)?
                        .expect("synonym_rules.set_name is NOT NULL"),
                );
            }
        }
        sets.sort();
        sets.dedup();
        Ok(sets)
    })
}

/// Runs after a statement changed the rules of `name`. It bumps the set's version and tells
/// every backend to drop its copy.
pub fn rules_changed(name: &str) -> Result<(), SynonymError> {
    Spi::connect_mut(|client| {
        client.update(
            "UPDATE pdb.synonym_sets \
             SET version = version + 1, updated_at = pg_catalog.now() \
             WHERE name = $1",
            None,
            &[name.into()],
        )?;
        Ok::<_, SynonymError>(())
    })?;
    // The invalidation reaches this backend at the next command. The writer's own later
    // statements in this transaction must see the new rules before that.
    cache().lock().remove(name);
    unsafe { pg_sys::CacheInvalidateRelcacheByRelid(relid(c"synonym_sets")) };
    Ok(())
}

/// Runs after rows of `pdb.synonym_sets` were deleted. A set deleted while it has no rules fires
/// no rule trigger, and its copies would stay in memory.
pub fn sets_removed() {
    cache().lock().clear();
    unsafe { pg_sys::CacheInvalidateRelcacheByRelid(relid(c"synonym_sets")) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::tokenizer::{LowerCaser, SimpleTokenizer, Stemmer, StopWordFilter};

    fn analyzer() -> TextAnalyzer {
        TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .build()
    }

    fn stemming_analyzer() -> TextAnalyzer {
        TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .filter(Stemmer::default())
            .build()
    }

    fn stopword_analyzer() -> TextAnalyzer {
        TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .filter(StopWordFilter::remove(vec!["the".to_string()]))
            .build()
    }

    fn rules(rules: &[&str]) -> Vec<String> {
        rules.iter().map(|r| r.to_string()).collect()
    }

    fn tokens(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn parses_solr_rules() {
        assert_eq!(
            parse_rule("couch, sofa , divan").unwrap(),
            Some(Rule::Equivalent(tokens(&["couch", "sofa", "divan"])))
        );
        assert_eq!(
            parse_rule("sneakers => running shoes, trainers").unwrap(),
            Some(Rule::Directed {
                from: tokens(&["sneakers"]),
                to: tokens(&["running shoes", "trainers"]),
            })
        );
        assert_eq!(parse_rule("  # a comment").unwrap(), None);
        assert_eq!(parse_rule("").unwrap(), None);
        assert_eq!(
            parse_rule(r"a\, b, c").unwrap(),
            Some(Rule::Equivalent(tokens(&["a, b", "c"])))
        );
        assert!(parse_rule("a => b => c").is_err());
        assert!(parse_rule("=> c").is_err());
        let long = (0..=MAX_RULE_TERMS)
            .map(|i| format!("t{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        assert!(parse_rule(&long).is_err());
    }

    #[test]
    fn equivalents_expand_to_the_group() {
        let compiled =
            SynonymFst::compile("t", &rules(&["couch, sofa, divan"]), &mut analyzer()).unwrap();
        let expansions = compiled.expand(&tokens(&["red", "sofa"]));
        assert_eq!(
            expansions,
            vec![
                Expansion {
                    tokens: tokens(&["red"]),
                    alternatives: None,
                },
                Expansion {
                    tokens: tokens(&["sofa"]),
                    alternatives: Some(vec![
                        tokens(&["couch"]),
                        tokens(&["sofa"]),
                        tokens(&["divan"])
                    ]),
                },
            ]
        );
    }

    #[test]
    fn directed_rules_replace_the_left_side() {
        let compiled =
            SynonymFst::compile("t", &rules(&["sneakers => running shoes"]), &mut analyzer())
                .unwrap();
        assert_eq!(
            compiled.expand(&tokens(&["sneakers"])),
            vec![Expansion {
                tokens: tokens(&["sneakers"]),
                alternatives: Some(vec![tokens(&["running", "shoes"])]),
            }]
        );
        assert_eq!(
            compiled.expand(&tokens(&["running", "shoes"])),
            vec![
                Expansion {
                    tokens: tokens(&["running"]),
                    alternatives: None,
                },
                Expansion {
                    tokens: tokens(&["shoes"]),
                    alternatives: None,
                },
            ]
        );
    }

    #[test]
    fn longest_key_wins_and_outputs_are_not_expanded_again() {
        let compiled = SynonymFst::compile(
            "t",
            &rules(&["running shoes, sneakers", "shoes, footwear"]),
            &mut analyzer(),
        )
        .unwrap();
        assert_eq!(
            compiled.expand(&tokens(&["running", "shoes"])),
            vec![Expansion {
                tokens: tokens(&["running", "shoes"]),
                alternatives: Some(vec![tokens(&["running", "shoes"]), tokens(&["sneakers"])]),
            }]
        );
        assert_eq!(
            compiled.expand(&tokens(&["blue", "shoes"])),
            vec![
                Expansion {
                    tokens: tokens(&["blue"]),
                    alternatives: None,
                },
                Expansion {
                    tokens: tokens(&["shoes"]),
                    alternatives: Some(vec![tokens(&["shoes"]), tokens(&["footwear"])]),
                },
            ]
        );
    }

    #[test]
    fn rules_go_through_the_analyzer() {
        let compiled = SynonymFst::compile(
            "t",
            &rules(&["Running Shoes, Sneakers"]),
            &mut stemming_analyzer(),
        )
        .unwrap();
        assert_eq!(
            compiled.expand(&tokens(&["run", "shoe"])),
            vec![Expansion {
                tokens: tokens(&["run", "shoe"]),
                alternatives: Some(vec![tokens(&["run", "shoe"]), tokens(&["sneaker"])]),
            }]
        );
    }

    #[test]
    fn same_key_in_two_rules_merges() {
        let compiled = SynonymFst::compile(
            "t",
            &rules(&["sofa, couch", "sofa => settee"]),
            &mut analyzer(),
        )
        .unwrap();
        assert_eq!(
            compiled.expand(&tokens(&["sofa"])),
            vec![Expansion {
                tokens: tokens(&["sofa"]),
                alternatives: Some(vec![
                    tokens(&["sofa"]),
                    tokens(&["couch"]),
                    tokens(&["settee"])
                ]),
            }]
        );
    }

    #[test]
    fn a_rule_of_one_term_is_dropped() {
        let compiled =
            SynonymFst::compile("t", &rules(&["sofa", "# note", ""]), &mut analyzer()).unwrap();
        assert_eq!(
            compiled.expand(&tokens(&["sofa"])),
            vec![Expansion {
                tokens: tokens(&["sofa"]),
                alternatives: None,
            }]
        );
    }

    #[test]
    fn a_right_side_of_stopwords_keeps_the_typed_term() {
        let compiled = SynonymFst::compile(
            "t",
            &rules(&["sofa => the", "couch => the settee"]),
            &mut stopword_analyzer(),
        )
        .unwrap();
        assert_eq!(
            compiled.expand(&tokens(&["sofa", "couch"])),
            vec![
                Expansion {
                    tokens: tokens(&["sofa"]),
                    alternatives: None,
                },
                Expansion {
                    tokens: tokens(&["couch"]),
                    alternatives: Some(vec![tokens(&["settee"])]),
                },
            ]
        );
    }

    #[test]
    fn the_cap_keeps_the_typed_tokens() {
        let compiled = SynonymFst::compile(
            "t",
            &rules(&["couch, sofa, divan, settee"]),
            &mut analyzer(),
        )
        .unwrap();
        let mut expansions = compiled.expand(&tokens(&["divan"]));
        expansions.iter_mut().for_each(|e| e.cap(2));
        assert_eq!(
            expansions,
            vec![Expansion {
                tokens: tokens(&["divan"]),
                alternatives: Some(vec![tokens(&["couch"]), tokens(&["divan"])]),
            }]
        );
        let mut expansions = compiled.expand(&tokens(&["couch", "red"]));
        expansions.iter_mut().for_each(|e| e.cap(2));
        assert_eq!(
            expansions,
            vec![
                Expansion {
                    tokens: tokens(&["couch"]),
                    alternatives: Some(vec![tokens(&["couch"]), tokens(&["sofa"])]),
                },
                Expansion {
                    tokens: tokens(&["red"]),
                    alternatives: None,
                },
            ]
        );
    }

    #[test]
    fn the_stored_bytes_expand_the_same() {
        let compiled = SynonymFst::compile(
            "t",
            &rules(&[
                "couch, sofa, divan",
                "sneakers => running shoes",
                "a\\, b => c",
            ]),
            &mut analyzer(),
        )
        .unwrap();
        let (fst, blob) = compiled.to_bytes();
        let restored = SynonymFst::from_bytes("t", fst, blob).unwrap();
        for query in [
            tokens(&["red", "sofa"]),
            tokens(&["sneakers"]),
            tokens(&["a", "b"]),
            tokens(&["running", "shoes"]),
            tokens(&[]),
        ] {
            assert_eq!(restored.expand(&query), compiled.expand(&query));
        }
        assert!(SynonymFst::from_bytes("t", vec![1, 2, 3], vec![BLOB_FORMAT, 0]).is_err());
        assert!(SynonymFst::from_bytes("t", compiled.to_bytes().0, vec![9]).is_err());
    }

    #[test]
    fn the_keys_of_one_rule_share_one_blob_entry() {
        let compiled =
            SynonymFst::compile("t", &rules(&["couch, sofa, divan"]), &mut analyzer()).unwrap();
        assert_eq!(compiled.keys.get(b"couch"), compiled.keys.get(b"sofa"));
        assert_eq!(compiled.keys.get(b"sofa"), compiled.keys.get(b"divan"));
    }

    #[test]
    fn a_damaged_blob_does_not_load() {
        let compiled = SynonymFst::compile("t", &rules(&["couch, sofa"]), &mut analyzer()).unwrap();
        let (fst, mut blob) = compiled.to_bytes();
        // A huge count where the first entry's alternative count was.
        let entry = compiled.keys.get(b"couch").unwrap() as usize;
        blob[entry] = 0xff;
        blob.truncate(entry + 1);
        assert!(SynonymFst::from_bytes("t", fst, blob).is_err());
    }
}
