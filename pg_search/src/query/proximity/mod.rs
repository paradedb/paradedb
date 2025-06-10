use crate::query::proximity::query::ProximityQuery;
use tantivy::schema::Field;
use tantivy::Term;

pub mod query;
mod scorer;
mod weight;

#[derive(Debug, Clone)]
pub enum ProximityClauseStyle {
    Terms { terms: Vec<Term> },
    Proximity { prox: ProximityQuery },
}

impl ProximityClauseStyle {
    pub fn field(&self) -> Field {
        match self {
            ProximityClauseStyle::Terms { terms } => terms[0].field(),
            ProximityClauseStyle::Proximity { prox } => prox.field(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProximityClause {
    style: ProximityClauseStyle,
}

impl ProximityClause {
    pub fn new(style: ProximityClauseStyle) -> Self {
        Self { style }
    }
    pub fn field(&self) -> Field {
        self.style.field()
    }
    pub fn terms(&self) -> impl Iterator<Item = &Term> {
        let iter: Box<dyn Iterator<Item = &Term>> = match &self.style {
            ProximityClauseStyle::Terms { terms } => Box::new(terms.iter()),
            ProximityClauseStyle::Proximity { prox } => {
                Box::new(prox.left().terms().chain(prox.right().terms()))
            }
        };
        iter
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ProximityDistance {
    InOrder(u32),
    AnyOrder(u32),
}

impl ProximityDistance {
    pub fn distance(&self) -> u32 {
        match self {
            ProximityDistance::InOrder(distance) => *distance,
            ProximityDistance::AnyOrder(distance) => *distance,
        }
    }

    pub fn in_order(&self) -> bool {
        matches!(self, ProximityDistance::InOrder(_))
    }
}
