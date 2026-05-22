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

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::TantivyValue;
use pgrx::spi::SpiError;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use tantivy::query::{BooleanQuery, BoostQuery, EnableScoring, Occur, Query, TermQuery, Weight};
use tantivy::schema::{Field, FieldType, IndexRecordOption, OwnedValue, Term, Value};
use tantivy::tokenizer::{FacetTokenizer, PreTokenizedStream, Token, TokenStream, Tokenizer};
use tantivy::{Searcher, TantivyError};

#[derive(Debug, PartialEq)]
struct ScoreTerm {
    term: Term,
    score: f32,
}

impl ScoreTerm {
    fn new(term: Term, score: f32) -> Self {
        Self { term, score }
    }
}

impl Eq for ScoreTerm {}

impl PartialOrd for ScoreTerm {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoreTerm {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score
            .partial_cmp(&other.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[derive(Debug, Clone)]
pub struct MoreLikeThis {
    min_doc_frequency: Option<u64>,
    max_doc_frequency: Option<u64>,
    min_term_frequency: Option<usize>,
    max_term_frequency: Option<usize>,
    max_query_terms: Option<usize>,
    min_word_length: Option<usize>,
    max_word_length: Option<usize>,
    boost_factor: Option<f32>,
    stop_words: Vec<String>,
}

impl Default for MoreLikeThis {
    fn default() -> Self {
        Self {
            min_doc_frequency: Some(5),
            max_doc_frequency: None,
            min_term_frequency: Some(2),
            max_term_frequency: None,
            max_query_terms: Some(25),
            min_word_length: None,
            max_word_length: None,
            boost_factor: Some(1.0),
            stop_words: vec![],
        }
    }
}

impl MoreLikeThis {
    pub fn query_with_document_fields<'a, V: Value<'a>>(
        &self,
        searcher: &Searcher,
        doc_fields: &[(Field, Vec<V>)],
    ) -> tantivy::Result<BooleanQuery> {
        if doc_fields.is_empty() {
            return Err(TantivyError::InvalidArgument(
                "Cannot create more like this query on empty field values. The document may not have stored fields"
                    .to_string(),
            ));
        }

        let mut per_field_term_frequencies = HashMap::new();
        for (field, values) in doc_fields {
            self.add_term_frequencies(searcher, *field, values, &mut per_field_term_frequencies)?;
        }

        let score_terms = self.create_score_terms(searcher, per_field_term_frequencies)?;
        Ok(self.create_query(score_terms))
    }

    fn create_query(&self, mut score_terms: Vec<ScoreTerm>) -> BooleanQuery {
        score_terms.sort_by(|left_ts, right_ts| right_ts.cmp(left_ts));
        let best_score = score_terms.first().map_or(1f32, |x| x.score);
        let queries = score_terms
            .into_iter()
            .map(|ScoreTerm { term, score }| {
                let mut query: Box<dyn Query> =
                    Box::new(TermQuery::new(term, IndexRecordOption::Basic));
                if let Some(factor) = self.boost_factor {
                    query = Box::new(BoostQuery::new(query, score * factor / best_score));
                }
                (Occur::Should, query)
            })
            .collect::<Vec<_>>();
        BooleanQuery::from(queries)
    }

    fn add_term_frequencies<'a, V: Value<'a>>(
        &self,
        searcher: &Searcher,
        field: Field,
        values: &[V],
        term_frequencies: &mut HashMap<Term, usize>,
    ) -> tantivy::Result<()> {
        let schema = searcher.schema();
        let tokenizer_manager = searcher.index().tokenizers();

        let field_entry = schema.get_field_entry(field);
        if !field_entry.is_indexed() {
            return Ok(());
        }

        match field_entry.field_type() {
            FieldType::Facet(_) => {
                let facets: Vec<&str> = values
                    .iter()
                    .map(|value| {
                        value.as_facet().ok_or_else(|| {
                            TantivyError::InvalidArgument("invalid field value".to_string())
                        })
                    })
                    .collect::<tantivy::Result<Vec<_>>>()?;
                for fake_str in facets {
                    FacetTokenizer::default()
                        .token_stream(fake_str)
                        .process(&mut |token| {
                            if !self.is_noise_word(token.text.clone()) {
                                let term = Term::from_field_text(field, &token.text);
                                *term_frequencies.entry(term).or_insert(0) += 1;
                            }
                        });
                }
            }
            FieldType::Str(text_options) => {
                let mut tokenizer_opt = text_options
                    .get_indexing_options()
                    .map(|options| options.tokenizer())
                    .and_then(|tokenizer_name| tokenizer_manager.get(tokenizer_name));

                let sink = &mut |token: &Token| {
                    if !self.is_noise_word(token.text.clone()) {
                        let term = Term::from_field_text(field, &token.text);
                        *term_frequencies.entry(term).or_insert(0) += 1;
                    }
                };

                for value in values {
                    if let Some(text) = value.as_str() {
                        let tokenizer = match &mut tokenizer_opt {
                            None => continue,
                            Some(tokenizer) => tokenizer,
                        };

                        let mut token_stream = tokenizer.token_stream(text);
                        token_stream.process(sink);
                    } else if let Some(tok_str) = value.as_pre_tokenized_text() {
                        let mut token_stream = PreTokenizedStream::from(*tok_str.clone());
                        token_stream.process(sink);
                    }
                }
            }
            FieldType::U64(_) => {
                for value in values {
                    let val = value.as_u64().ok_or_else(|| {
                        TantivyError::InvalidArgument("invalid value".to_string())
                    })?;
                    if !self.is_noise_word(val.to_string()) {
                        let term = Term::from_field_u64(field, val);
                        *term_frequencies.entry(term).or_insert(0) += 1;
                    }
                }
            }
            FieldType::Date(_) => {
                for value in values {
                    let timestamp = value.as_datetime().ok_or_else(|| {
                        TantivyError::InvalidArgument("invalid value".to_string())
                    })?;
                    let term = Term::from_field_date_for_search(field, timestamp);
                    *term_frequencies.entry(term).or_insert(0) += 1;
                }
            }
            FieldType::I64(_) => {
                for value in values {
                    let val = value.as_i64().ok_or_else(|| {
                        TantivyError::InvalidArgument("invalid value".to_string())
                    })?;
                    if !self.is_noise_word(val.to_string()) {
                        let term = Term::from_field_i64(field, val);
                        *term_frequencies.entry(term).or_insert(0) += 1;
                    }
                }
            }
            FieldType::F64(_) => {
                for value in values {
                    let val = value.as_f64().ok_or_else(|| {
                        TantivyError::InvalidArgument("invalid value".to_string())
                    })?;
                    if !self.is_noise_word(val.to_string()) {
                        let term = Term::from_field_f64(field, val);
                        *term_frequencies.entry(term).or_insert(0) += 1;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn create_score_terms(
        &self,
        searcher: &Searcher,
        per_field_term_frequencies: HashMap<Term, usize>,
    ) -> tantivy::Result<Vec<ScoreTerm>> {
        let mut score_terms: BinaryHeap<Reverse<ScoreTerm>> = BinaryHeap::new();
        let num_docs = searcher
            .segment_readers()
            .iter()
            .map(|segment_reader| segment_reader.num_docs() as u64)
            .sum::<u64>();

        for (term, term_frequency) in per_field_term_frequencies.iter() {
            if self
                .min_term_frequency
                .map(|min_term_frequency| *term_frequency < min_term_frequency)
                .unwrap_or(false)
            {
                continue;
            }

            if self
                .max_term_frequency
                .map(|max_term_frequency| *term_frequency > max_term_frequency)
                .unwrap_or(false)
            {
                continue;
            }

            let doc_freq = searcher.doc_freq(term)?;

            if self
                .min_doc_frequency
                .map(|min_doc_frequency| doc_freq < min_doc_frequency)
                .unwrap_or(false)
            {
                continue;
            }

            if self
                .max_doc_frequency
                .map(|max_doc_frequency| doc_freq > max_doc_frequency)
                .unwrap_or(false)
            {
                continue;
            }

            if doc_freq == 0 {
                continue;
            }

            let idf = Self::idf(doc_freq, num_docs);
            let score = (*term_frequency as f32) * idf;
            if let Some(limit) = self.max_query_terms {
                if score_terms.len() > limit {
                    let least_significant_term_score = score_terms.peek().unwrap().0.score;
                    if least_significant_term_score < score {
                        score_terms.peek_mut().unwrap().0 = ScoreTerm::new(term.clone(), score);
                    }
                } else {
                    score_terms.push(Reverse(ScoreTerm::new(term.clone(), score)));
                }
            } else {
                score_terms.push(Reverse(ScoreTerm::new(term.clone(), score)));
            }
        }

        Ok(score_terms
            .into_iter()
            .map(|reverse_score_term| reverse_score_term.0)
            .collect())
    }

    fn is_noise_word(&self, word: String) -> bool {
        let word_length = word.len();
        if word_length == 0 {
            return true;
        }
        if self
            .min_word_length
            .map(|min| word_length < min)
            .unwrap_or(false)
        {
            return true;
        }
        if self
            .max_word_length
            .map(|max| word_length > max)
            .unwrap_or(false)
        {
            return true;
        }

        self.stop_words.contains(&word)
    }

    fn idf(doc_freq: u64, doc_count: u64) -> f32 {
        assert!(doc_count >= doc_freq, "{doc_count} >= {doc_freq}");
        let x = ((doc_count - doc_freq) as f32 + 0.5) / (doc_freq as f32 + 0.5);
        (1.0 + x).ln()
    }
}

#[derive(Debug, Clone)]
pub struct MoreLikeThisQuery {
    mlt: MoreLikeThis,
    doc_fields: Vec<(Field, Vec<OwnedValue>)>,
}

impl MoreLikeThisQuery {
    pub fn builder() -> MoreLikeThisQueryBuilder {
        MoreLikeThisQueryBuilder::default()
    }
}

impl Query for MoreLikeThisQuery {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        let searcher = match enable_scoring {
            EnableScoring::Enabled { searcher, .. } => searcher,
            EnableScoring::Disabled { .. } => {
                let err = "MoreLikeThisQuery requires to enable scoring.".to_string();
                return Err(TantivyError::InvalidArgument(err));
            }
        };

        let values = self
            .doc_fields
            .iter()
            .map(|(field, values)| (*field, values.iter().collect::<Vec<&OwnedValue>>()))
            .collect::<Vec<_>>();

        self.mlt
            .query_with_document_fields(searcher, &values)?
            .weight(enable_scoring)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MoreLikeThisQueryBuilder {
    mlt: MoreLikeThis,
}

impl MoreLikeThisQueryBuilder {
    #[must_use]
    pub fn with_min_doc_frequency(mut self, value: u64) -> Self {
        self.mlt.min_doc_frequency = Some(value);
        self
    }

    #[must_use]
    pub fn with_max_doc_frequency(mut self, value: u64) -> Self {
        self.mlt.max_doc_frequency = Some(value);
        self
    }

    #[must_use]
    pub fn with_min_term_frequency(mut self, value: usize) -> Self {
        self.mlt.min_term_frequency = Some(value);
        self
    }

    #[must_use]
    pub fn with_max_term_frequency(mut self, value: usize) -> Self {
        self.mlt.max_term_frequency = Some(value);
        self
    }

    #[must_use]
    pub fn with_max_query_terms(mut self, value: usize) -> Self {
        self.mlt.max_query_terms = Some(value);
        self
    }

    #[must_use]
    pub fn with_min_word_length(mut self, value: usize) -> Self {
        self.mlt.min_word_length = Some(value);
        self
    }

    #[must_use]
    pub fn with_max_word_length(mut self, value: usize) -> Self {
        self.mlt.max_word_length = Some(value);
        self
    }

    #[must_use]
    pub fn with_boost_factor(mut self, value: f32) -> Self {
        self.mlt.boost_factor = Some(value);
        self
    }

    #[must_use]
    pub fn with_stop_words(mut self, value: Vec<String>) -> Self {
        self.mlt.stop_words = value;
        self
    }

    pub fn with_key_value(
        self,
        key_value: OwnedValue,
        fields: Option<Vec<String>>,
        index_oid: pgrx::pg_sys::Oid,
    ) -> Option<MoreLikeThisQuery> {
        let index_relation = PgSearchRelation::open(index_oid);
        let heap_relation = index_relation
            .heap_relation()
            .expect("more_like_this: index should have a heap relation");
        let schema = index_relation
            .schema()
            .expect("more_like_this: should be able to open schema");
        let (key_field_name, key_field_type) = (schema.key_field_name(), schema.key_field_type());
        let categorized_fields = schema.categorized_fields();

        let maybe_doc_fields: Result<Vec<(Field, Vec<OwnedValue>)>, SpiError> = pgrx::Spi::connect(
            |client| {
                let mut doc_fields = Vec::new();
                let result =
                    client
                        .select(
                            &format!(
                                "SELECT * FROM {}.{} WHERE {} = $1",
                                pgrx::spi::quote_identifier(heap_relation.namespace()),
                                pgrx::spi::quote_identifier(heap_relation.name()),
                                pgrx::spi::quote_identifier(key_field_name.root())
                            ),
                            None,
                            unsafe {
                                &[TantivyValue(key_value)
                            .try_into_datum(key_field_type.typeoid())
                            .expect("more_like_this: should be able to convert key value to datum")
                            .into()]
                            },
                        )?
                        .first();

                for (search_field, categorized) in categorized_fields.iter() {
                    if search_field.is_ctid() {
                        continue;
                    }

                    if let Some(ref fields) = fields {
                        if !fields.contains(&search_field.field_name().clone().into_inner()) {
                            continue;
                        }

                        if search_field.is_json() {
                            panic!("json fields are not supported for more_like_this");
                        }
                    }

                    if categorized.is_json {
                        continue;
                    }

                    if let Some(datum) =
                        result.get_datum_by_name(search_field.field_name().root())?
                    {
                        if categorized.is_array {
                            let values = unsafe {
                                TantivyValue::try_from_datum_array(datum, categorized.base_oid)
                                .expect("more_like_this: should be able to convert array to tantivy value")
                                .into_iter()
                                .map(|v| v.into())
                                .collect::<Vec<_>>()
                            };
                            doc_fields.push((search_field.field(), values));
                        } else {
                            let value = unsafe {
                                TantivyValue::try_from_datum(datum, categorized.base_oid)
                                .expect("more_like_this: should be able to convert datum to tantivy value")
                            };
                            doc_fields.push((search_field.field(), vec![value.into()]));
                        }
                    }
                }

                Ok::<_, SpiError>(doc_fields)
            },
        );

        match maybe_doc_fields {
            Ok(doc_fields) => Some(MoreLikeThisQuery {
                mlt: self.mlt,
                doc_fields,
            }),
            Err(_) => None,
        }
    }

    pub fn with_document(self, doc_fields: Vec<(Field, Vec<OwnedValue>)>) -> MoreLikeThisQuery {
        MoreLikeThisQuery {
            mlt: self.mlt,
            doc_fields,
        }
    }
}
