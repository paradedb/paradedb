use crate::query::proximity::weight::ProximityWeight;
use crate::query::proximity::{ProximityClause, ProximityDistance};
use tantivy::query::{Bm25Weight, EnableScoring, Query, Weight};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::{TantivyError, Term};

#[derive(Debug, Clone)]
pub struct ProximityQuery {
    left: Box<ProximityClause>,
    distance: ProximityDistance,
    right: Box<ProximityClause>,
}

impl ProximityQuery {
    pub fn new(left: ProximityClause, distance: ProximityDistance, right: ProximityClause) -> Self {
        assert!(
            left.field() == right.field(),
            "ProximityClauses must all use the same field"
        );
        Self {
            left: Box::new(left),
            distance,
            right: Box::new(right),
        }
    }

    pub fn field(&self) -> Field {
        self.left.field()
    }

    pub fn left(&self) -> &ProximityClause {
        &self.left
    }

    pub fn right(&self) -> &ProximityClause {
        &self.right
    }

    pub fn terms(&self) -> impl Iterator<Item = &Term> {
        self.left.terms().chain(self.right.terms())
    }

    pub fn distance(&self) -> ProximityDistance {
        self.distance
    }
}

impl Query for ProximityQuery {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        let schema = enable_scoring.schema();
        let field_entry = schema.get_field_entry(self.field());
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

        let terms = self.terms().map(|term| term.clone()).collect::<Vec<_>>();
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
        for term in self.terms() {
            visitor(term, true)
        }
    }
}
