use crate::index::mvcc::MVCCDirectory;
use crate::postgres::types::TantivyValue;
use crate::postgres::utils::categorize_fields;
use crate::query::AsFieldType;
use crate::schema::SearchIndexSchema;
use tantivy::index::Index;
use tantivy::query::{
    BooleanQuery, EnableScoring, MoreLikeThis as TantivyMoreLikeThis, Query, Weight,
};
use tantivy::schema::{Field, OwnedValue, Value};
use tantivy::{Result, Searcher, TantivyError};

#[derive(Debug, Default, Clone)]
pub struct MoreLikeThis {
    inner: TantivyMoreLikeThis,
}

impl MoreLikeThis {
    pub fn query_with_document_fields<'a, V: Value<'a>>(
        &self,
        searcher: &Searcher,
        doc_fields: &[(Field, Vec<V>)],
    ) -> Result<BooleanQuery> {
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
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> Result<Box<dyn Weight>> {
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

    pub fn with_document(
        self,
        key_value: OwnedValue,
        index_oid: pgrx::pg_sys::Oid,
    ) -> MoreLikeThisQuery {
        let index_relation = unsafe { pgrx::PgRelation::open(index_oid) };
        let heap_relation = index_relation
            .heap_relation()
            .expect("more_like_this: index should have a heap relation");
        let directory = MVCCDirectory::snapshot(index_relation.oid());
        let index = Index::open(directory).expect("more_like_this: should be able to open index");
        let schema = SearchIndexSchema::open(index.schema(), &index_relation);
        let key_field_name = schema.key_field().name.0;
        let key_oid = (&index_relation, &schema).key_field().1;
        let categorized_fields = categorize_fields(&index_relation.tuple_desc(), &schema);

        let doc_fields: Vec<(Field, Vec<OwnedValue>)> = pgrx::Spi::connect(|client| {
            let mut doc_fields = Vec::new();
            let result = client
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
                            .try_into_datum(key_oid)
                            .expect("more_like_this: should be able to convert key value to datum")
                            .into()]
                    },
                )?
                .first();

            for (field, categorized) in categorized_fields {
                if field.name.0 == "ctid" {
                    continue;
                }

                if let Some(datum) = result.get_datum_by_name(field.name.0)? {
                    if categorized.is_array {
                        let values = unsafe {
                            TantivyValue::try_from_datum_array(datum, categorized.base_oid)
                                .expect("more_like_this: should be able to convert array to tantivy value")
                                .into_iter()
                                .map(|v| v.into())
                                .collect::<Vec<_>>()
                        };
                        doc_fields.push((field.id.0, values));
                    } else if categorized.is_json {
                        let values = unsafe {
                            TantivyValue::try_from_datum_json(datum, categorized.base_oid)
                                .expect("more_like_this: should be able to convert json to tantivy value")
                                .into_iter()
                                .map(|v| v.into())
                                .collect::<Vec<_>>()
                        };
                        doc_fields.push((field.id.0, values));
                    } else {
                        let value = unsafe {
                            TantivyValue::try_from_datum(datum, categorized.base_oid)
                                .expect("more_like_this: should be able to convert datum to tantivy value")
                        };
                        doc_fields.push((field.id.0, vec![value.into()]));
                    }
                }
            }

            Ok::<_, pgrx::spi::SpiError>(doc_fields)
        })
        .expect("more_like_this: should be able to construct document");

        MoreLikeThisQuery {
            mlt: self.mlt,
            doc_fields,
        }
    }

    pub fn with_document_fields(
        self,
        doc_fields: Vec<(Field, Vec<OwnedValue>)>,
    ) -> MoreLikeThisQuery {
        MoreLikeThisQuery {
            mlt: self.mlt,
            doc_fields,
        }
    }
}
