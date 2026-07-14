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

//! Deterministic-simulation-testing (DST) hooks.
//!
//! The only module that talks to the DST vendor SDK (Antithesis), pulled in as
//! `antithesis_sdk`.
//!
//! The assertion wrappers are forwarding macros, not functions, so the SDK captures each
//! assertion's location at the real call site rather than here in `dst.rs`. [`init`] carries
//! no location, so it stays a plain function.
//!
//! Stressgres links the SDK unconditionally: it is test-only tooling, and the SDK is a
//! runtime no-op outside the DST environment, so there is no feature gate here (unlike
//! `pg_search`, which gates its equivalent module behind `--features dst`).

/// Register the assertion catalog so the harness knows which sites exist but were never
/// hit, instead of a never-reached assertion passing vacuously. Call once at startup.
pub fn init() {
    antithesis_sdk::antithesis_init();
}

/// Reachability: mark that the workload retried and rode out a transient database fault.
/// A run where this is never hit exercised no fault and proves nothing about recovery.
macro_rules! retried_transient_fault {
    () => {
        ::antithesis_sdk::assert_reachable!("stressgres retried a transient database fault")
    };
}
pub(crate) use retried_transient_fault;

/// Sometimes: `$narrowed` is true when a recovery poke shrank the grace window during an
/// active fault — the only situation in which the liveness check can actually fail, so it
/// must hold true at least once across the fault search or the check proved nothing.
macro_rules! poke_narrowed_window {
    ($narrowed:expr) => {
        ::antithesis_sdk::assert_sometimes!(
            $narrowed,
            "a recovery poke narrowed the grace window during an active database fault"
        )
    };
}
pub(crate) use poke_narrowed_window;
