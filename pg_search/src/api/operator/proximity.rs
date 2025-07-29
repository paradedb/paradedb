use crate::query::proximity::{ProximityClause, ProximityDistance};
use pgrx::{opname, pg_operator};

#[pg_operator(immutable, parallel_safe)]
#[opname(pg_catalog.##)]
fn lhs_prox(left: ProximityClause, distance: i32) -> ProximityClause {
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
