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

use crate::api::version::Version;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::TantivyValue;
use crate::postgres::utils::{FieldSource, strip_tokenizer_cast};
use crate::schema::SearchFieldType;
use pgrx::spi::SpiError;
use serde::{Deserialize, Serialize};
use tantivy::query::{
    BooleanQuery, EnableScoring, MoreLikeThis as TantivyMoreLikeThis, Query, Weight,
};
use tantivy::schema::{Field, OwnedValue, Value};
use tantivy::{Searcher, TantivyError};

#[derive(Debug, Default, Clone)]
pub struct MoreLikeThis {
    inner: TantivyMoreLikeThis,
}

impl MoreLikeThis {
    pub fn query_with_document_fields<'a, V: Value<'a>>(
        &self,
        searcher: &Searcher,
        doc_fields: &[(Field, Vec<V>)],
    ) -> tantivy::Result<BooleanQuery> {
        self.inner.query_with_document_fields(searcher, doc_fields)
    }
}

#[derive(Debug, Clone)]
pub struct MoreLikeThisQuery {
    mlt: MoreLikeThis,
    doc_fields: Vec<(Field, Vec<PdbOwnedValue>)>,
    index_created_by_version: Option<Version>,
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
            .map(|(field, values)| {
                (
                    *field,
                    values
                        .iter()
                        .map(|v| v.clone().into_tantivy_value(self.index_created_by_version))
                        .collect::<Vec<OwnedValue>>(),
                )
            })
            .collect::<Vec<_>>();
        let value_refs = values
            .iter()
            .map(|(field, values)| (*field, values.iter().collect::<Vec<&OwnedValue>>()))
            .collect::<Vec<_>>();

        self.mlt
            .query_with_document_fields(searcher, &value_refs)?
            .weight(enable_scoring)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MoreLikeThisOptions {
    pub min_doc_frequency: Option<u64>,
    pub max_doc_frequency: Option<u64>,
    pub min_term_frequency: Option<usize>,
    pub max_query_terms: Option<usize>,
    pub min_word_length: Option<usize>,
    pub max_word_length: Option<usize>,
    pub boost_factor: Option<f32>,
    pub stopwords: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct MoreLikeThisQueryBuilder {
    mlt: MoreLikeThis,
    index_created_by_version: Option<Version>,
}

impl MoreLikeThisQueryBuilder {
    pub fn new(options: MoreLikeThisOptions, index_created_by_version: Option<Version>) -> Self {
        let defaults = TantivyMoreLikeThis::default();
        Self {
            mlt: MoreLikeThis {
                inner: TantivyMoreLikeThis {
                    // ParadeDB includes terms occurring once, unlike Tantivy's defaults.
                    min_doc_frequency: Some(options.min_doc_frequency.unwrap_or(1)),
                    min_term_frequency: Some(options.min_term_frequency.unwrap_or(1)),
                    max_doc_frequency: options.max_doc_frequency,
                    max_query_terms: options.max_query_terms.or(defaults.max_query_terms),
                    min_word_length: options.min_word_length,
                    max_word_length: options.max_word_length,
                    boost_factor: options.boost_factor.or(defaults.boost_factor),
                    stop_words: options.stopwords.unwrap_or_default(),
                },
            },
            index_created_by_version,
        }
    }

    pub fn with_field_value(
        self,
        lookup_field: crate::api::FieldName,
        key_value: PdbOwnedValue,
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
        let categorized_fields = schema.categorized_fields();
        let source = categorized_fields
            .iter()
            .find(|(field, _)| field.field_name() == &lookup_field)
            .map(|(_, field)| field.source)
            .unwrap_or_else(|| {
                panic!("more_like_this: lookup field '{lookup_field}' does not exist")
            });
        let attno = match source {
            FieldSource::Heap { attno } => Some(attno),
            // Tokenizer casts can name a heap column; computed expressions cannot.
            FieldSource::Expression { att_idx } => unsafe {
                index_relation
                    .index_expressions()
                    .get_ptr(att_idx)
                    .and_then(|expr| {
                        crate::nodecast!(Var, T_Var, strip_tokenizer_cast(expr.cast()))
                            .filter(|var| (**var).varattno > 0)
                            .map(|var| ((*var).varattno - 1) as usize)
                    })
            },
            FieldSource::CompositeField { .. } => None,
        }
        .expect("more_like_this(key_value => ...) requires a heap column on the left-hand side");
        let tuple_desc = heap_relation.tuple_desc();
        let attribute = tuple_desc.get(attno).expect("lookup column should exist");
        let lookup_field = attribute.name();
        let lookup_type = attribute.type_oid();

        let maybe_doc_fields: Result<Vec<(Field, Vec<PdbOwnedValue>)>, SpiError> =
            pgrx::Spi::connect(|client| {
                let mut doc_fields = Vec::new();
                // Bound to a local rather than built inline: in edition 2024 the temporary array
                // produced by the `unsafe` block's tail expression is dropped at the end of that
                // block instead of living until the end of the enclosing statement.
                let key_args = unsafe {
                    [pgrx::datum::DatumWithOid::new(
                        TantivyValue(key_value)
                            .try_into_datum(lookup_type)
                            .expect("more_like_this: should be able to convert key value to datum"),
                        lookup_type.value(),
                    )]
                };
                let result = client
                    .select(
                        &format!(
                            // Duplicate lookup values intentionally select an arbitrary source row.
                            "SELECT * FROM {}.{} WHERE {} = $1 LIMIT 1",
                            pgrx::spi::quote_identifier(heap_relation.namespace()),
                            pgrx::spi::quote_identifier(heap_relation.name()),
                            pgrx::spi::quote_identifier(lookup_field)
                        ),
                        None,
                        &key_args,
                    )?
                    .first();

                for (search_field, categorized) in categorized_fields.iter() {
                    if search_field.is_ctid() {
                        continue;
                    }

                    let is_vector =
                        matches!(search_field.field_type(), SearchFieldType::Vector(..));

                    if let Some(ref fields) = fields {
                        if !fields.contains(&search_field.field_name().clone().into_inner()) {
                            continue;
                        }

                        if search_field.is_json() {
                            panic!("json fields are not supported for more_like_this");
                        }

                        if is_vector {
                            panic!("vector fields are not supported for more_like_this");
                        }
                    }

                    if categorized.is_json || is_vector {
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
                                .map(|v| v.0)
                                .collect::<Vec<_>>()
                            };
                            doc_fields.push((search_field.field(), values));
                        } else {
                            let value = unsafe {
                                TantivyValue::try_from_datum(datum, categorized.base_oid)
                                .expect("more_like_this: should be able to convert datum to tantivy value")
                            };
                            doc_fields.push((search_field.field(), vec![value.0]));
                        }
                    }
                }

                Ok::<_, SpiError>(doc_fields)
            });

        match maybe_doc_fields {
            Ok(doc_fields) => Some(MoreLikeThisQuery {
                mlt: self.mlt,
                doc_fields,
                index_created_by_version: self.index_created_by_version,
            }),
            Err(_) => None,
        }
    }

    pub fn with_document(self, doc_fields: Vec<(Field, Vec<PdbOwnedValue>)>) -> MoreLikeThisQuery {
        MoreLikeThisQuery {
            mlt: self.mlt,
            doc_fields,
            index_created_by_version: self.index_created_by_version,
        }
    }
}
