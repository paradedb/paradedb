use crate::query::proximity::{ProximityClause, ProximityTermStyle};
use pgrx::pg_cast;

pub use pdb::*;

#[pgrx::pg_schema]
pub mod pdb {
    use crate::api::Regex;
    use crate::query::pdb_query::pdb;
    use crate::query::proximity::{ProximityClause, ProximityDistance, ProximityTermStyle};
    use macros::builder_fn;
    use pgrx::{default, pg_extern, VariadicArray};

    #[pg_extern(immutable, parallel_safe)]
    pub fn prox_term(term: String) -> ProximityClause {
        ProximityClause::Term(ProximityTermStyle::Term(term))
    }

    #[pg_extern(immutable, parallel_safe)]
    pub fn prox_regex(
        regex: String,
        max_expansions: default!(i32, 50),
    ) -> anyhow::Result<ProximityClause> {
        let max_expansions: usize = max_expansions.try_into()?;
        Ok(ProximityClause::Term(
            Regex::new(&regex).map(|re| ProximityTermStyle::Regex(re, max_expansions))?,
        ))
    }

    #[pg_extern(immutable, parallel_safe)]
    pub fn prox_array(clauses: VariadicArray<ProximityClause>) -> ProximityClause {
        ProximityClause::Clauses(clauses.into_iter().flatten().collect())
    }

    #[pg_extern(immutable, parallel_safe)]
    pub fn prox_clause(
        left: ProximityClause,
        distance: i32,
        right: ProximityClause,
    ) -> anyhow::Result<ProximityClause> {
        Ok(ProximityClause::Proximity {
            left: Box::new(left),
            distance: ProximityDistance::AnyOrder(distance.try_into()?),
            right: Box::new(right),
        })
    }

    #[pg_extern(immutable, parallel_safe)]
    pub fn prox_clause_in_order(
        left: ProximityClause,
        distance: i32,
        right: ProximityClause,
    ) -> anyhow::Result<ProximityClause> {
        Ok(ProximityClause::Proximity {
            left: Box::new(left),
            distance: ProximityDistance::InOrder(distance.try_into()?),
            right: Box::new(right),
        })
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe)]
    pub fn proximity(
        left: crate::query::proximity::ProximityClause,
        distance: i32,
        right: crate::query::proximity::ProximityClause,
    ) -> pdb::Query {
        let distance: u32 = distance
            .try_into()
            .expect("distance should not be out of bounds `[0..]`");
        pdb::Query::Proximity {
            left,
            distance: ProximityDistance::AnyOrder(distance),
            right,
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe)]
    pub fn proximity_in_order(
        left: crate::query::proximity::ProximityClause,
        distance: i32,
        right: crate::query::proximity::ProximityClause,
    ) -> pdb::Query {
        let distance: u32 = distance
            .try_into()
            .expect("distance should not be out of bounds `[0..]`");
        pdb::Query::Proximity {
            left,
            distance: ProximityDistance::InOrder(distance),
            right,
        }
    }
}

#[pg_cast(implicit, immutable, parallel_safe)]
pub fn text_to_prox_clause(t: String) -> ProximityClause {
    ProximityClause::Term(ProximityTermStyle::Term(t))
}

#[pg_cast(implicit, immutable, parallel_safe)]
pub fn text_array_to_prox_clause(t: Vec<String>) -> ProximityClause {
    ProximityClause::Clauses(t.into_iter().map(text_to_prox_clause).collect())
}
