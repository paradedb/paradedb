// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Conservative query proofs over one execution-visible segment view.
//!
//! An immutable segment's statistics are fixed and valid for that segment. What can change between
//! planning and execution is the visible segment set: inserts add segments and merges replace
//! them. The proof table is therefore rebuilt against the exact execution Searcher instead of
//! carrying planner-visible segment identities forward.
//!
//! Statistics are accelerators, never substitutes for the query predicate. Missing, unreadable,
//! unsupported, or incomparable data is an explicit [`predicate::SegmentTruth::Maybe`], never a
//! default.

pub(crate) mod predicate;
mod snapshot;

pub(crate) use snapshot::SegmentStatsSnapshot;
#[cfg(any(test, feature = "pg_test"))]
pub(crate) use snapshot::test_support::{
    EMPIRICAL_READS, FIELD_PROOF_PASSES, InjectedStatsFailure, STATS_OPENS, inject_stats_failure,
};
