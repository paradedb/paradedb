// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::query::proximity::ProximityClause;
use pgrx::pg_cast;

pub use pdb::*;

#[pgrx::pg_schema]
pub mod pdb {
    use crate::api::Regex;
    use crate::query::pdb_query::pdb;
    use crate::query::proximity::{ProximityClause, ProximityDistance};
    use macros::builder_fn;
    use pgrx::{default, pg_extern, VariadicArray};

    #[pg_extern(immutable, parallel_safe)]
    pub fn prox_term(term: String) -> ProximityClause {
        ProximityClause::Term(term)
    }

    #[pg_extern(immutable, parallel_safe)]
    pub fn prox_regex(
        regex: String,
        max_expansions: default!(i32, 50),
    ) -> anyhow::Result<ProximityClause> {
        let max_expansions: usize = max_expansions.try_into()?;
        Ok(ProximityClause::Regex {
            pattern: Regex::new(&regex)?,
            max_expansions,
        })
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
    #[pg_extern(immutable, parallel_safe, name = "proximity")]
    pub fn proximity(prox: crate::query::proximity::ProximityClause) -> pdb::Query {
        match prox {
            crate::query::proximity::ProximityClause::Proximity {
                left,
                distance,
                right,
            } => pdb::Query::Proximity {
                left: *left,
                distance,
                right: *right,
            },
            _ => panic!("prox must be a complete ProximityClause"),
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "proximity")]
    pub fn proximity_pair(
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
    #[pg_extern(immutable, parallel_safe, name = "proximity_in_order")]
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
    ProximityClause::Term(t)
}

#[pg_cast(implicit, immutable, parallel_safe)]
pub fn text_array_to_prox_clause(t: Vec<String>) -> ProximityClause {
    ProximityClause::Clauses(t.into_iter().map(text_to_prox_clause).collect())
}
