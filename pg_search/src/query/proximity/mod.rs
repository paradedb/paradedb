use crate::api::Regex;
use pgrx::PostgresType;
use serde::{Deserialize, Serialize};

pub mod query;
mod scorer;
mod weight;

#[derive(Debug, Clone, Eq, PartialEq, PostgresType, Serialize, Deserialize)]
pub enum ProximityTermStyle {
    Term(String),
    Rexgex(Regex, usize),
}

impl ProximityTermStyle {
    pub fn as_str(&self) -> &str {
        match self {
            ProximityTermStyle::Term(term) => term.as_str(),
            ProximityTermStyle::Rexgex(regex, ..) => regex.as_str(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, PostgresType, Serialize, Deserialize)]
pub enum ProximityClause {
    Term(ProximityTermStyle),
    Clauses(Vec<ProximityClause>),
    Proximity {
        left: Box<ProximityClause>,
        distance: ProximityDistance,
        right: Box<ProximityClause>,
    },
}

#[derive(Copy, Clone)]
pub enum WhichTerms {
    Left,
    Right,
    All,
}

impl ProximityClause {
    pub fn is_empty(&self) -> bool {
        match self {
            ProximityClause::Term(_) => false,
            ProximityClause::Clauses(clauses) => {
                clauses.is_empty() || clauses.iter().all(|clause| clause.is_empty())
            }
            ProximityClause::Proximity { left, right, .. } => left.is_empty() || right.is_empty(),
        }
    }

    pub fn terms(&self, which_terms: WhichTerms) -> impl Iterator<Item = &ProximityTermStyle> {
        let iter: Box<dyn Iterator<Item = &ProximityTermStyle>> = match self {
            ProximityClause::Term(term) => Box::new(std::iter::once(term)),
            ProximityClause::Clauses(clauses) => Box::new(
                clauses
                    .iter()
                    .flat_map(move |clause| clause.terms(which_terms)),
            ),
            ProximityClause::Proximity { left, right, .. } => match which_terms {
                WhichTerms::Left => Box::new(left.terms(which_terms)),
                WhichTerms::Right => Box::new(right.terms(which_terms)),
                WhichTerms::All => {
                    Box::new(left.terms(which_terms).chain(right.terms(which_terms)))
                }
            },
        };
        iter
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
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
