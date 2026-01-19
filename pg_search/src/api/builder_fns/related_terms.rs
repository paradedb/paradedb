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

use crate::api::HashSet;
use crate::index::mvcc::MvccSatisfies;
use crate::postgres::utils::locate_bm25_index;
use pgrx::iter::TableIterator;
use pgrx::{default, name, pg_extern, PgRelation};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use tantivy::collector::DocSetCollector;
use tantivy::index::Index;
use tantivy::query::{BooleanQuery, Occur, Query, TermQuery};
use tantivy::schema::{Field, FieldType, IndexRecordOption, OwnedValue, Schema, Term};
use tantivy::tokenizer::{TokenStream, TokenizerManager};
use tantivy::{DateTime, DocAddress, ReloadPolicy, Searcher, TantivyDocument};

/// A scored term for sorting, including field information for MLT-compatible per-field DF.
#[derive(Debug, PartialEq)]
struct ScoreTerm {
    field_name: String,
    term_text: String,
    score: f32,
}

impl Eq for ScoreTerm {}

impl PartialOrd for ScoreTerm {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoreTerm {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.score.total_cmp(&other.score) {
            std::cmp::Ordering::Equal => {
                (&self.field_name, &self.term_text).cmp(&(&other.field_name, &other.term_text))
            }
            ord => ord,
        }
    }
}

/// Compute IDF using the same formula as Tantivy's MLT:
/// idf = ln(1 + (N - df + 0.5) / (df + 0.5))
#[inline]
fn idf(doc_freq: u64, num_docs: u64) -> f32 {
    let x = ((num_docs - doc_freq) as f32 + 0.5) / (doc_freq as f32 + 0.5);
    (1.0 + x).ln()
}

/// Build term queries for a given query_term across target fields.
/// Returns the subqueries and the set of tokens to exclude from results.
fn build_term_queries(
    query_term: &str,
    target_fields: &[Field],
    tantivy_schema: &Schema,
    tokenizer_manager: &TokenizerManager,
) -> (Vec<(Occur, Box<dyn Query>)>, HashSet<String>) {
    let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
    let mut excluded_tokens: HashSet<String> = HashSet::default();

    // Also exclude the original query term itself (as-is)
    excluded_tokens.insert(query_term.to_string());

    for &field in target_fields {
        let field_entry = tantivy_schema.get_field_entry(field);
        match field_entry.field_type() {
            FieldType::Str(text_options) => {
                if let Some(indexing_options) = text_options.get_indexing_options() {
                    if let Some(mut tokenizer) = tokenizer_manager.get(indexing_options.tokenizer())
                    {
                        let mut stream = tokenizer.token_stream(query_term);
                        while stream.advance() {
                            let token_text = stream.token().text.clone();
                            excluded_tokens.insert(token_text.clone());
                            let term = Term::from_field_text(field, &token_text);
                            subqueries.push((
                                Occur::Should,
                                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                            ));
                        }
                    }
                }
            }
            FieldType::I64(_) => {
                if let Ok(val) = query_term.parse::<i64>() {
                    excluded_tokens.insert(val.to_string());
                    let term = Term::from_field_i64(field, val);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                    ));
                }
            }
            FieldType::U64(_) => {
                if let Ok(val) = query_term.parse::<u64>() {
                    excluded_tokens.insert(val.to_string());
                    let term = Term::from_field_u64(field, val);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                    ));
                }
            }
            FieldType::F64(_) => {
                if let Ok(val) = query_term.parse::<f64>() {
                    excluded_tokens.insert(val.to_string());
                    let term = Term::from_field_f64(field, val);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                    ));
                }
            }
            FieldType::Bool(_) => {
                if let Ok(val) = query_term.parse::<bool>() {
                    excluded_tokens.insert(val.to_string());
                    let term = Term::from_field_bool(field, val);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                    ));
                }
            }
            _ => {}
        }
    }

    (subqueries, excluded_tokens)
}

/// Accumulate term frequencies from matching documents, keyed by (field, term_text).
/// This matches MLT's per-field term tracking.
fn accumulate_term_frequencies(
    doc_addresses: &std::collections::HashSet<DocAddress>,
    target_fields: &[Field],
    searcher: &Searcher,
    tantivy_schema: &Schema,
    tokenizer_manager: &TokenizerManager,
) -> HashMap<(Field, String), usize> {
    let mut term_frequencies: HashMap<(Field, String), usize> = HashMap::new();

    for doc_address in doc_addresses {
        let doc: TantivyDocument = match searcher.doc(*doc_address) {
            Ok(d) => d,
            Err(_) => continue,
        };

        for &field in target_fields {
            let field_entry = tantivy_schema.get_field_entry(field);

            for compact_value in doc.get_all(field) {
                let field_value: OwnedValue = compact_value.into();
                match (field_entry.field_type(), &field_value) {
                    (FieldType::Str(text_options), OwnedValue::Str(text)) => {
                        if let Some(indexing_options) = text_options.get_indexing_options() {
                            if let Some(mut tokenizer) =
                                tokenizer_manager.get(indexing_options.tokenizer())
                            {
                                let mut stream = tokenizer.token_stream(text);
                                while stream.advance() {
                                    let token_text = stream.token().text.clone();
                                    *term_frequencies.entry((field, token_text)).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                    (FieldType::I64(_), OwnedValue::I64(val)) => {
                        let token_text = val.to_string();
                        *term_frequencies.entry((field, token_text)).or_insert(0) += 1;
                    }
                    (FieldType::U64(_), OwnedValue::U64(val)) => {
                        let token_text = val.to_string();
                        *term_frequencies.entry((field, token_text)).or_insert(0) += 1;
                    }
                    (FieldType::F64(_), OwnedValue::F64(val)) => {
                        let token_text = val.to_string();
                        *term_frequencies.entry((field, token_text)).or_insert(0) += 1;
                    }
                    (FieldType::Bool(_), OwnedValue::Bool(val)) => {
                        let token_text = val.to_string();
                        *term_frequencies.entry((field, token_text)).or_insert(0) += 1;
                    }
                    (FieldType::Date(_), OwnedValue::Date(val)) => {
                        let token_text = val.into_timestamp_micros().to_string();
                        *term_frequencies.entry((field, token_text)).or_insert(0) += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    term_frequencies
}

/// Create a Tantivy Term for the given field and term_text.
fn create_term(field: Field, term_text: &str, field_type: &FieldType) -> Option<Term> {
    match field_type {
        FieldType::Str(_) => Some(Term::from_field_text(field, term_text)),
        FieldType::I64(_) => term_text
            .parse::<i64>()
            .ok()
            .map(|val| Term::from_field_i64(field, val)),
        FieldType::U64(_) => term_text
            .parse::<u64>()
            .ok()
            .map(|val| Term::from_field_u64(field, val)),
        FieldType::F64(_) => term_text
            .parse::<f64>()
            .ok()
            .map(|val| Term::from_field_f64(field, val)),
        FieldType::Bool(_) => term_text
            .parse::<bool>()
            .ok()
            .map(|val| Term::from_field_bool(field, val)),
        FieldType::Date(_) => term_text
            .parse::<i64>()
            .ok()
            .map(|micros| Term::from_field_date(field, DateTime::from_timestamp_micros(micros))),
        _ => None,
    }
}

/// Contains the `pdb.related_terms` function.
#[pgrx::pg_schema]
mod pdb {
    use super::*;

    /// Returns related terms and their TF-IDF weights for a given query term.
    ///
    /// This function finds all documents containing the query_term, extracts terms from those
    /// documents, and scores them using TF-IDF weighting aligned with Tantivy's MoreLikeThis.
    ///
    /// Unlike the original implementation, this version tracks terms per-field (matching MLT
    /// behavior) and returns (field, term, weight) tuples. This ensures document frequency
    /// is computed per (field, term) pair, matching Tantivy's internal MLT implementation.
    #[allow(clippy::too_many_arguments)]
    #[pg_extern(immutable, parallel_safe)]
    pub fn related_terms(
        query_term: &str,
        relation: PgRelation,
        fields: default!(Option<Vec<String>>, "NULL"),
        min_doc_frequency: default!(Option<i32>, "1"),
        max_doc_frequency: default!(Option<i32>, "NULL"),
        min_term_frequency: default!(Option<i32>, "1"),
        max_query_terms: default!(Option<i32>, "25"),
        min_word_length: default!(Option<i32>, "NULL"),
        max_word_length: default!(Option<i32>, "NULL"),
    ) -> TableIterator<
        'static,
        (
            name!(field, String),
            name!(term, String),
            name!(weight, f32),
        ),
    > {
        let heap_oid = relation.oid();

        // Find the BM25 index for this relation
        let index_relation = match locate_bm25_index(heap_oid) {
            Some(rel) => rel,
            None => {
                pgrx::error!("no BM25 index found for relation '{}'", relation.name());
            }
        };

        // Open schema and set up tokenizers
        let schema = match index_relation.schema() {
            Ok(s) => s,
            Err(e) => {
                pgrx::error!("failed to open schema: {}", e);
            }
        };

        // Open the tantivy index
        let directory = MvccSatisfies::Snapshot.directory(&index_relation);
        let mut index = match Index::open(directory) {
            Ok(idx) => idx,
            Err(e) => {
                pgrx::error!("failed to open index: {}", e);
            }
        };

        if let Err(e) = crate::index::setup_tokenizers(&index_relation, &mut index) {
            pgrx::error!("failed to setup tokenizers: {}", e);
        }

        let reader = match index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
        {
            Ok(r) => r,
            Err(e) => {
                pgrx::error!("failed to create reader: {}", e);
            }
        };
        let searcher = reader.searcher();

        let tantivy_schema = schema.tantivy_schema();
        let tokenizer_manager = index.tokenizers();

        // Determine which fields to search
        let target_fields: Vec<Field> = if let Some(ref field_names) = fields {
            field_names
                .iter()
                .filter_map(|name| {
                    let search_field = schema.search_field(name)?;
                    if search_field.is_json() {
                        pgrx::error!("JSON fields are not supported for related_terms");
                    }
                    Some(search_field.field())
                })
                .collect()
        } else {
            // Use all indexed non-JSON fields
            schema
                .fields()
                .filter_map(|(field, entry)| {
                    if entry.is_indexed() && !entry.field_type().is_json() && entry.name() != "ctid"
                    {
                        Some(field)
                    } else {
                        None
                    }
                })
                .collect()
        };

        if target_fields.is_empty() {
            return TableIterator::new(vec![]);
        }

        // Build term queries and get excluded tokens
        let (subqueries, excluded_tokens) = build_term_queries(
            query_term,
            &target_fields,
            &tantivy_schema,
            &tokenizer_manager,
        );

        if subqueries.is_empty() {
            pgrx::error!(
                "query_term '{}' could not be parsed for any target field",
                query_term
            );
        }

        let query = BooleanQuery::new(subqueries);

        // Collect all matching documents
        let doc_addresses = match searcher.search(&query, &DocSetCollector) {
            Ok(addrs) => addrs,
            Err(e) => {
                pgrx::error!("search failed: {}", e);
            }
        };

        if doc_addresses.is_empty() {
            return TableIterator::new(vec![]);
        }

        // Accumulate term frequencies per (field, term) - matching MLT behavior
        let term_frequencies = accumulate_term_frequencies(
            &doc_addresses,
            &target_fields,
            &searcher,
            &tantivy_schema,
            &tokenizer_manager,
        );

        // Get total document count for IDF calculation
        let num_docs: u64 = searcher
            .segment_readers()
            .iter()
            .map(|r| r.num_docs() as u64)
            .sum();

        // Score and filter terms
        let min_tf = min_term_frequency.unwrap_or(1) as usize;
        let min_df = min_doc_frequency.unwrap_or(1) as u64;
        let max_df = max_doc_frequency.map(|v| v as u64);
        let min_len = min_word_length.map(|v| v as usize);
        let max_len = max_word_length.map(|v| v as usize);
        let max_terms = max_query_terms.unwrap_or(25) as usize;

        let mut score_terms: BinaryHeap<Reverse<ScoreTerm>> = BinaryHeap::new();

        for ((field, term_text), term_freq) in term_frequencies {
            // Skip excluded tokens (the query term tokens)
            if excluded_tokens.contains(&term_text) {
                continue;
            }

            // Filter by term frequency
            if term_freq < min_tf {
                continue;
            }

            // Filter by word length
            if let Some(min) = min_len {
                if term_text.len() < min {
                    continue;
                }
            }
            if let Some(max) = max_len {
                if term_text.len() > max {
                    continue;
                }
            }

            // Get document frequency for this specific (field, term) pair
            // This matches MLT's per-field DF calculation
            let field_entry = tantivy_schema.get_field_entry(field);
            let doc_freq = match create_term(field, &term_text, field_entry.field_type()) {
                Some(term) => searcher.doc_freq(&term).unwrap_or(0),
                None => continue,
            };

            // Filter by document frequency
            if doc_freq < min_df {
                continue;
            }
            if let Some(max) = max_df {
                if doc_freq > max {
                    continue;
                }
            }

            if doc_freq == 0 {
                continue;
            }

            // Compute TF-IDF score
            let idf_score = idf(doc_freq, num_docs);
            let score = (term_freq as f32) * idf_score;

            let field_name = field_entry.name().to_string();

            // Maintain top N terms using a min-heap
            if score_terms.len() >= max_terms {
                if let Some(Reverse(min_term)) = score_terms.peek() {
                    if score > min_term.score {
                        score_terms.pop();
                        score_terms.push(Reverse(ScoreTerm {
                            field_name,
                            term_text,
                            score,
                        }));
                    }
                }
            } else {
                score_terms.push(Reverse(ScoreTerm {
                    field_name,
                    term_text,
                    score,
                }));
            }
        }

        // Convert to sorted results (descending by score)
        let mut results: Vec<(String, String, f32)> = score_terms
            .into_iter()
            .map(|Reverse(st)| (st.field_name, st.term_text, st.score))
            .collect();
        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        TableIterator::new(results)
    }
}
