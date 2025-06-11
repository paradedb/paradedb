use crate::query::proximity::weight::ProximityWeight;
use crate::query::proximity::{ProximityClause, ProximityDistance, WhichTerms};
use tantivy::query::{Bm25Weight, EnableScoring, Query, Weight};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::{TantivyError, Term};

#[derive(Debug, Clone)]
pub struct ProximityQuery {
    field: Field,
    left: ProximityClause,
    distance: ProximityDistance,
    right: ProximityClause,
}

impl ProximityQuery {
    pub fn new(
        field: Field,
        left: ProximityClause,
        distance: ProximityDistance,
        right: ProximityClause,
    ) -> Self {
        Self {
            field,
            left,
            distance,
            right,
        }
    }

    pub fn field(&self) -> Field {
        self.field
    }

    pub fn left(&self) -> &ProximityClause {
        &self.left
    }

    pub fn right(&self) -> &ProximityClause {
        &self.right
    }

    pub fn terms(&self) -> impl Iterator<Item = Term> + '_ {
        self.left
            .terms(WhichTerms::All)
            .chain(self.right.terms(WhichTerms::All))
            .map(|t| Term::from_field_text(self.field, t.as_str()))
    }

    pub fn distance(&self) -> ProximityDistance {
        self.distance
    }
}

impl Query for ProximityQuery {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        let schema = enable_scoring.schema();
        let field_entry = schema.get_field_entry(self.field);
        let has_positions = field_entry
            .field_type()
            .get_index_record_option()
            .map(IndexRecordOption::has_positions)
            .unwrap_or(false);
        if !has_positions {
            let field_name = field_entry.name();
            return Err(TantivyError::SchemaError(format!(
                "proximity queries require fields indexed with positions.  `{field_name:?}` does not have positions."
            )));
        }

        let terms = self.terms().collect::<Vec<_>>();
        let bm25_weight_opt = match enable_scoring {
            EnableScoring::Enabled {
                statistics_provider,
                ..
            } => Some(Bm25Weight::for_terms(statistics_provider, &terms)?),
            EnableScoring::Disabled { .. } => None,
        };

        let weight = ProximityWeight::new(self.clone(), bm25_weight_opt);
        Ok(Box::new(weight))
    }

    fn query_terms<'a>(&'a self, visitor: &mut dyn FnMut(&'a Term, bool)) {
        // TODO:  figure out how to do this one
        // for term in self.terms() {
        //     visitor(term, true)
        // }
    }
}
