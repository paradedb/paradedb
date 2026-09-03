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

//! Deterministic-simulation-testing (DST) hooks — the one place that talks to the Antithesis
//! Rust SDK.
//!
//! Three build configurations, selected automatically:
//!
//! * **`enabled`** — macros forward to `antithesis_sdk`; used for Antithesis runs
//!   (`pg_search --features dst`).
//! * **`enabled` off, `debug_assertions` on** — the always/unreachable asserts lower to
//!   `debug_assert!` and `GhostState`/`observe!` run, so invariants are also checked in normal
//!   testing; `assert_sometimes!`/`assert_reachable!` need a whole run, so they only type-check.
//! * **neither** (release) — compiles to nothing.
//!
//! The assertion wrappers mirror the SDK's signatures (`$message` must be a string literal) and
//! are macros, not functions, so each assertion's location is captured at the real call site.
//!
//! `observe!` and `GhostState` are ported from `precept` (orbitinghail/precept): `observe!` runs
//! a read-only property block whose `Fn` bound makes the compiler reject any code that mutates
//! the observed system, and `GhostState<T>` is auxiliary property-only state erased from
//! non-instrumented builds. Here they sit on top of the official SDK rather than precept.

// Re-export the SDK so the `#[macro_export]` wrappers below can reach it from a consumer crate
// that does not depend on `antithesis_sdk` directly.
#[cfg(feature = "enabled")]
#[doc(hidden)]
pub use antithesis_sdk;

// `as _` keeps the linker from dropping the coverage shim.
#[cfg(feature = "enabled")]
use antithesis_instrumentation as _;

/// Register the Antithesis assertion catalog for this process. Required once per process that
/// emits assertions; without it a never-hit `assert_unreachable!` would pass vacuously instead
/// of being reported. `antithesis_init` is idempotent, so it is safe to call from every process
/// / forked worker. A no-op unless `enabled`.
#[cfg(feature = "enabled")]
pub fn init() {
    antithesis_sdk::antithesis_init();
}

/// See the [`enabled` definition](init).
#[cfg(not(feature = "enabled"))]
pub fn init() {}

/// Whether a Postgres SQLSTATE means the connection is gone and reconnecting is the right
/// response: class 08 (connection exception), operator/crash shutdown, "cannot connect now"
/// (server starting up or shutting down), and idle-session timeout. Shared by the stressgres and
/// qgen workloads so their transient-fault classifications cannot drift. (stressgres additionally
/// keeps a stringified-needle fallback in `fault_tolerance::TRANSIENT_ERROR_NEEDLES`, which must
/// mirror this list.)
pub fn is_connection_lost_sqlstate(code: &str) -> bool {
    code.starts_with("08") || matches!(code, "57P01" | "57P02" | "57P03" | "57P05")
}

#[cfg(test)]
mod sqlstate_tests {
    use super::is_connection_lost_sqlstate;

    #[test]
    fn connection_class_and_shutdown_codes_are_lost() {
        for code in [
            "08000", "08006", "08P01", "57P01", "57P02", "57P03", "57P05",
        ] {
            assert!(is_connection_lost_sqlstate(code), "{code}");
        }
    }

    #[test]
    fn query_and_data_errors_are_not_lost() {
        for code in ["57014", "40001", "40P01", "23505", "42601", ""] {
            assert!(!is_connection_lost_sqlstate(code), "{code}");
        }
    }
}

// The leading `debug_assert` / `type_only` keyword selects the SDK-off lowering (see crate docs).
// `$d` is bound to `$` so a generated macro can name its own metavariables (the escape for a macro
// that defines a macro).

// Type-check-only expansion for a compiled-out assertion: evaluates nothing.
#[doc(hidden)]
#[macro_export]
macro_rules! __dst_typecheck_cond {
    ($condition:expr, $message:literal $(, $details:expr)?) => {
        if false {
            let _: bool = $condition;
            let _: &str = $message;
            $(let _ = &$details;)?
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __dst_typecheck_msg {
    ($message:literal $(, $details:expr)?) => {
        if false {
            let _: &str = $message;
            $(let _ = &$details;)?
        }
    };
}

/// Generate a condition-style wrapper: `name!(condition, "message" [, &details])`.
// rustfmt cannot format a `macro_rules!` that defines a `macro_rules!` idempotently — each pass
// re-indents the nested arms further — so pin this generator's formatting by hand.
#[rustfmt::skip]
macro_rules! define_condition_assert {
    (debug_assert $d:tt $name:ident, $doc:literal) => {
        #[doc = $doc]
        #[cfg(feature = "enabled")]
        #[macro_export]
        macro_rules! $name {
            ($d condition:expr, $d message:literal $d(, $d details:expr)?) => {
                $crate::antithesis_sdk::$name!($d condition, $d message $d(, $d details)?)
            };
        }

        // `details` folds into the panic message.
        #[doc = $doc]
        #[cfg(all(not(feature = "enabled"), debug_assertions))]
        #[macro_export]
        macro_rules! $name {
            ($d condition:expr, $d message:literal, $d details:expr) => {
                ::core::debug_assert!($d condition, "{}: {}", $d message, $d details)
            };
            ($d condition:expr, $d message:literal) => {
                ::core::debug_assert!($d condition, "{}", $d message)
            };
        }

        #[doc = $doc]
        #[cfg(all(not(feature = "enabled"), not(debug_assertions)))]
        #[macro_export]
        macro_rules! $name {
            ($d condition:expr, $d message:literal $d(, $d details:expr)?) => {
                $crate::__dst_typecheck_cond!($d condition, $d message $d(, $d details)?)
            };
        }
    };

    (type_only $d:tt $name:ident, $doc:literal) => {
        #[doc = $doc]
        #[cfg(feature = "enabled")]
        #[macro_export]
        macro_rules! $name {
            ($d condition:expr, $d message:literal $d(, $d details:expr)?) => {
                $crate::antithesis_sdk::$name!($d condition, $d message $d(, $d details)?)
            };
        }

        #[doc = $doc]
        #[cfg(not(feature = "enabled"))]
        #[macro_export]
        macro_rules! $name {
            ($d condition:expr, $d message:literal $d(, $d details:expr)?) => {
                $crate::__dst_typecheck_cond!($d condition, $d message $d(, $d details)?)
            };
        }
    };
}

/// Generate a message-only wrapper: `name!("message" [, &details])`.
#[rustfmt::skip]
macro_rules! define_message_assert {
    (debug_assert $d:tt $name:ident, $doc:literal) => {
        #[doc = $doc]
        #[cfg(feature = "enabled")]
        #[macro_export]
        macro_rules! $name {
            ($d message:literal $d(, $d details:expr)?) => {
                $crate::antithesis_sdk::$name!($d message $d(, $d details)?)
            };
        }

        // Reaching this site is the violation, so panic.
        #[doc = $doc]
        #[cfg(all(not(feature = "enabled"), debug_assertions))]
        #[macro_export]
        macro_rules! $name {
            ($d message:literal, $d details:expr) => {
                ::core::debug_assert!(false, "{}: {}", $d message, $d details)
            };
            ($d message:literal) => {
                ::core::debug_assert!(false, "{}", $d message)
            };
        }

        #[doc = $doc]
        #[cfg(all(not(feature = "enabled"), not(debug_assertions)))]
        #[macro_export]
        macro_rules! $name {
            ($d message:literal $d(, $d details:expr)?) => {
                $crate::__dst_typecheck_msg!($d message $d(, $d details)?)
            };
        }
    };

    (type_only $d:tt $name:ident, $doc:literal) => {
        #[doc = $doc]
        #[cfg(feature = "enabled")]
        #[macro_export]
        macro_rules! $name {
            ($d message:literal $d(, $d details:expr)?) => {
                $crate::antithesis_sdk::$name!($d message $d(, $d details)?)
            };
        }

        #[doc = $doc]
        #[cfg(not(feature = "enabled"))]
        #[macro_export]
        macro_rules! $name {
            ($d message:literal $d(, $d details:expr)?) => {
                $crate::__dst_typecheck_msg!($d message $d(, $d details)?)
            };
        }
    };
}

define_condition_assert!(debug_assert $ assert_always,
    "Assert `condition` holds every time this site runs and that it is hit at least once. `message` must be a string literal; optional `details` is a `&serde_json::Value`.");
define_condition_assert!(debug_assert $ assert_always_or_unreachable,
    "Like `assert_always!`, but the property still passes if the site is never hit.");
define_condition_assert!(type_only $ assert_sometimes,
    "Assert `condition` holds at least once across the run.");
define_message_assert!(type_only $ assert_reachable,
    "Assert this site is reached at least once across the run.");
define_message_assert!(debug_assert $ assert_unreachable,
    "Assert this site is never reached; reaching it reports a violation.");

// Ghost state + read-only observation, ported from precept (orbitinghail/precept).

/// Auxiliary *ghost state* for expressing properties: read only via [`observe!`], mutated only via
/// [`GhostState::mutate`]. Value-carrying where assertions can fire (`enabled` or
/// `debug_assertions`); a zero-sized no-op otherwise.
#[cfg(any(feature = "enabled", debug_assertions))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GhostState<T>(T);

/// See the [value-carrying definition](GhostState).
#[cfg(not(any(feature = "enabled", debug_assertions)))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GhostState<T>(::core::marker::PhantomData<T>);

#[cfg(any(feature = "enabled", debug_assertions))]
impl<T> GhostState<T> {
    /// Create ghost state, initializing the inner `T` with `init`. `init` is read-only (an `Fn`);
    /// when the crate is compiled out it is never called and no `T` is constructed.
    pub fn new<F: Fn() -> T>(init: F) -> Self {
        GhostState(init())
    }

    /// The sole mutator. `f` gets `&mut T`; everything it captures is read-only (`Fn`). When the
    /// crate is compiled out, `f` is type-checked but never called.
    pub fn mutate<F: Fn(&mut T)>(&mut self, f: F) {
        f(&mut self.0)
    }

    // Private — the only readers are the crate-internal `__observeN` helpers, so there is no
    // public way to obtain a `&T`: ghost state can only be read from inside an `observe!` block.
    fn inner(&self) -> &T {
        &self.0
    }
}

#[cfg(not(any(feature = "enabled", debug_assertions)))]
impl<T> GhostState<T> {
    /// See the value-carrying definition.
    #[allow(unused_variables)]
    pub fn new<F: Fn() -> T>(init: F) -> Self {
        GhostState(::core::marker::PhantomData)
    }

    /// See the value-carrying definition.
    #[allow(unused_variables)]
    pub fn mutate<F: Fn(&mut T)>(&mut self, f: F) {}
}

// The per-arity `__observeN` helpers that back `observe!`. Each carries the `Fn(&T0, ..)` bound
// that enforces read-only access. When `enabled` is off the body is empty, so the closure is
// type-checked but never executed.
macro_rules! define_observe_helpers {
    ($( $name:ident ( $($ty:ident : $arg:ident),* ) ),* $(,)?) => {$(
        #[cfg(any(feature = "enabled", debug_assertions))]
        #[doc(hidden)]
        #[allow(clippy::too_many_arguments)]
        pub fn $name<$($ty,)* F: Fn($(&$ty),*)>($($arg: &$crate::GhostState<$ty>,)* f: F) {
            f($($arg.inner()),*)
        }

        #[cfg(not(any(feature = "enabled", debug_assertions)))]
        #[doc(hidden)]
        #[allow(unused_variables, clippy::too_many_arguments)]
        pub fn $name<$($ty,)* F: Fn($(&$ty),*)>($($arg: &$crate::GhostState<$ty>,)* f: F) {}
    )*};
}

define_observe_helpers! {
    __observe0(),
    __observe1(T0: m0),
    __observe2(T0: m0, T1: m1),
    __observe3(T0: m0, T1: m1, T2: m2),
    __observe4(T0: m0, T1: m1, T2: m2, T3: m3),
}

/// Run a read-only observation block, optionally borrowing one or more [`GhostState`]s. The
/// block is an `Fn` closure — the compiler rejects any attempt to mutate what it captures — and
/// may call the assertion macros in this crate. When `enabled` is off it is type-checked but
/// never executed. Up to 4 ghost states may be observed at once.
#[macro_export]
macro_rules! observe {
    ($closure:expr $(,)?) => {
        $crate::__observe0($closure)
    };
    ($m0:expr, $closure:expr $(,)?) => {
        $crate::__observe1(&$m0, $closure)
    };
    ($m0:expr, $m1:expr, $closure:expr $(,)?) => {
        $crate::__observe2(&$m0, &$m1, $closure)
    };
    ($m0:expr, $m1:expr, $m2:expr, $closure:expr $(,)?) => {
        $crate::__observe3(&$m0, &$m1, &$m2, $closure)
    };
    ($m0:expr, $m1:expr, $m2:expr, $m3:expr, $closure:expr $(,)?) => {
        $crate::__observe4(&$m0, &$m1, &$m2, &$m3, $closure)
    };
}

#[cfg(test)]
mod tests {
    use crate::GhostState;

    // Must compile and not panic in every configuration (the asserts below all hold).
    #[test]
    fn compiles_and_runs_in_all_configs() {
        crate::init();

        let mut seen = GhostState::new(|| 0i64);
        seen.mutate(|n| *n += 1);

        // Called unqualified: `#[macro_export]` macros aren't reachable by absolute path from
        // within the defining crate (consumers use `dst::assert_*!`).
        crate::observe!(seen, |n: &i64| {
            assert_always!(
                *n >= 0,
                "seen count is never negative",
                &::serde_json::json!({ "n": *n })
            );
        });

        crate::observe!(|| {
            assert_reachable!("observation ran");
            assert_sometimes!(true, "sometimes true");
            assert_always_or_unreachable!(1 + 1 == 2, "arithmetic holds");
        });
    }

    // A failing assert panics only where it lowers to `debug_assert!` (SDK off, debug_assertions
    // on), so these two tests are gated to that config.
    #[cfg(all(not(feature = "enabled"), debug_assertions))]
    #[test]
    #[should_panic(expected = "always-false correctness assert")]
    fn failing_assert_always_panics_in_debug() {
        crate::observe!(|| {
            assert_always!(false, "always-false correctness assert");
        });
    }

    #[cfg(all(not(feature = "enabled"), debug_assertions))]
    #[test]
    #[should_panic(expected = "always-false correctness assert: {\"key\":\"value\"}")]
    fn failing_assert_always_with_details_panics_in_debug() {
        crate::observe!(|| {
            assert_always!(
                false,
                "always-false correctness assert",
                &::serde_json::json!({ "key": "value" })
            );
        });
    }

    #[cfg(all(not(feature = "enabled"), debug_assertions))]
    #[test]
    #[should_panic(expected = "reached an impossible state: {\"key\":\"value\"}")]
    fn reached_unreachable_with_details_panics_in_debug() {
        crate::observe!(|| {
            assert_unreachable!(
                "reached an impossible state",
                &::serde_json::json!({ "key": "value" })
            );
        });
    }
}
