use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::TantivyValue;
use pgrx::spi::SpiError;
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
        self.mlt.inner.min_doc_frequency = Some(value);
        self
    }

    #[must_use]
    pub fn with_max_doc_frequency(mut self, value: u64) -> Self {
        self.mlt.inner.max_doc_frequency = Some(value);
        self
    }

    #[must_use]
    pub fn with_min_term_frequency(mut self, value: usize) -> Self {
        self.mlt.inner.min_term_frequency = Some(value);
        self
    }

    #[must_use]
    pub fn with_max_query_terms(mut self, value: usize) -> Self {
        self.mlt.inner.max_query_terms = Some(value);
        self
    }

    #[must_use]
    pub fn with_min_word_length(mut self, value: usize) -> Self {
        self.mlt.inner.min_word_length = Some(value);
        self
    }

    #[must_use]
    pub fn with_max_word_length(mut self, value: usize) -> Self {
        self.mlt.inner.max_word_length = Some(value);
        self
    }

    #[must_use]
    pub fn with_boost_factor(mut self, value: f32) -> Self {
        self.mlt.inner.boost_factor = Some(value);
        self
    }

    #[must_use]
    pub fn with_stop_words(mut self, value: Vec<String>) -> Self {
        self.mlt.inner.stop_words = value;
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
        let key_field_names = schema.key_field_names();
        let key_field_types = schema.key_field_types();
        let categorized_fields = schema.categorized_fields();

        // For composite keys, we currently only support single key field for more_like_this
        // TODO: Extend to support composite key matching
        if key_field_names.len() > 1 {
            panic!("more_like_this currently only supports single key fields, not composite keys");
        }
        
        let key_field_name = &key_field_names[0];
        let key_field_type = key_field_types[0];

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
                                key_field_name
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
