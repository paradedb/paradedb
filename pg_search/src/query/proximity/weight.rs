use crate::query::proximity::query::ProximityQuery;
use crate::query::proximity::scorer::ProximityScorer;
use crate::query::proximity::{ProximityClause, ProximityTermStyle, WhichTerms};
use std::sync::Arc;
use tantivy::fieldnorm::FieldNormReader;
use tantivy::postings::{LoadedPostings, Postings};
use tantivy::query::{
    does_not_match, AutomatonWeight, Bm25Weight, EmptyScorer, Explanation, RegexPhraseWeight,
    Scorer, Weight,
};
use tantivy::schema::IndexRecordOption;
use tantivy::{DocId, DocSet, Score, SegmentReader, Term, TERMINATED};

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

        let left_postings =
            self.read_postings(reader, self.query.left(), WhichTerms::Left, false)?;
        if left_postings.is_empty() {
            return Ok(None);
        }
        let right_postings =
            self.read_postings(reader, self.query.right(), WhichTerms::Right, false)?;
        if right_postings.is_empty() {
            return Ok(None);
        }

        Ok(Some(ProximityScorer::new(
            left_postings,
            self.query.distance(),
            right_postings,
            fieldnorm_reader,
            weight_opt,
        )))
    }

    fn read_postings(
        &self,
        segment_reader: &SegmentReader,
        clause: &ProximityClause,
        which_terms: WhichTerms,
        nested: bool,
    ) -> tantivy::Result<Vec<Box<dyn Postings>>> {
        if let ProximityClause::Proximity {
            left,
            distance,
            right,
        } = clause
        {
            let query =
                ProximityQuery::new(self.query.field(), *left.clone(), *distance, *right.clone());
            let weight = ProximityWeight::new(query, self.weight_opt.clone());
            let left_postings = weight.read_postings(segment_reader, left, which_terms, true)?;
            let right_postings = weight.read_postings(segment_reader, right, which_terms, true)?;

            let mut scorer = ProximityScorer::new(
                left_postings,
                *distance,
                right_postings,
                self.fieldnorm_reader(segment_reader)?,
                self.weight_opt.clone(),
            );

            let mut doc_ids = Vec::new();
            let mut positions = Vec::new();
            let mut offsets = Vec::new();
            while scorer.doc() != TERMINATED {
                offsets.push(positions.len() as u32);
                doc_ids.push(scorer.doc());

                for (l, r) in scorer.prox_iter() {
                    if nested {
                        match which_terms {
                            // *** NOTICE! ***
                            //
                            // these are *purposely* reversed.  If we're nested and the user is asking
                            // for postings on the left-side of a proximity query what *we* need to
                            // return are the postings from the right side
                            WhichTerms::Left => positions.push(r),

                            // same goes for the right-side, just in reverse
                            WhichTerms::Right => positions.push(l),

                            // gotta collect 'em all
                            WhichTerms::All => {
                                positions.push(l);
                                positions.push(r);
                            }
                        }
                    } else {
                        positions.push(l);
                        positions.push(r);
                    }
                }
                scorer.advance();
            }
            offsets.push(positions.len() as u32);

            let loaded_postings = LoadedPostings {
                doc_ids: doc_ids.into_boxed_slice(),
                position_offsets: offsets.into_boxed_slice(),
                positions: positions.into_boxed_slice(),
                cursor: 0,
            };

            Ok(vec![Box::new(loaded_postings)])
        } else {
            let mut postings: Vec<Box<dyn Postings>> = Vec::new();
            let mut num_regex_terms = 0;
            let inverted_index = segment_reader.inverted_index(self.query.field())?;
            for term in clause.terms(self.query.field(), Some(segment_reader), which_terms)? {
                match term.as_ref() {
                    ProximityTermStyle::Term(term) => {
                        let term = Term::from_field_text(self.query.field(), term);
                        if let Some(segment_postings) = inverted_index
                            .read_postings(&term, IndexRecordOption::WithFreqsAndPositions)?
                        {
                            postings.push(Box::new(segment_postings));
                        }
                    }
                    ProximityTermStyle::Rexgex(re, max_expansions) => {
                        let regex =
                            tantivy_fst::Regex::new(re.as_str()).unwrap_or_else(|e| panic!("{e}"));
                        let automaton = AutomatonWeight::<tantivy_fst::Regex>::new(
                            self.query.field(),
                            Arc::new(regex),
                        );
                        let term_infos = automaton.get_match_term_infos(segment_reader)?;
                        if term_infos.is_empty() {
                            // if term_infos is empty, that's fine -- we might have other terms
                            continue;
                        }
                        num_regex_terms += term_infos.len();
                        if num_regex_terms > *max_expansions {
                            // we have more regex matches than our max_expansions -- stop matching now
                            continue;
                            // return Err(TantivyError::InvalidArgument(format!(
                            //     "Regex ProximityClause(s) exceeded max expansions: {num_regex_terms} > {max_expansions}",
                            // )));
                        }
                        let union = RegexPhraseWeight::get_union_from_term_infos(
                            &term_infos,
                            segment_reader,
                            &inverted_index,
                        )?;
                        postings.push(Box::new(union))
                    }
                }
            }
            Ok(postings)
        }
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
