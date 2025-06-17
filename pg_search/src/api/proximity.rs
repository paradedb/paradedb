use crate::api::FieldName;
use crate::api::Regex;
use crate::query::proximity::{ProximityClause, ProximityDistance, ProximityTermStyle};
use crate::query::SearchQueryInput;
use pgrx::{default, pg_cast, pg_extern, VariadicArray};

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
    Ok(ProximityClause::Term(Regex::new(&regex).map(|re| {
        ProximityTermStyle::Rexgex(re, max_expansions)
    })?))
}

#[pg_extern(immutable, parallel_safe)]
pub fn prox_array(clauses: VariadicArray<ProximityClause>) -> ProximityClause {
    ProximityClause::Clauses(clauses.into_iter().filter_map(|clause| clause).collect())
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

#[pg_extern(immutable, parallel_safe)]
pub fn proximity(
    field: FieldName,
    left: ProximityClause,
    distance: i32,
    right: ProximityClause,
) -> anyhow::Result<SearchQueryInput> {
    let distance: u32 = distance.try_into()?;
    Ok(SearchQueryInput::Proximity {
        field,
        left,
        distance: ProximityDistance::AnyOrder(distance),
        right,
    })
}

#[pg_extern(immutable, parallel_safe)]
pub fn proximity_in_order(
    field: FieldName,
    left: ProximityClause,
    distance: i32,
    right: ProximityClause,
) -> anyhow::Result<SearchQueryInput> {
    let distance: u32 = distance.try_into()?;
    Ok(SearchQueryInput::Proximity {
        field,
        left,
        distance: ProximityDistance::InOrder(distance),
        right,
    })
}

#[pg_cast(implicit, immutable, parallel_safe)]
pub fn text_to_prox_clause(t: String) -> ProximityClause {
    ProximityClause::Term(ProximityTermStyle::Term(t))
}

#[pg_cast(implicit, immutable, parallel_safe)]
pub fn text_array_to_prox_clause(t: Vec<String>) -> ProximityClause {
    ProximityClause::Clauses(t.into_iter().map(text_to_prox_clause).collect())
}
