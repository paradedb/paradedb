use crate::query::proximity::query::ProximityQuery;
use crate::query::proximity::scorer::ProximityScorer;
use tantivy::fieldnorm::FieldNormReader;
use tantivy::query::{does_not_match, Bm25Weight, EmptyScorer, Explanation, Scorer, Weight};
use tantivy::schema::IndexRecordOption;
use tantivy::{DocId, DocSet, Score, SegmentReader};

pub struct ProximityWeight {
    query: ProximityQuery,
    weight_opt: Option<Bm25Weight>,
}

impl ProximityWeight {
    pub fn new(query: ProximityQuery, bm25_weight_opt: Option<Bm25Weight>) -> Self {
        Self {
            query,
            weight_opt: bm25_weight_opt,
        }
    }

    fn fieldnorm_reader(&self, reader: &SegmentReader) -> tantivy::Result<FieldNormReader> {
        let field = self.query.field();
        if self.weight_opt.is_some() {
            if let Some(fieldnorm_reader) = reader.fieldnorms_readers().get_field(field)? {
                return Ok(fieldnorm_reader);
            }
        }
        Ok(FieldNormReader::constant(reader.max_doc(), 1))
    }

    fn prox_scorer(
        &self,
        reader: &SegmentReader,
        boost: Score,
    ) -> tantivy::Result<Option<ProximityScorer>> {
        let weight_opt = self
            .weight_opt
            .as_ref()
            .map(|bm25_weight| bm25_weight.boost_by(boost));
        let fieldnorm_reader = self.fieldnorm_reader(reader)?;

        let mut left_postings = Vec::new();
        for term in self.query.left().terms() {
            if let Some(postings) = reader
                .inverted_index(term.field())?
                .read_postings(term, IndexRecordOption::WithFreqsAndPositions)?
            {
                left_postings.push(postings);
            } else {
                return Ok(None);
            }
        }

        let mut right_postings = Vec::new();
        for term in self.query.right().terms() {
            if let Some(postings) = reader
                .inverted_index(term.field())?
                .read_postings(term, IndexRecordOption::WithFreqsAndPositions)?
            {
                right_postings.push(postings);
            } else {
                return Ok(None);
            }
        }

        Ok(Some(ProximityScorer::new(
            left_postings,
            self.query.distance(),
            right_postings,
            fieldnorm_reader,
            weight_opt,
        )))
    }
}

impl Weight for ProximityWeight {
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        Ok(self
            .prox_scorer(reader, boost)?
            .map(|scorer| Box::new(scorer) as Box<dyn Scorer>)
            .unwrap_or_else(|| Box::new(EmptyScorer) as Box<dyn Scorer>))
    }

    fn explain(&self, reader: &SegmentReader, doc: DocId) -> tantivy::Result<Explanation> {
        let scorer_opt = self.prox_scorer(reader, 1.0)?;
        if scorer_opt.is_none() {
            return Err(does_not_match(doc));
        }
        let mut scorer = scorer_opt.unwrap();
        if scorer.seek(doc) != doc {
            return Err(does_not_match(doc));
        }
        let fieldnorm_reader = self.fieldnorm_reader(reader)?;
        let fieldnorm_id = fieldnorm_reader.fieldnorm_id(doc);
        let prox_count = scorer.prox_count();
        let mut explanation = Explanation::new("Proximity Scorer", scorer.score());
        if let Some(similarity_weight) = self.weight_opt.as_ref() {
            explanation.add_detail(similarity_weight.explain(fieldnorm_id, prox_count as u32));
        }
        Ok(explanation)
    }
}
