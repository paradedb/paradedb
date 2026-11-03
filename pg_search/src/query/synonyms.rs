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
//! The rules of a set live in `paradedb.synonym_sets`, one Solr-format rule per array element.
//! A set is compiled against the analyzer of the query, so stemming and lowercasing apply to the
//! rules and to the query alike, and the compiled form is cached per backend. The cache is checked
//! against the row on every use, so an `UPDATE` to a set reaches the next query without a reload.

use crate::api::HashMap;
use crate::api::tokenizers::typmod::StmtHolder;
use crate::postgres::utils::ActiveSnapshotGuard;
use parking_lot::Mutex;
use pgrx::datum::DatumWithOid;
use pgrx::pg_sys::BuiltinOid;
use pgrx::spi::Query;
use pgrx::{PgOid, Spi, pg_sys};
use std::sync::{Arc, OnceLock};
use tantivy::tokenizer::TextAnalyzer;
use thiserror::Error;

/// The `synonyms` option value that turns expansion off for a field or an index.
pub const SYNONYMS_NONE: &str = "none";

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
    #[error(transparent)]
    Spi(#[from] pgrx::spi::Error),
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
fn parse_rule(rule: &str) -> Result<Option<Rule>, &'static str> {
    let rule = rule.trim();
    if rule.is_empty() || rule.starts_with('#') {
        return Ok(None);
    }
    let mut sides = split_rule(rule);
    match sides.len() {
        1 => Ok(Some(Rule::Equivalent(sides.pop().unwrap()))),
        2 => {
            let to = sides.pop().unwrap();
            let from = sides.pop().unwrap();
            if from.is_empty() || to.is_empty() {
                return Err("both sides of '=>' need at least one term");
            }
            Ok(Some(Rule::Directed { from, to }))
        }
        _ => Err("a rule can have at most one '=>'"),
    }
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
    pub fn is_single_term(&self) -> bool {
        match &self.alternatives {
            None => true,
            Some(alternatives) => alternatives.len() == 1 && alternatives[0].len() == 1,
        }
    }

    /// Keeps at most `max` alternatives, in rule order. The typed tokens survive the cap, so a
    /// long rule never drops what the user typed.
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

/// A set compiled against one analyzer: every key and every alternative is a token sequence that
/// analyzer produced.
#[derive(Debug, Default)]
pub struct CompiledSynonyms {
    map: HashMap<Vec<String>, Vec<Vec<String>>>,
    max_key_len: usize,
}

impl CompiledSynonyms {
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
                reason: reason.to_string(),
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
        let max_key_len = map.keys().map(Vec::len).max().unwrap_or(0);
        Ok(Self { map, max_key_len })
    }

    /// Longest key first, left to right, one pass. An alternative is never expanded again, the
    /// same as Lucene's `SynonymGraphFilter`, so a pasted Elasticsearch file behaves the same.
    pub fn expand(&self, tokens: &[String]) -> Vec<Expansion> {
        let mut expansions = Vec::with_capacity(tokens.len());
        let mut i = 0;
        while i < tokens.len() {
            let longest = (1..=self.max_key_len.min(tokens.len() - i))
                .rev()
                .find_map(|len| {
                    self.map
                        .get(&tokens[i..i + len])
                        .map(|alternatives| (len, alternatives))
                });
            match longest {
                Some((len, alternatives)) => {
                    expansions.push(Expansion {
                        tokens: tokens[i..i + len].to_vec(),
                        alternatives: Some(alternatives.clone()),
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

/// What a backend remembers about one set. `version` alone cannot tell a live copy from a stale
/// one, because a rolled-back `UPDATE` hands its version number to the next committed one, so the
/// row's `xmin` is part of the stamp.
struct CachedSet {
    version: i64,
    xmin: i64,
    /// The statement that last read the row. One statement sees one version of the set, and a
    /// query builds its Tantivy query more than once (estimation, then execution), so the row is
    /// read once per statement.
    checked_in: StatementId,
    rules: Arc<Vec<String>>,
    compiled: HashMap<String, Arc<CompiledSynonyms>>,
}

/// Names the current statement without assigning a transaction id: the transaction start time
/// tells transactions apart, the command id tells statements inside one apart.
type StatementId = (pg_sys::TimestampTz, pg_sys::CommandId);

fn current_statement() -> StatementId {
    unsafe {
        (
            pg_sys::GetCurrentTransactionStartTimestamp(),
            pg_sys::GetCurrentCommandId(false),
        )
    }
}

static CACHE: OnceLock<Mutex<HashMap<String, CachedSet>>> = OnceLock::new();

enum Loaded {
    Unchanged,
    Fresh {
        version: i64,
        xmin: i64,
        rules: Arc<Vec<String>>,
    },
}

/// Reads the set's row under the query's snapshot. Only the stamp is decoded when it matches
/// `cached`, so an unchanged set costs one primary-key lookup and no detoasting.
fn load_row(name: &str, cached: Option<(i64, i64)>) -> Result<Loaded, SynonymError> {
    // Planning and execution have an active snapshot. Callers such as the logical-replication
    // apply worker may not, and SPI needs one.
    let _snapshot = ActiveSnapshotGuard::ensure();
    Spi::connect(|client| {
        static STMT: OnceLock<StmtHolder> = OnceLock::new();

        let prepared = STMT.get_or_init(|| {
            StmtHolder(
                client
                    .prepare(
                        "SELECT version, xmin::text::int8, rules \
                         FROM paradedb.synonym_sets WHERE name = $1",
                        &[PgOid::BuiltIn(BuiltinOid::TEXTOID)],
                    )
                    .expect("failed to prepare statement")
                    .keep(),
            )
        });

        let args = unsafe { [DatumWithOid::new(name, pg_sys::TEXTOID)] };
        let rows = (&prepared.0).execute(client, None, &args)?;
        if rows.is_empty() {
            return Err(SynonymError::NotFound(name.to_string()));
        }
        let row = rows.first();
        let version = row
            .get::<i64>(1)?
            .expect("synonym_sets.version is NOT NULL");
        let xmin = row.get::<i64>(2)?.expect("xmin is never NULL");
        if cached == Some((version, xmin)) {
            return Ok(Loaded::Unchanged);
        }
        let rules = row
            .get::<Vec<Option<String>>>(3)?
            .expect("synonym_sets.rules is NOT NULL")
            .into_iter()
            .flatten()
            .collect();
        Ok(Loaded::Fresh {
            version,
            xmin,
            rules: Arc::new(rules),
        })
    })
}

/// The set `name` compiled against `analyzer`. `analyzer_key` must change whenever the analyzer's
/// output would, because the compiled form is cached under it.
pub fn compiled_set(
    name: &str,
    analyzer_key: &str,
    analyzer: &mut TextAnalyzer,
) -> Result<Arc<CompiledSynonyms>, SynonymError> {
    let cache = CACHE.get_or_init(Default::default);

    // The lock is not held across SPI, or across compilation, so a nested use of the cache
    // from inside either cannot wait on itself.
    let statement = current_statement();
    let cached = cache
        .lock()
        .get(name)
        .map(|set| (set.version, set.xmin, set.checked_in == statement));
    let loaded = match cached {
        Some((_, _, true)) => Loaded::Unchanged,
        _ => match load_row(name, cached.map(|(version, xmin, _)| (version, xmin))) {
            Err(SynonymError::NotFound(name)) => {
                cache.lock().remove(&name);
                return Err(SynonymError::NotFound(name));
            }
            loaded => loaded?,
        },
    };

    let rules = {
        let mut cache = cache.lock();
        let set = match loaded {
            Loaded::Unchanged => cache
                .get_mut(name)
                .expect("a set with a matching stamp is in the cache"),
            Loaded::Fresh {
                version,
                xmin,
                rules,
            } => {
                cache.insert(
                    name.to_string(),
                    CachedSet {
                        version,
                        xmin,
                        checked_in: statement,
                        rules,
                        compiled: HashMap::default(),
                    },
                );
                cache.get_mut(name).unwrap()
            }
        };
        set.checked_in = statement;
        if let Some(compiled) = set.compiled.get(analyzer_key) {
            return Ok(compiled.clone());
        }
        set.rules.clone()
    };

    let compiled = Arc::new(CompiledSynonyms::compile(name, &rules, analyzer)?);
    if let Some(set) = cache.lock().get_mut(name) {
        set.compiled
            .insert(analyzer_key.to_string(), compiled.clone());
    }
    Ok(compiled)
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
    }

    #[test]
    fn equivalents_expand_to_the_group() {
        let compiled =
            CompiledSynonyms::compile("t", &rules(&["couch, sofa, divan"]), &mut analyzer())
                .unwrap();
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
            CompiledSynonyms::compile("t", &rules(&["sneakers => running shoes"]), &mut analyzer())
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
        let compiled = CompiledSynonyms::compile(
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
        let compiled = CompiledSynonyms::compile(
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
        let compiled = CompiledSynonyms::compile(
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
            CompiledSynonyms::compile("t", &rules(&["sofa", "# note", ""]), &mut analyzer())
                .unwrap();
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
        let compiled = CompiledSynonyms::compile(
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
        let compiled = CompiledSynonyms::compile(
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
    fn invalid_rule_names_the_set() {
        let err = CompiledSynonyms::compile("shop", &rules(&["a => b => c"]), &mut analyzer())
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "synonym set 'shop' has an invalid rule 'a => b => c': a rule can have at most one '=>'"
        );
    }
}
