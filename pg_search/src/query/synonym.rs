// Copyright (c) 2023-2025 ParadeDB, Inc.
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
//! This module provides Elasticsearch-style synonym expansion at query time.
//! Synonyms are loaded from a PostgreSQL table and used to expand query terms
//! into Boolean queries with term and phrase alternatives.
//!
//! ## Table Schema
//!
//! ```sql
//! CREATE TABLE synonyms (
//!     term TEXT PRIMARY KEY,      -- term to match (can be multi-word)
//!     expansions TEXT[] NOT NULL  -- what to expand to (can include multi-word)
//! );
//!
//! -- Equivalent synonyms: each term maps to all
//! INSERT INTO synonyms VALUES ('ny', ARRAY['ny', 'new york', 'nyc']);
//! INSERT INTO synonyms VALUES ('new york', ARRAY['ny', 'new york', 'nyc']);
//! INSERT INTO synonyms VALUES ('nyc', ARRAY['ny', 'new york', 'nyc']);
//! ```

use pgrx::Spi;
use std::collections::HashMap;
use tantivy::query::{BooleanQuery, Occur, PhraseQuery, Query, TermQuery};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::tokenizer::TextAnalyzer;
use tantivy::Term;

/// Tokenize text using the provided analyzer.
fn tokenize_text(text: &str, tokenizer: &mut TextAnalyzer) -> Vec<String> {
    let mut stream = tokenizer.token_stream(text);
    let mut tokens = Vec::new();
    while stream.advance() {
        tokens.push(stream.token().text.clone());
    }
    tokens
}

/// A trie node for efficient multi-word phrase prefix matching.
#[derive(Clone, Default, Debug)]
struct TrieNode {
    children: HashMap<String, TrieNode>,
    /// If Some, this node represents a complete term with expansions.
    expansions: Option<Vec<String>>,
}

/// Synonym map supporting both single-word and multi-word terms.
#[derive(Clone, Default, Debug)]
pub struct SynonymMap {
    /// Maps single-word terms to their expansions.
    single_word: HashMap<String, Vec<String>>,
    /// Trie for multi-word term matching.
    multi_word_trie: TrieNode,
    /// Whether there are any multi-word terms.
    has_multi_word: bool,
}

impl SynonymMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a synonym rule using the provided tokenizer.
    /// The tokenizer handles normalization (lowercasing, etc.) consistently.
    pub fn insert(&mut self, term: &str, expansions: Vec<String>, tokenizer: &mut TextAnalyzer) {
        let words = tokenize_text(term, tokenizer);
        if words.len() == 1 {
            self.single_word.insert(words[0].clone(), expansions);
        } else if !words.is_empty() {
            self.has_multi_word = true;
            let mut node = &mut self.multi_word_trie;
            for word in &words {
                node = node
                    .children
                    .entry(word.clone())
                    .or_insert_with(TrieNode::default);
            }
            node.expansions = Some(expansions);
        }
    }

    /// Look up a single-word term.
    pub fn get_single(&self, term: &str) -> Option<&Vec<String>> {
        self.single_word.get(term)
    }

    /// Check if a sequence of tokens matches a multi-word term.
    /// Returns (match_length, expansions) if found.
    pub fn get_multi(&self, tokens: &[&str]) -> Option<(usize, &Vec<String>)> {
        if !self.has_multi_word || tokens.is_empty() {
            return None;
        }

        let mut node = &self.multi_word_trie;
        let mut last_match: Option<(usize, &Vec<String>)> = None;

        for (i, token) in tokens.iter().enumerate() {
            match node.children.get(*token) {
                Some(child) => {
                    node = child;
                    if let Some(ref expansions) = node.expansions {
                        last_match = Some((i + 1, expansions));
                    }
                }
                None => break,
            }
        }

        last_match
    }

    /// Load synonyms from a PostgreSQL table, filtered by query tokens.
    /// Table must have columns: term TEXT, expansions TEXT[]
    /// Uses the provided tokenizer to consistently tokenize both synonym terms and expansions.
    /// Only loads synonyms where the term starts with one of the query tokens (for efficiency).
    pub fn load_from_table(
        table_name: &str,
        query_tokens: &[String],
        tokenizer: &mut TextAnalyzer,
    ) -> Result<Self, String> {
        // Validate table name
        if !table_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        {
            return Err(format!(
                "Invalid table name: '{}'. Only alphanumeric, underscore, and dot allowed.",
                table_name
            ));
        }

        if query_tokens.is_empty() {
            return Ok(SynonymMap::new());
        }

        // Build a query that filters by tokens that could match
        // We use LIKE 'token%' to match both single-word and multi-word terms
        let like_conditions: Vec<String> = query_tokens
            .iter()
            .map(|t| format!("lower(term) LIKE '{}%'", t.to_lowercase().replace('\'', "''")))
            .collect();
        let where_clause = like_conditions.join(" OR ");
        let query = format!(
            "SELECT term, expansions FROM {} WHERE {}",
            table_name, where_clause
        );

        let mut map = SynonymMap::new();

        Spi::connect(|client| {
            let result = client.select(&query, None, &[]);
            match result {
                Ok(table) => {
                    for row in table {
                        let term: Option<String> = row.get(1).ok().flatten();
                        let expansions: Option<Vec<String>> = row.get(2).ok().flatten();

                        if let (Some(term), Some(expansions)) = (term, expansions) {
                            map.insert(&term, expansions, tokenizer);
                        }
                    }
                }
                Err(e) => {
                    return Err(format!("Failed to load synonyms from '{}': {}", table_name, e));
                }
            }
            Ok(())
        })?;

        Ok(map)
    }
}

/// Represents an expanded query term.
#[derive(Debug, Clone)]
pub enum ExpandedTerm {
    /// A single term (no expansion or single-word expansion).
    Term(String),
    /// Multiple alternatives (OR of terms and/or phrases).
    Alternatives(Vec<Expansion>),
}

/// A single expansion - either a term or a phrase.
#[derive(Debug, Clone)]
pub enum Expansion {
    Term(String),
    Phrase(Vec<String>),
}

impl Expansion {
    /// Convert to a Tantivy query.
    pub fn to_query(&self, field: Field) -> Box<dyn Query> {
        match self {
            Expansion::Term(text) => term_query(field, text),
            Expansion::Phrase(words) => {
                let terms: Vec<Term> = words
                    .iter()
                    .map(|w| Term::from_field_text(field, w))
                    .collect();
                Box::new(PhraseQuery::new(terms))
            }
        }
    }
}

/// Create a TermQuery for the given field and text.
fn term_query(field: Field, text: &str) -> Box<dyn Query> {
    let term = Term::from_field_text(field, text);
    Box::new(TermQuery::new(term, IndexRecordOption::WithFreqsAndPositions))
}

/// Parse expansion strings into Expansion variants using the tokenizer.
fn parse_expansions(expansions: &[String], tokenizer: &mut TextAnalyzer) -> Vec<Expansion> {
    expansions
        .iter()
        .map(|exp| {
            let words = tokenize_text(exp, tokenizer);
            if words.len() == 1 {
                Expansion::Term(words[0].clone())
            } else {
                Expansion::Phrase(words)
            }
        })
        .collect()
}

/// Expand a sequence of tokens using the synonym map.
/// Returns expanded terms, consuming multi-word matches greedily.
/// Uses the provided tokenizer to parse expansion strings consistently.
pub fn expand_tokens(tokens: &[String], synonym_map: &SynonymMap, tokenizer: &mut TextAnalyzer) -> Vec<ExpandedTerm> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        // First, try multi-word match (greedy)
        let remaining: Vec<&str> = tokens[i..].iter().map(|s| s.as_str()).collect();
        if let Some((match_len, expansions)) = synonym_map.get_multi(&remaining) {
            result.push(ExpandedTerm::Alternatives(parse_expansions(expansions, tokenizer)));
            i += match_len;
            continue;
        }

        // Try single-word match
        let token = &tokens[i];
        if let Some(expansions) = synonym_map.get_single(token) {
            result.push(ExpandedTerm::Alternatives(parse_expansions(expansions, tokenizer)));
        } else {
            // No synonym, keep original
            result.push(ExpandedTerm::Term(token.clone()));
        }

        i += 1;
    }

    result
}

/// Convert an ExpandedTerm to a Tantivy query.
fn expanded_term_to_query(exp: &ExpandedTerm, field: Field) -> Box<dyn Query> {
    match exp {
        ExpandedTerm::Term(text) => term_query(field, text),
        ExpandedTerm::Alternatives(alts) if alts.len() == 1 => alts[0].to_query(field),
        ExpandedTerm::Alternatives(alts) => {
            let subqueries: Vec<(Occur, Box<dyn Query>)> = alts
                .iter()
                .map(|alt| (Occur::Should, alt.to_query(field)))
                .collect();
            Box::new(BooleanQuery::new(subqueries))
        }
    }
}

/// Build a Tantivy query from expanded terms.
/// Uses AND semantics between terms, OR for alternatives.
pub fn build_query(expanded: &[ExpandedTerm], field: Field) -> Box<dyn Query> {
    match expanded.len() {
        0 => Box::new(BooleanQuery::new(vec![])),
        1 => expanded_term_to_query(&expanded[0], field),
        _ => {
            let subqueries: Vec<(Occur, Box<dyn Query>)> = expanded
                .iter()
                .map(|exp| (Occur::Must, expanded_term_to_query(exp, field)))
                .collect();
            Box::new(BooleanQuery::new(subqueries))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::tokenizer::{SimpleTokenizer, TokenizerManager};

    fn simple_tokenizer() -> TextAnalyzer {
        TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(tantivy::tokenizer::LowerCaser)
            .build()
    }

    #[test]
    fn test_single_word_synonyms() {
        let mut map = SynonymMap::new();
        let mut tokenizer = simple_tokenizer();
        map.insert("cat", vec!["cat".to_string(), "feline".to_string(), "kitty".to_string()], &mut tokenizer);

        let expansions = map.get_single("cat").unwrap();
        assert_eq!(expansions.len(), 3);
        assert!(expansions.contains(&"cat".to_string()));
        assert!(expansions.contains(&"feline".to_string()));
        assert!(expansions.contains(&"kitty".to_string()));
    }

    #[test]
    fn test_multi_word_synonyms() {
        let mut map = SynonymMap::new();
        let mut tokenizer = simple_tokenizer();
        map.insert("new york", vec!["new york".to_string(), "ny".to_string(), "nyc".to_string()], &mut tokenizer);

        let tokens = vec!["new", "york", "city"];
        let result = map.get_multi(&tokens);
        assert!(result.is_some());
        let (len, expansions) = result.unwrap();
        assert_eq!(len, 2);
        assert_eq!(expansions.len(), 3);
    }

    #[test]
    fn test_expand_tokens_single() {
        let mut map = SynonymMap::new();
        let mut tokenizer = simple_tokenizer();
        map.insert("cat", vec!["cat".to_string(), "feline".to_string()], &mut tokenizer);

        let tokens = vec!["the".to_string(), "cat".to_string(), "runs".to_string()];
        let expanded = expand_tokens(&tokens, &map, &mut tokenizer);

        assert_eq!(expanded.len(), 3);
        assert!(matches!(&expanded[0], ExpandedTerm::Term(t) if t == "the"));
        assert!(matches!(&expanded[1], ExpandedTerm::Alternatives(alts) if alts.len() == 2));
        assert!(matches!(&expanded[2], ExpandedTerm::Term(t) if t == "runs"));
    }

    #[test]
    fn test_expand_tokens_multi_word() {
        let mut map = SynonymMap::new();
        let mut tokenizer = simple_tokenizer();
        map.insert("new york", vec!["new york".to_string(), "ny".to_string()], &mut tokenizer);

        let tokens = vec!["visit".to_string(), "new".to_string(), "york".to_string(), "today".to_string()];
        let expanded = expand_tokens(&tokens, &map, &mut tokenizer);

        assert_eq!(expanded.len(), 3); // "visit", "new york" (combined), "today"
        assert!(matches!(&expanded[0], ExpandedTerm::Term(t) if t == "visit"));
        assert!(matches!(&expanded[1], ExpandedTerm::Alternatives(_)));
        assert!(matches!(&expanded[2], ExpandedTerm::Term(t) if t == "today"));
    }

    #[test]
    fn test_greedy_multi_word_match() {
        let mut map = SynonymMap::new();
        let mut tokenizer = simple_tokenizer();
        map.insert("new york", vec!["ny".to_string()], &mut tokenizer);
        map.insert("new york city", vec!["nyc".to_string()], &mut tokenizer);

        let tokens = vec!["new".to_string(), "york".to_string(), "city".to_string()];
        let expanded = expand_tokens(&tokens, &map, &mut tokenizer);

        // Should match "new york city" (longest), not "new york"
        assert_eq!(expanded.len(), 1);
        if let ExpandedTerm::Alternatives(alts) = &expanded[0] {
            assert_eq!(alts.len(), 1);
            assert!(matches!(&alts[0], Expansion::Term(t) if t == "nyc"));
        } else {
            panic!("Expected Alternatives");
        }
    }
}
