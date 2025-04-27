use crate::index::mvcc::MVCCDirectory;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::types::TantivyValue;
use std::collections::HashMap;
use tantivy::index::Index;
use tantivy::query::{
    BooleanQuery, EnableScoring, MoreLikeThis as TantivyMoreLikeThis, Query, ScoreTerm, Weight,
};
use tantivy::schema::{Field, OwnedValue, Value};
use tantivy::{Result, Searcher, TantivyError, Term};

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
        key_field: pgrx::AnyElement,
        index_oid: pgrx::pg_sys::Oid,
    ) -> MoreLikeThisQuery {
        let index_relation = unsafe { pgrx::PgRelation::open(index_oid) };
        let heap_relation = index_relation.heap_relation();
        let heap_oid = heap_relation
            .expect("index should have a heap relation")
            .oid();
        let options = unsafe { SearchIndexCreateOptions::from_relation(&index_relation) };
        let key_field = options.get_key_field().expect("key_field is required").0;
        let directory = MVCCDirectory::snapshot(index_relation.oid());
        let index = Index::open(directory).expect("custom_scan: should be able to open index");
        let schema = index.schema();

        let doc_fields: Vec<(Field, Vec<OwnedValue>)> = pgrx::Spi::connect(|client| {
            if let Some(htup) = client
                .select(
                    &format!(
                        "SELECT * FROM {}::regclass WHERE {} = $1",
                        heap_oid.as_u32(),
                        key_field
                    ),
                    None,
                    &[key_field.into()],
                )?
                .first()
                .get_heap_tuple()?
            {
                let mut fields_map = vec![];
                for (field, entry) in schema.fields() {
                    let spi_datum = htup.get_datum_by_name(entry.name())?;
                    if let Some(datum) = spi_datum.value::<pgrx::pg_sys::Datum>()? {
                        let value = unsafe { TantivyValue::try_from_datum(
                            datum,
                            pgrx::PgOid::from(spi_datum.oid()),
                        ).expect("should be able to convert datum to tantivy value") };
                        fields_map.push((field, vec![value.into()]));
                    }
                }

                Ok::<_, pgrx::spi::SpiError>(fields_map)
            } else {
                Ok::<_, pgrx::spi::SpiError>(vec![])
            }
        }).expect("should be able to construct document");

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
