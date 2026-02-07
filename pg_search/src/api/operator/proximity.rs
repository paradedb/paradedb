// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use crate::query::proximity::{ProximityClause, ProximityDistance};
use pgrx::{opname, pg_operator};

#[pg_operator(immutable, parallel_safe)]
#[opname(pg_catalog.##)]
fn lhs_prox(left: ProximityClause, distance: i32) -> ProximityClause {
    ProximityClause::Proximity {
        left: Box::new(left),
        distance: ProximityDistance::AnyOrder(
            distance
                .try_into()
                .expect("distance should not be out of bounds `[0..]`"),
        ),
        right: Box::new(ProximityClause::Uninitialized),
    }
}

#[pg_operator(immutable, parallel_safe)]
#[opname(pg_catalog.##)]
fn rhs_prox(left: ProximityClause, right: ProximityClause) -> ProximityClause {
    match left {
        ProximityClause::Proximity {
            left,
            distance,
            right: original_right,
        } if matches!(original_right.as_ref(), ProximityClause::Uninitialized) => {
            ProximityClause::Proximity {
                left,
                distance,
                right: Box::new(right),
            }
        }
        _ => panic!("lhs of ## must be a ProximityClause with an uninitialized rhs"),
    }
}

#[pg_operator(immutable, parallel_safe)]
#[opname(pg_catalog.##>)]
fn lhs_prox_in_order(left: ProximityClause, distance: i32) -> ProximityClause {
    ProximityClause::Proximity {
        left: Box::new(left),
        distance: ProximityDistance::InOrder(
            distance
                .try_into()
                .expect("distance should not be out of bounds `[0..]`"),
        ),
        right: Box::new(ProximityClause::Uninitialized),
    }
}

#[pg_operator(immutable, parallel_safe)]
#[opname(pg_catalog.##>)]
fn rhs_prox_in_order(left: ProximityClause, right: ProximityClause) -> ProximityClause {
    match left {
        ProximityClause::Proximity {
            left,
            distance,
            right: original_right,
        } if matches!(original_right.as_ref(), ProximityClause::Uninitialized) => {
            ProximityClause::Proximity {
                left,
                distance,
                right: Box::new(right),
            }
        }
        _ => panic!("lhs of ## must be a ProximityClause with an uninitialized rhs"),
    }
}
