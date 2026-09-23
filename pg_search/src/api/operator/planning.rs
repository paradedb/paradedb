// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::index::reader::index::DocsEstimate;
use crate::query::SearchQueryInput;
use pgrx::pg_sys::Oid;
use std::cell::RefCell;

#[derive(Default)]
struct PlanningEstimate {
    last: Option<(Oid, SearchQueryInput, DocsEstimate)>,
}

// PostgreSQL 19 planner extension-private state can replace this thread-local.
thread_local! {
    static ACTIVE: RefCell<Option<PlanningEstimate>> = const { RefCell::new(None) };
}

/// Reuse the most recent estimate only within this planner invocation.
pub(crate) struct EstimateScope {
    previous: Option<PlanningEstimate>,
}

impl EstimateScope {
    pub(crate) fn enter() -> Self {
        Self {
            previous: ACTIVE.replace(Some(PlanningEstimate::default())),
        }
    }
}

impl Drop for EstimateScope {
    fn drop(&mut self) {
        ACTIVE.set(self.previous.take());
    }
}

pub(crate) fn segment_count(index: Oid) -> Option<usize> {
    ACTIVE.with_borrow(|state| {
        let (source, _, estimate) = state.as_ref()?.last.as_ref()?;
        (*source == index).then_some(estimate.total_segments)
    })
}

pub(super) fn estimate(
    index: Oid,
    query: SearchQueryInput,
    compute: impl FnOnce(SearchQueryInput) -> Option<DocsEstimate>,
) -> Option<DocsEstimate> {
    let (active, previous) = ACTIVE.with_borrow(|state| {
        let previous = state.as_ref().and_then(|state| {
            state.last.as_ref().and_then(|(source, input, estimate)| {
                (*source == index && *input == query).then_some(*estimate)
            })
        });
        (state.is_some(), previous)
    });
    if previous.is_some() {
        return previous;
    }
    let key = active.then(|| query.clone());
    let result = compute(query)?;
    if let Some(key) = key {
        ACTIVE.with_borrow_mut(|state| {
            if let Some(state) = state {
                state.last = Some((index, key, result));
            }
        });
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn result(value: u64) -> DocsEstimate {
        DocsEstimate {
            matching_docs: value as usize,
            total_docs: 100,
            query_cost: value,
            total_segments: value as usize,
        }
    }

    #[test]
    fn reuse_requires_an_active_scope_and_identical_input() {
        let calls = Cell::new(0);
        let compute = |_| {
            calls.set(calls.get() + 1);
            Some(result(calls.get()))
        };
        let index = Oid::from(42u32);
        assert_eq!(segment_count(index), None);
        estimate(index, SearchQueryInput::All, compute);
        estimate(index, SearchQueryInput::All, compute);
        assert_eq!(calls.get(), 2);
        {
            let _scope = EstimateScope::enter();
            estimate(index, SearchQueryInput::All, compute);
            estimate(index, SearchQueryInput::All, compute);
            assert_eq!(calls.get(), 3);
            assert_eq!(segment_count(index), Some(3));
            assert_eq!(segment_count(Oid::from(43u32)), None);
            estimate(index, SearchQueryInput::Empty, compute);
            estimate(Oid::from(43u32), SearchQueryInput::Empty, compute);
            assert_eq!(calls.get(), 5);
        }
        estimate(index, SearchQueryInput::All, compute);
        assert_eq!(calls.get(), 6);
        assert_eq!(segment_count(index), None);
    }

    #[test]
    fn nested_planning_restores_the_outer_estimate() {
        let index = Oid::from(42u32);
        let _outer = EstimateScope::enter();
        estimate(index, SearchQueryInput::All, |_| Some(result(10)));
        {
            let _inner = EstimateScope::enter();
            assert_eq!(
                estimate(index, SearchQueryInput::All, |_| Some(result(20)))
                    .unwrap()
                    .query_cost,
                20
            );
            assert_eq!(segment_count(index), Some(20));
        }
        assert_eq!(
            estimate(index, SearchQueryInput::All, |_| panic!(
                "unexpected recomputation"
            ))
            .unwrap()
            .query_cost,
            10
        );
        assert_eq!(segment_count(index), Some(10));
    }

    #[test]
    fn failure_is_retryable_and_unwind_drops_the_scope() {
        let index = Oid::from(42u32);
        let panic = std::panic::catch_unwind(|| {
            let _scope = EstimateScope::enter();
            assert!(estimate(index, SearchQueryInput::All, |_| None).is_none());
            assert!(estimate(index, SearchQueryInput::All, |_| Some(result(10))).is_some());
            panic!("planning failed");
        });
        assert!(panic.is_err());
        assert_eq!(
            estimate(index, SearchQueryInput::All, |_| Some(result(20)))
                .unwrap()
                .query_cost,
            20
        );
    }
}
